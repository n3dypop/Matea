#![forbid(unsafe_code)]
//! Typed TEA state, cooperative effects and deterministic subscriptions, without terminal I/O.
use matea_render::{GeometryError, Size, Surface, View};
use matea_style::Theme;
use std::{
    any::{TypeId, type_name},
    collections::{BTreeMap, HashMap},
    hash::BuildHasher,
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, SyncSender, TrySendError},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

pub mod input;
pub use input::{
    Input, Key, KeyCode, KeyKind, KeyState, MediaKey, ModifierKey, Modifiers, Mouse, MouseButton,
    MouseKind, ScrollDirection,
};
#[derive(Debug)]
pub enum Event<M> {
    Input(Input),
    Resize(Size),
    Message(M),
    Failure(EffectFailure),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectFailure {
    pub id: String,
    pub message: String,
    pub panicked: bool,
}

pub struct ViewContext<'a> {
    pub size: Size,
    pub theme: &'a Theme,
}
pub trait Model {
    type Message: Send + 'static;
    fn init(&mut self) -> Effect<Self::Message> {
        Effect::None
    }
    fn update(&mut self, event: Event<Self::Message>) -> Effect<Self::Message>;
    fn view(&self, context: ViewContext<'_>) -> View;
    fn subscriptions(&self) -> Vec<Subscription<Self::Message>> {
        Vec::new()
    }
}

/// Cloned tokens share cancellation. `wait` returns true if canceled.
#[derive(Clone, Default)]
pub struct Cancellation(Arc<(Mutex<bool>, Condvar)>);
impl Cancellation {
    pub fn cancel(&self) {
        let (m, c) = &*self.0;
        *m.lock().unwrap_or_else(|e| e.into_inner()) = true;
        c.notify_all();
    }
    pub fn is_cancelled(&self) -> bool {
        *self.0.0.lock().unwrap_or_else(|e| e.into_inner())
    }
    pub fn wait(&self, duration: Duration) -> bool {
        let (m, c) = &*self.0;
        let guard = m.lock().unwrap_or_else(|e| e.into_inner());
        let (guard, _) = c
            .wait_timeout_while(guard, duration, |c| !*c)
            .unwrap_or_else(|e| e.into_inner());
        *guard
    }
}

type Work<M> = Box<dyn FnOnce(Cancellation) -> Result<Option<M>, String> + Send>;
pub struct Task<M> {
    id: String,
    work: Work<M>,
}
impl<M> Task<M> {
    pub fn new(
        id: impl Into<String>,
        work: impl FnOnce(Cancellation) -> Result<Option<M>, String> + Send + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            work: Box::new(work),
        }
    }
}
pub enum Effect<M> {
    None,
    Quit,
    Task(Task<M>),
    Batch(Vec<Self>),
    Sequence { id: String, tasks: Vec<Task<M>> },
    Cancel(String),
}
impl<M> Effect<M> {
    pub fn none() -> Self {
        Self::None
    }
    pub fn quit() -> Self {
        Self::Quit
    }
    pub fn perform(
        id: impl Into<String>,
        work: impl FnOnce(Cancellation) -> Result<M, String> + Send + 'static,
    ) -> Self {
        Self::Task(Task::new(id, move |c| work(c).map(Some)))
    }
    pub fn cancel(id: impl Into<String>) -> Self {
        Self::Cancel(id.into())
    }
    pub fn batch(effects: impl IntoIterator<Item = Self>) -> Self {
        Self::Batch(effects.into_iter().collect())
    }
    /// One worker slot, ordered results; child task IDs are diagnostic only.
    pub fn sequence(id: impl Into<String>, tasks: Vec<Task<M>>) -> Self {
        Self::Sequence {
            id: id.into(),
            tasks,
        }
    }
}

/// In-process subscription identity. Named and type-scoped keys never collide.
/// A scoped key is not a stable serialization or cross-build ordering format.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SubscriptionId(SubscriptionIdentity);
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum SubscriptionIdentity {
    Named(String),
    Scoped {
        namespace: &'static str,
        type_id: TypeId,
        instance: u64,
        revision: u64,
        variant: u128,
    },
}
impl SubscriptionId {
    pub fn named(name: impl Into<String>) -> Self {
        Self(SubscriptionIdentity::Named(name.into()))
    }
    /// Use a private tag type for an independently owned component namespace.
    /// The full tuple is retained and compared, without formatting or hashing it
    /// down to a lossy short ID. Changing behavior requires changing the tuple.
    pub fn scoped<Tag: 'static>(instance: u64, revision: u64, variant: u128) -> Self {
        Self(SubscriptionIdentity::Scoped {
            namespace: type_name::<Tag>(),
            type_id: TypeId::of::<Tag>(),
            instance,
            revision,
            variant,
        })
    }
}
impl std::fmt::Display for SubscriptionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            SubscriptionIdentity::Named(name) => f.write_str(name),
            SubscriptionIdentity::Scoped {
                namespace,
                type_id,
                instance,
                revision,
                variant,
            } => write!(
                f,
                "{namespace}[{type_id:?}]:{instance}:{revision}:{variant}"
            ),
        }
    }
}
/// Stable ID identifies behavior. To restart/change a subscription, change its ID.
pub struct Subscription<M> {
    id: SubscriptionId,
    interval: Duration,
    message: Box<dyn Fn() -> M>,
}
impl<M> Subscription<M> {
    pub fn every(
        id: impl Into<String>,
        interval: Duration,
        message: impl Fn() -> M + 'static,
    ) -> Self {
        Self::every_with_id(SubscriptionId::named(id), interval, message)
    }
    pub fn every_with_id(
        id: SubscriptionId,
        interval: Duration,
        message: impl Fn() -> M + 'static,
    ) -> Self {
        Self {
            id,
            interval,
            message: Box::new(message),
        }
    }
}
struct Timer<M> {
    interval: Duration,
    message: Box<dyn Fn() -> M>,
    // None means no future deadline fits in the logical Duration domain.
    next: Option<Duration>,
}
struct Worker {
    id: String,
    token: Cancellation,
    handle: JoinHandle<()>,
}
enum Completion<M> {
    Message(M),
    Failure(EffectFailure),
    Done,
}
struct Envelope<M> {
    id: String,
    generation: u64,
    completion: Completion<M>,
}

#[derive(Debug)]
pub struct RuntimeError(pub String);
impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for RuntimeError {}

/// UI-thread state driver. At most 8 workers and 32 pending completion envelopes.
pub struct Runtime<A: Model> {
    model: A,
    size: Size,
    theme: Theme,
    quitting: bool,
    tx: SyncSender<Envelope<A::Message>>,
    rx: Receiver<Envelope<A::Message>>,
    workers: Vec<Worker>,
    current: HashMap<String, u64>,
    generation: u64,
    timers: BTreeMap<SubscriptionId, Timer<A::Message>>,
    now: Duration,
    worker_panic: Arc<AtomicBool>,
}
impl<A: Model> Runtime<A> {
    pub fn new(mut model: A, size: Size, theme: Theme) -> Result<Self, RuntimeError> {
        let effect = model.init();
        let (tx, rx) = mpsc::sync_channel(32);
        let mut r = Self {
            model,
            size,
            theme,
            quitting: false,
            tx,
            rx,
            workers: Vec::new(),
            current: HashMap::new(),
            generation: 0,
            timers: BTreeMap::new(),
            now: Duration::ZERO,
            worker_panic: Arc::new(AtomicBool::new(false)),
        };
        r.schedule(effect)?;
        r.dispatch(Event::Resize(size))?;
        Ok(r)
    }
    pub fn model(&self) -> &A {
        &self.model
    }
    pub fn is_quitting(&self) -> bool {
        self.quitting
    }
    pub fn size(&self) -> Size {
        self.size
    }
    /// Time from the last logical advance to the nearest active subscription.
    /// Drivers should subtract wall time spent since that advance before waiting.
    /// None means no subscription has a representable future deadline.
    /// This does not report pending worker completions or read a platform clock.
    pub fn next_subscription_delay(&self) -> Option<Duration> {
        self.timers
            .values()
            .filter_map(|timer| timer.next)
            .min()
            .map(|deadline| deadline.saturating_sub(self.now))
    }
    /// Observe a caught worker panic once, including canceled/stale effects.
    /// The normal panic hook may have written outside the application's view.
    /// Drivers can invalidate output caches without installing a global hook.
    pub fn take_worker_panic_notice(&self) -> bool {
        self.worker_panic.swap(false, Ordering::Acquire)
    }
    pub fn frame(&self) -> Result<Surface, GeometryError> {
        self.model
            .view(ViewContext {
                size: self.size,
                theme: &self.theme,
            })
            .render(self.size)
    }
    pub fn dispatch(&mut self, event: Event<A::Message>) -> Result<(), RuntimeError> {
        if self.quitting {
            return Ok(());
        }
        if let Event::Resize(size) = &event {
            self.size = *size;
        }
        let effect = self.model.update(event);
        self.schedule(effect)?;
        if self.quitting {
            return Ok(());
        }
        self.reconcile()
    }
    fn reconcile(&mut self) -> Result<(), RuntimeError> {
        let mut subscriptions = self.model.subscriptions();
        if subscriptions.is_empty() {
            self.timers.clear();
            return Ok(());
        }
        if subscriptions.len() > 64 {
            return Err(RuntimeError("subscription limit is 64".into()));
        }
        subscriptions.sort_unstable_by(|a, b| a.id.cmp(&b.id));
        for s in &subscriptions {
            if s.interval.is_zero() {
                return Err(RuntimeError(format!(
                    "subscription {} requires a nonzero interval",
                    s.id
                )));
            }
        }
        for pair in subscriptions.windows(2) {
            if pair[0].id == pair[1].id {
                return Err(RuntimeError(format!(
                    "duplicate subscription ID: {}",
                    pair[0].id
                )));
            }
        }
        // Validate everything before mutating the table. Retain unchanged nodes,
        // deadlines and callbacks; a rejected configuration cannot postpone them.
        self.timers
            .retain(|id, _| subscriptions.binary_search_by(|s| s.id.cmp(id)).is_ok());
        let now = self.now;
        for s in subscriptions {
            let Subscription {
                id,
                interval,
                message,
            } = s;
            self.timers.entry(id).or_insert_with(|| Timer {
                next: now.checked_add(interval),
                interval,
                message,
            });
        }
        Ok(())
    }
    /// Advance logical time. Delayed timers coalesce to one message per call.
    /// A deadline beyond Duration::MAX cannot fire; the exact maximum fires once.
    pub fn advance(&mut self, elapsed: Duration) -> Result<bool, RuntimeError> {
        self.now = self.now.saturating_add(elapsed);
        let due: Vec<_> = self
            .timers
            .iter()
            .filter(|(_, t)| t.next.is_some_and(|next| next <= self.now))
            .map(|(id, _)| id.clone())
            .collect();
        let mut changed = false;
        for id in due {
            if let Some(t) = self.timers.get_mut(&id) {
                if t.next.is_none_or(|next| next > self.now) {
                    continue;
                }
                t.next = self.now.checked_add(t.interval);
                let message = (t.message)();
                self.dispatch(Event::Message(message))?;
                changed = true;
            }
        }
        Ok(changed)
    }
    pub fn poll_effects(&mut self) -> Result<bool, RuntimeError> {
        let mut changed = false;
        for _ in 0..32 {
            let Ok(e) = self.rx.try_recv() else { break };
            if self.current.get(&e.id) != Some(&e.generation) {
                continue;
            }
            match e.completion {
                Completion::Message(m) => {
                    self.dispatch(Event::Message(m))?;
                    changed = true;
                }
                Completion::Failure(f) => {
                    self.dispatch(Event::Failure(f))?;
                    changed = true;
                }
                Completion::Done => {
                    self.current.remove(&e.id);
                }
            }
        }
        self.reap();
        Ok(changed)
    }
    fn reap(&mut self) {
        let mut i = 0;
        while i < self.workers.len() {
            if self.workers[i].handle.is_finished() {
                let w = self.workers.swap_remove(i);
                let _ = w.handle.join();
            } else {
                i += 1;
            }
        }
    }
    fn cancel(&mut self, id: &str) {
        self.current.remove(id);
        for w in &self.workers {
            if w.id == id {
                w.token.cancel();
            }
        }
    }
    fn schedule(&mut self, effect: Effect<A::Message>) -> Result<(), RuntimeError> {
        if self.quitting {
            return Ok(());
        }
        match effect {
            Effect::None => {}
            Effect::Quit => {
                self.quitting = true;
                self.shutdown();
            }
            Effect::Cancel(id) => self.cancel(&id),
            Effect::Batch(effects) => {
                for e in effects {
                    self.schedule(e)?;
                }
            }
            Effect::Task(task) => self.launch(task.id.clone(), vec![task])?,
            Effect::Sequence { id, tasks } => {
                if !tasks.is_empty() {
                    self.launch(id, tasks)?;
                }
            }
        }
        Ok(())
    }
    fn launch(&mut self, id: String, tasks: Vec<Task<A::Message>>) -> Result<(), RuntimeError> {
        self.reap();
        if self.workers.len() >= 8 {
            return Err(RuntimeError(
                "effect worker limit (8) reached; cancel cooperatively or batch fewer tasks".into(),
            ));
        }
        self.cancel(&id);
        self.generation = self
            .generation
            .checked_add(1)
            .ok_or_else(|| RuntimeError("effect generation exhausted".into()))?;
        let generation = self.generation;
        let token = Cancellation::default();
        let c = token.clone();
        let tx = self.tx.clone();
        let worker_id = id.clone();
        let worker_panic = self.worker_panic.clone();
        let handle = thread::Builder::new()
            .name(format!("matea:{id}"))
            .spawn(move || {
                let send = |completion| {
                    send_completion(
                        &tx,
                        &c,
                        Envelope {
                            id: worker_id.clone(),
                            generation,
                            completion,
                        },
                    )
                };
                for task in tasks {
                    if c.is_cancelled() {
                        return;
                    }
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        (task.work)(c.clone())
                    }));
                    let completion = match result {
                        Ok(Ok(Some(m))) => Completion::Message(m),
                        Ok(Ok(None)) => continue,
                        Ok(Err(message)) => Completion::Failure(EffectFailure {
                            id: task.id,
                            message,
                            panicked: false,
                        }),
                        Err(_) => {
                            worker_panic.store(true, Ordering::Release);
                            Completion::Failure(EffectFailure {
                                id: task.id,
                                message: "effect panicked".into(),
                                panicked: true,
                            })
                        }
                    };
                    let failed = matches!(completion, Completion::Failure(_));
                    if !send(completion) || failed {
                        break;
                    }
                }
                send(Completion::Done);
            })
            .map_err(|e| RuntimeError(format!("could not spawn effect {id}: {e}")))?;
        self.current.insert(id.clone(), generation);
        self.workers.push(Worker { id, token, handle });
        Ok(())
    }
    pub fn shutdown(&mut self) {
        self.quitting = true;
        for w in &self.workers {
            w.token.cancel();
        }
        self.current.clear();
        self.timers.clear();
        self.reap();
    }
}
impl<A: Model> Drop for Runtime<A> {
    fn drop(&mut self) {
        self.shutdown();
    }
}
fn send_completion<M>(tx: &SyncSender<Envelope<M>>, c: &Cancellation, mut e: Envelope<M>) -> bool {
    loop {
        if c.is_cancelled() {
            return false;
        }
        match tx.try_send(e) {
            Ok(()) => return true,
            Err(TrySendError::Disconnected(_)) => return false,
            Err(TrySendError::Full(v)) => {
                e = v;
                if c.wait(Duration::from_millis(2)) {
                    return false;
                }
            }
        }
    }
}

/// Public component contract usable from a separately compiled crate.
pub trait Component {
    type Message;
    fn event(&mut self, input: &Input, focused: bool) -> Response<Self::Message>;
    fn view(&self, theme: &Theme, focused: bool) -> View;
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Response<M> {
    pub consumed: bool,
    pub message: Option<M>,
}
impl<M> Response<M> {
    pub fn ignored() -> Self {
        Self {
            consumed: false,
            message: None,
        }
    }
    pub fn consumed(message: Option<M>) -> Self {
        Self {
            consumed: true,
            message,
        }
    }
}
#[derive(Clone, Debug, Default)]
pub struct FocusRing {
    ids: Vec<String>,
    index: usize,
}
impl FocusRing {
    #[inline]
    pub fn new(ids: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            ids: focus_ids(
                ids.into_iter().map(Into::into),
                std::collections::hash_map::RandomState::new,
            ),
            index: 0,
        }
    }
    pub fn current(&self) -> Option<&str> {
        self.ids.get(self.index).map(String::as_str)
    }
    pub fn handle(&mut self, input: &Input) -> bool {
        if self.ids.is_empty() {
            return false;
        }
        match input.key_down().and_then(Key::focus_direction) {
            Some(1) => self.index = (self.index + 1) % self.ids.len(),
            Some(-1) => self.index = (self.index + self.ids.len() - 1) % self.ids.len(),
            _ => return false,
        }
        true
    }
}

/// Keep small rings and duplicate-heavy streams on the allocation-light scan.
/// For larger unique sets, hashes select a linked chain of indices. Strings stay
/// owned once in `out`; full equality verifies every hash match. No pointers or
/// self-referential borrows survive Vec growth, and collisions never lose IDs.
#[inline]
fn focus_ids<H: BuildHasher>(
    mut input: impl Iterator<Item = String>,
    make_hasher: impl FnOnce() -> H,
) -> Vec<String> {
    let mut out = Vec::new();
    for id in input.by_ref() {
        if !out.contains(&id) {
            out.push(id);
            if out.len() > 128 {
                return indexed_focus_ids(out, input, make_hasher());
            }
        }
    }
    out
}

// Keep the hash table's stack/drop machinery out of the common small ring path.
#[inline(never)]
fn indexed_focus_ids<H: BuildHasher>(
    mut out: Vec<String>,
    input: impl Iterator<Item = String>,
    hasher: H,
) -> Vec<String> {
    let mut heads = HashMap::with_capacity(out.capacity());
    let mut links = Vec::with_capacity(out.capacity());
    for (index, id) in out.iter().enumerate() {
        let key = hasher.hash_one(id.as_str());
        links.push(heads.insert(key, index).unwrap_or(usize::MAX));
    }
    for id in input {
        let key = hasher.hash_one(id.as_str());
        let head = heads.get(&key).copied().unwrap_or(usize::MAX);
        let mut index = head;
        while index != usize::MAX && out[index] != id {
            index = links[index];
        }
        if index == usize::MAX {
            heads.insert(key, out.len());
            links.push(head);
            out.push(id);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    struct DeadlineModel {
        fast: Duration,
        slow: Option<Duration>,
        enabled: bool,
    }
    impl Model for DeadlineModel {
        type Message = bool;
        fn update(&mut self, event: Event<bool>) -> Effect<bool> {
            if let Event::Message(enabled) = event {
                self.enabled = enabled;
            }
            Effect::none()
        }
        fn view(&self, _: ViewContext<'_>) -> View {
            View::text("deadline")
        }
        fn subscriptions(&self) -> Vec<Subscription<bool>> {
            if !self.enabled {
                return Vec::new();
            }
            let mut timers = vec![Subscription::every("fast", self.fast, || true)];
            if let Some(slow) = self.slow {
                timers.push(Subscription::every("slow", slow, || true));
            }
            timers
        }
    }
    #[test]
    fn subscription_deadline_tracks_nearest_logical_time_removal_and_coalescing() {
        let model = DeadlineModel {
            fast: Duration::from_millis(3),
            slow: Some(Duration::from_millis(10)),
            enabled: true,
        };
        let mut runtime = Runtime::new(model, Size::new(4, 1), Theme::plain()).unwrap();
        assert_eq!(
            runtime.next_subscription_delay(),
            Some(Duration::from_millis(3))
        );
        runtime.advance(Duration::from_millis(2)).unwrap();
        assert_eq!(
            runtime.next_subscription_delay(),
            Some(Duration::from_millis(1))
        );
        runtime.advance(Duration::from_millis(1)).unwrap();
        assert_eq!(
            runtime.next_subscription_delay(),
            Some(Duration::from_millis(3))
        );
        runtime.advance(Duration::from_millis(5)).unwrap();
        assert_eq!(
            runtime.next_subscription_delay(),
            Some(Duration::from_millis(2))
        );
        runtime.dispatch(Event::Message(false)).unwrap();
        assert_eq!(runtime.next_subscription_delay(), None);
        runtime.advance(Duration::from_secs(1)).unwrap();
        assert_eq!(runtime.next_subscription_delay(), None);
        runtime.dispatch(Event::Message(true)).unwrap();
        assert_eq!(
            runtime.next_subscription_delay(),
            Some(Duration::from_millis(3))
        );
        runtime.shutdown();
        assert_eq!(runtime.next_subscription_delay(), None);
    }
    #[test]
    fn subscription_deadline_distinguishes_maximum_from_unrepresentable_future() {
        let model = DeadlineModel {
            fast: Duration::MAX,
            slow: None,
            enabled: true,
        };
        let mut runtime = Runtime::new(model, Size::new(4, 1), Theme::plain()).unwrap();
        assert_eq!(runtime.next_subscription_delay(), Some(Duration::MAX));
        runtime
            .advance(Duration::MAX - Duration::from_nanos(1))
            .unwrap();
        assert_eq!(
            runtime.next_subscription_delay(),
            Some(Duration::from_nanos(1))
        );
        runtime.advance(Duration::from_nanos(1)).unwrap();
        assert_eq!(runtime.next_subscription_delay(), None);
        runtime.advance(Duration::MAX).unwrap();
        assert_eq!(runtime.next_subscription_delay(), None);
    }
    #[test]
    fn large_focus_rings_preserve_first_order_even_with_total_hash_collision() {
        #[derive(Default)]
        struct ZeroHasher;
        impl std::hash::Hasher for ZeroHasher {
            fn write(&mut self, _: &[u8]) {}
            fn finish(&self) -> u64 {
                0
            }
        }
        let (count, modulus) = if cfg!(miri) { (160, 137) } else { (900, 257) };
        let input: Vec<_> = (0..count)
            .map(|i| format!("field-{}", i * 73 % modulus))
            .collect();
        let mut expected = Vec::new();
        for name in &input {
            if !expected.contains(name) {
                expected.push(name.clone());
            }
        }
        let actual = focus_ids(
            input.clone().into_iter(),
            std::hash::BuildHasherDefault::<ZeroHasher>::default,
        );
        assert_eq!(actual, expected);
        let mut ring = FocusRing::new(input);
        assert_eq!(ring.ids, expected);
        for name in expected.iter().cycle().take(expected.len() * 2) {
            assert_eq!(ring.current(), Some(name.as_str()));
            assert!(ring.handle(&Input::Key(Key::new(KeyCode::Tab))));
        }
        assert_eq!(ring.current(), Some(expected[0].as_str()));
    }
    #[test]
    fn small_or_duplicate_heavy_focus_rings_never_construct_a_hash_table() {
        for count in [0, 3, 128, 1000] {
            let mut i = 0usize;
            let actual = focus_ids(
                (0..count).map(|_| {
                    i += 1;
                    "same".to_owned()
                }),
                || -> std::collections::hash_map::RandomState { panic!("unneeded hasher") },
            );
            assert_eq!(i, count);
            assert_eq!(actual.len(), usize::from(count != 0));
        }
    }
    struct App {
        values: Vec<i32>,
        effect: Option<Effect<i32>>,
        ticks: bool,
        failures: Vec<EffectFailure>,
    }
    impl Model for App {
        type Message = i32;
        fn init(&mut self) -> Effect<i32> {
            self.effect.take().unwrap_or(Effect::None)
        }
        fn update(&mut self, e: Event<i32>) -> Effect<i32> {
            match e {
                Event::Message(n) => self.values.push(n),
                Event::Failure(f) => self.failures.push(f),
                _ => {}
            }
            Effect::None
        }
        fn view(&self, _: ViewContext<'_>) -> View {
            View::text(format!("{:?}", self.values))
        }
        fn subscriptions(&self) -> Vec<Subscription<i32>> {
            if self.ticks {
                vec![Subscription::every("clock", Duration::from_secs(1), || 7)]
            } else {
                vec![]
            }
        }
    }
    fn app(effect: Effect<i32>) -> Runtime<App> {
        Runtime::new(
            App {
                values: vec![],
                effect: Some(effect),
                ticks: false,
                failures: vec![],
            },
            Size::new(20, 1),
            Theme::default(),
        )
        .unwrap()
    }
    fn drain(r: &mut Runtime<App>) {
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while !r.workers.is_empty() || !r.current.is_empty() {
            r.poll_effects().unwrap();
            assert!(std::time::Instant::now() < deadline);
            thread::yield_now();
        }
    }
    #[test]
    fn sequence_and_failures() {
        let mut r = app(Effect::sequence(
            "seq",
            vec![
                Task::new("a", |_| Ok(Some(1))),
                Task::new("b", |_| Ok(Some(2))),
            ],
        ));
        drain(&mut r);
        assert_eq!(r.model.values, vec![1, 2]);
        let mut r = app(Effect::perform("fail", |_| {
            Err("network unavailable".into())
        }));
        drain(&mut r);
        assert_eq!(r.model.failures[0].message, "network unavailable");
        let mut r = app(Effect::perform("panic", |_| panic!("fixture")));
        drain(&mut r);
        assert!(r.model.failures[0].panicked);
        assert!(r.take_worker_panic_notice());
        assert!(!r.take_worker_panic_notice());
        let mut r = app(Effect::perform("stale-panic", |_| panic!("stale fixture")));
        while !r.workers[0].handle.is_finished() {
            thread::yield_now();
        }
        r.schedule(Effect::cancel("stale-panic")).unwrap();
        drain(&mut r);
        assert!(r.model.failures.is_empty());
        assert!(r.take_worker_panic_notice());
    }
    #[test]
    fn cancelled_queued_results_are_discarded() {
        let mut r = app(Effect::perform("request", |_| Ok(99)));
        while !r.workers[0].handle.is_finished() {
            thread::yield_now();
        }
        r.schedule(Effect::cancel("request")).unwrap();
        drain(&mut r);
        assert!(r.model.values.is_empty());
    }
    #[test]
    fn replacing_an_id_discards_the_old_generation() {
        let mut r = app(Effect::perform("request", |_| Ok(99)));
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while !r.workers[0].handle.is_finished() {
            assert!(std::time::Instant::now() < deadline);
            thread::yield_now();
        }
        r.schedule(Effect::perform("request", |_| Ok(100))).unwrap();
        drain(&mut r);
        assert_eq!(r.model.values, vec![100]);
    }
    #[test]
    fn full_completion_queue_does_not_prevent_shutdown() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let progress = Arc::new(AtomicUsize::new(0));
        let tasks = (0..64)
            .map(|n| {
                let p = progress.clone();
                Task::new(n.to_string(), move |_| {
                    p.store(n + 1, Ordering::Release);
                    Ok(Some(n as i32))
                })
            })
            .collect();
        let mut r = app(Effect::sequence("producer", tasks));
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while progress.load(Ordering::Acquire) < 33 {
            assert!(std::time::Instant::now() < deadline);
            thread::yield_now();
        }
        r.shutdown();
        while r.workers.iter().any(|w| !w.handle.is_finished()) {
            assert!(std::time::Instant::now() < deadline);
            thread::yield_now();
        }
        assert_eq!(progress.load(Ordering::Acquire), 33);
        assert!(r.model.values.is_empty());
    }
    #[test]
    fn model_state_can_remain_non_send() {
        use std::{cell::Cell, rc::Rc};
        struct Local(Rc<Cell<usize>>);
        impl Model for Local {
            type Message = ();
            fn update(&mut self, e: Event<()>) -> Effect<()> {
                if matches!(e, Event::Message(())) {
                    self.0.set(self.0.get() + 1);
                }
                Effect::None
            }
            fn view(&self, _: ViewContext<'_>) -> View {
                View::text(self.0.get().to_string())
            }
        }
        let state = Rc::new(Cell::new(0));
        let mut r = Runtime::new(Local(state.clone()), Size::new(4, 1), Theme::plain()).unwrap();
        r.dispatch(Event::Message(())).unwrap();
        assert_eq!(state.get(), 1);
    }
    #[test]
    fn cancellation_interrupts_wait_and_worker_limit_is_enforced() {
        let token = Cancellation::default();
        let c = token.clone();
        let h = thread::spawn(move || c.wait(Duration::from_secs(60)));
        token.cancel();
        assert!(h.join().unwrap());
        let mut r = app(Effect::None);
        for n in 0..8 {
            r.schedule(Effect::perform(n.to_string(), |c| {
                c.wait(Duration::from_secs(60));
                Ok(0)
            }))
            .unwrap();
        }
        assert!(r.schedule(Effect::perform("overflow", |_| Ok(0))).is_err());
        r.shutdown();
    }
    #[test]
    fn deterministic_subscriptions_resize_and_focus() {
        let mut r = app(Effect::None);
        r.model.ticks = true;
        r.reconcile().unwrap();
        assert!(!r.advance(Duration::from_millis(999)).unwrap());
        r.advance(Duration::from_millis(1)).unwrap();
        r.advance(Duration::from_secs(20)).unwrap();
        assert_eq!(r.model.values, vec![7, 7]);
        r.model.ticks = false;
        r.reconcile().unwrap();
        r.advance(Duration::from_secs(1)).unwrap();
        assert_eq!(r.model.values.len(), 2);
        r.dispatch(Event::Resize(Size::new(3, 2))).unwrap();
        assert_eq!(r.frame().unwrap().size(), Size::new(3, 2));
        let mut f = FocusRing::new(["a", "b", "a"]);
        assert_eq!(f.current(), Some("a"));
        f.handle(&Input::Key(Key::new(KeyCode::BackTab)));
        assert_eq!(f.current(), Some("b"));
    }
    #[test]
    fn saturated_clock_never_refires_or_rounds_down_unrepresentable_deadlines() {
        let mut r = app(Effect::None);
        r.advance(Duration::MAX - Duration::from_secs(1)).unwrap();
        r.model.ticks = true;
        r.reconcile().unwrap();
        assert!(r.advance(Duration::from_secs(1)).unwrap());
        assert_eq!(r.model.values, [7]);
        assert!(!r.advance(Duration::ZERO).unwrap());
        assert!(!r.advance(Duration::from_secs(1)).unwrap());
        let mut r = app(Effect::None);
        r.advance(Duration::MAX - Duration::from_nanos(1)).unwrap();
        r.model.ticks = true;
        r.reconcile().unwrap();
        assert!(!r.advance(Duration::from_nanos(1)).unwrap());
        assert!(r.model.values.is_empty());
    }
}

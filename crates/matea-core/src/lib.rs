#![forbid(unsafe_code)]
//! Typed TEA state, cooperative effects and deterministic subscriptions, without terminal I/O.
use matea_render::{GeometryError, Size, Surface, View};
use matea_style::Theme;
use std::{
    collections::{BTreeMap, HashMap},
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, SyncSender, TrySendError},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyCode {
    Char(char),
    Enter,
    Esc,
    Backspace,
    Delete,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    Tab,
    BackTab,
    PageUp,
    PageDown,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Key {
    pub code: KeyCode,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
}
impl Key {
    pub const fn new(code: KeyCode) -> Self {
        Self {
            code,
            ctrl: false,
            alt: false,
            shift: false,
        }
    }
    pub fn is_char(self, c: char) -> bool {
        self.code == KeyCode::Char(c) && !self.ctrl && !self.alt
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Input {
    Key(Key),
    Paste(String),
    Focus(bool),
}
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

/// Stable ID identifies behavior. To restart/change a subscription, change its ID.
pub struct Subscription<M> {
    id: String,
    interval: Duration,
    message: Box<dyn Fn() -> M>,
}
impl<M> Subscription<M> {
    pub fn every(
        id: impl Into<String>,
        interval: Duration,
        message: impl Fn() -> M + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            interval,
            message: Box::new(message),
        }
    }
}
struct Timer<M> {
    subscription: Subscription<M>,
    next: Duration,
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
    timers: BTreeMap<String, Timer<A::Message>>,
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
        let subscriptions = self.model.subscriptions();
        if subscriptions.len() > 64 {
            return Err(RuntimeError("subscription limit is 64".into()));
        }
        let mut next = BTreeMap::new();
        for s in subscriptions {
            if s.interval.is_zero() {
                return Err(RuntimeError(format!(
                    "subscription {} requires a nonzero interval",
                    s.id
                )));
            }
            if next.contains_key(&s.id) {
                return Err(RuntimeError(format!("duplicate subscription ID: {}", s.id)));
            }
            let id = s.id.clone();
            let t = self.timers.remove(&id).unwrap_or_else(|| Timer {
                next: self.now.saturating_add(s.interval),
                subscription: s,
            });
            next.insert(id, t);
        }
        self.timers = next;
        Ok(())
    }
    /// Advance logical time. Delayed timers coalesce to one message per call.
    pub fn advance(&mut self, elapsed: Duration) -> Result<bool, RuntimeError> {
        self.now = self.now.saturating_add(elapsed);
        let due: Vec<_> = self
            .timers
            .iter()
            .filter(|(_, t)| t.next <= self.now)
            .map(|(id, _)| id.clone())
            .collect();
        let mut changed = false;
        for id in due {
            if let Some(t) = self.timers.get_mut(&id) {
                if t.next > self.now {
                    continue;
                }
                t.next = self.now.saturating_add(t.subscription.interval);
                let message = (t.subscription.message)();
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
    pub fn new(ids: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let mut out = Vec::new();
        for id in ids {
            let id = id.into();
            if !out.contains(&id) {
                out.push(id);
            }
        }
        Self { ids: out, index: 0 }
    }
    pub fn current(&self) -> Option<&str> {
        self.ids.get(self.index).map(String::as_str)
    }
    pub fn handle(&mut self, input: &Input) -> bool {
        if self.ids.is_empty() {
            return false;
        }
        if let Input::Key(k) = input {
            match k.code {
                KeyCode::Tab if !k.shift => self.index = (self.index + 1) % self.ids.len(),
                KeyCode::BackTab | KeyCode::Tab => {
                    self.index = (self.index + self.ids.len() - 1) % self.ids.len()
                }
                _ => return false,
            }
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
}

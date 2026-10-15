//! Terminal lifecycle fixture used by scripts/pty_smoke.py.
use matea::prelude::*;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
struct App {
    cancelled: Arc<AtomicBool>,
    panic_view: bool,
}
impl Model for App {
    type Message = ();
    fn init(&mut self) -> Effect<()> {
        let flag = self.cancelled.clone();
        Effect::perform("long-work", move |token| {
            token.wait(Duration::from_secs(60));
            flag.store(token.is_cancelled(), Ordering::Release);
            Ok(())
        })
    }
    fn update(&mut self, event: Event<()>) -> Effect<()> {
        if let Event::Input(Input::Key(k)) = event {
            if k.is_char('p') {
                panic!("lifecycle fixture update panic");
            }
            if k.is_char('q') {
                return Effect::quit();
            }
        }
        Effect::none()
    }
    fn view(&self, c: ViewContext<'_>) -> View {
        assert!(!self.panic_view, "lifecycle fixture view panic");
        View::text(format!(
            "READY {}x{} | q exit | p panic",
            c.size.width, c.size.height
        ))
    }
}
fn main() -> Result<(), matea::terminal::Error> {
    let cancelled = Arc::new(AtomicBool::new(false));
    matea::run(App {
        cancelled: cancelled.clone(),
        panic_view: std::env::args().any(|s| s == "--panic-view"),
    })?;
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while !cancelled.load(Ordering::Acquire) && std::time::Instant::now() < deadline {
        std::thread::yield_now();
    }
    assert!(
        cancelled.load(Ordering::Acquire),
        "worker did not observe cancellation"
    );
    println!("CANCELLED");
    if std::env::args().any(|s| s == "--post-signal") {
        println!("POST_RUN_READY");
        loop {
            std::thread::park();
        }
    }
    Ok(())
}

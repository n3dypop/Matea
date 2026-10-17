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
    panic_format: bool,
    recovered: bool,
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
        if let Event::Failure(f) = &event {
            self.recovered |= f.panicked;
        }
        if let Event::Input(Input::Key(k)) = event {
            if k.is_char('w') {
                return Effect::perform("panicking-work", |_| panic!("worker panic hook fixture"));
            }
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
        if self.panic_format {
            struct Bad;
            impl std::fmt::Display for Bad {
                fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    Err(std::fmt::Error)
                }
            }
            return View::text(format_args!("{Bad}"));
        }
        View::text(format!(
            "READY {}x{} | q exit | p panic{}",
            c.size.width,
            c.size.height,
            if self.recovered { " | RECOVERED" } else { "" }
        ))
    }
}
fn main() -> Result<(), matea::terminal::Error> {
    let cancelled = Arc::new(AtomicBool::new(false));
    matea::run(App {
        cancelled: cancelled.clone(),
        panic_view: std::env::args().any(|s| s == "--panic-view"),
        panic_format: std::env::args().any(|s| s == "--panic-format"),
        recovered: false,
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

#![forbid(unsafe_code)]
//! Matea: typed state, composable terminal views and coherent components.
//!
//! ```no_run
//! use matea::prelude::*;
//! struct Counter(i32);
//! impl Model for Counter {
//!     type Message = ();
//!     fn update(&mut self, event: Event<()>) -> Effect<()> {
//!         if let Event::Input(Input::Key(key)) = event {
//!             if key.is_char('+') { self.0 += 1; }
//!             if key.is_char('q') { return Effect::quit(); }
//!         }
//!         Effect::none()
//!     }
//!     fn view(&self, context: ViewContext<'_>) -> View {
//!         View::styled(format!("Count: {}", self.0), context.theme.accent)
//!     }
//! }
//! fn main() -> Result<(), matea::terminal::Error> { matea::run(Counter(0)) }
//! ```
pub use matea_core as core;
pub use matea_motion as motion;
pub use matea_render as render;
pub use matea_style as style;
pub use matea_terminal as terminal;
pub use matea_terminal::{run, run_with_options};
pub use matea_widgets as widgets;
pub mod prelude {
    pub use matea_core::{
        Cancellation, Component, Effect, Event, FocusRing, Input, Key, KeyCode, Model, Response,
        Runtime, Subscription, Task, ViewContext,
    };
    pub use matea_render::{Rect, Size, Surface, View};
    pub use matea_style::{Border, Color, ColorLevel, Style, Theme};
    pub use matea_widgets::{
        Action, Field, Form, List, Spinner, Table, TextInput, Viewport, required,
    };
}

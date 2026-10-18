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
//!         View::styled(format_args!("Count: {}", self.0), context.theme.accent)
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
        Cancellation, Component, Effect, Event, FocusRing, Input, Key, KeyCode, KeyKind, KeyState,
        MediaKey, Model, ModifierKey, Modifiers, Mouse, MouseButton, MouseKind, Response, Runtime,
        ScrollDirection, Subscription, SubscriptionId, Task, ViewContext,
    };
    pub use matea_render::{Cursor, Rect, Size, Surface, Text, View};
    pub use matea_style::{
        BlinkStyle, Border, Color, ColorLevel, Gradient, GradientColors, GradientError,
        GradientGrid, GradientGridColors, Style, TextAttribute, TextAttributes, TextFrame, Theme,
        UnderlineStyle,
    };
    pub use matea_widgets::key::{Binding, NavigationKeys};
    pub use matea_widgets::{
        Action, ClockId, ClockSchedule, ClockText, ClockTick, ClockUpdate, Countdown, CursorMode,
        EchoMode, EmptySpinnerFrames, Field, Form, Help, HelpItem, HelpMap, HelpStyles,
        InputStyleState, InputStyles, List, PROGRESS_EMPTY_BLOCK, PROGRESS_FULL_BLOCK,
        PROGRESS_HALF_BLOCK, PageDisplay, PageKeys, PageNumbers, Pagination, Paginator, Progress,
        ProgressCell, ProgressCells, ProgressError, ProgressFill, ProgressFrame, ProgressStyles,
        Spinner, SpinnerAnimation, SpinnerPreset, Stopwatch, Table, TextEditKeys, TextInput,
        TickInterval, Viewport, VirtualCursor, required,
    };
}

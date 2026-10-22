//! Interactive pagination composed from bounded index state and typed bindings.
use crate::{Action, Pagination, key::Binding};
use matea_core::{Component, Input, Key, KeyCode, Response};
use matea_render::View;
use matea_style::Theme;
use std::{fmt, num::NonZeroUsize, rc::Rc};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PageDisplay {
    #[default]
    Arabic,
    Dots,
}
/// One-based display numbers, with 0/0 for an empty extent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PageNumbers {
    pub current: usize,
    pub total: usize,
}
#[derive(Clone, Debug)]
pub struct PageKeys {
    pub previous: Binding,
    pub next: Binding,
}
impl Default for PageKeys {
    fn default() -> Self {
        Self {
            previous: Binding::new(
                [KeyCode::PageUp, KeyCode::Left, KeyCode::Char('h')].map(Key::new),
                "←/h",
                "previous page",
            ),
            next: Binding::new(
                [KeyCode::PageDown, KeyCode::Right, KeyCode::Char('l')].map(Key::new),
                "→/l",
                "next page",
            ),
        }
    }
}
type Formatter = Rc<dyn Fn(PageNumbers) -> String>;
/// UI-thread component. Index state cannot be invalidated by direct page writes.
/// Custom formatters can capture non-Send UI state, like other components.
#[derive(Clone)]
pub struct Paginator {
    pub pages: Pagination,
    pub display: PageDisplay,
    pub keys: PageKeys,
    pub active_dot: String,
    pub inactive_dot: String,
    /// Maximum dot nodes to construct, independently of a virtual page count.
    pub dot_budget: usize,
    formatter: Option<Formatter>,
}
impl fmt::Debug for Paginator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Paginator")
            .field("pages", &self.pages)
            .field("display", &self.display)
            .field("keys", &self.keys)
            .field("active_dot", &self.active_dot)
            .field("inactive_dot", &self.inactive_dot)
            .field("dot_budget", &self.dot_budget)
            .field("has_formatter", &self.formatter.is_some())
            .finish()
    }
}
impl Default for Paginator {
    fn default() -> Self {
        Self::new(Pagination::from_pages(1, NonZeroUsize::new(1).unwrap()))
    }
}
impl Paginator {
    pub fn new(pages: Pagination) -> Self {
        Self {
            pages,
            display: PageDisplay::Arabic,
            keys: PageKeys::default(),
            active_dot: "•".into(),
            inactive_dot: "○".into(),
            dot_budget: 80,
            formatter: None,
        }
    }
    pub fn numbers(&self) -> PageNumbers {
        let total = self.pages.total_pages();
        PageNumbers {
            current: if total == 0 { 0 } else { self.pages.page() + 1 },
            total,
        }
    }
    /// Use Rust formatting/composition rather than interpreting a printf string.
    /// This callback runs during view construction on the UI thread.
    pub fn with_formatter(mut self, formatter: impl Fn(PageNumbers) -> String + 'static) -> Self {
        self.formatter = Some(Rc::new(formatter));
        self
    }
    pub fn reset_formatter(&mut self) {
        self.formatter = None;
    }
    /// Standalone label, without terminal escape sequences or runtime state.
    pub fn label(&self) -> String {
        match self.display {
            PageDisplay::Arabic => match &self.formatter {
                Some(formatter) => formatter(self.numbers()),
                None => self.pages.arabic(),
            },
            PageDisplay::Dots => {
                self.pages
                    .dots(&self.active_dot, &self.inactive_dot, self.dot_budget)
            }
        }
    }
}
impl Component for Paginator {
    type Message = Action;
    fn event(&mut self, input: &Input, focused: bool) -> Response<Action> {
        if !focused {
            return Response::ignored();
        }
        let Some(key) = input.key_down() else {
            return Response::ignored();
        };
        let previous = self.pages.page();
        if self.keys.next.matches(key) {
            self.pages.next();
        } else if self.keys.previous.matches(key) {
            self.pages.previous();
        } else {
            return Response::ignored();
        }
        Response::consumed((self.pages.page() != previous).then_some(Action::Changed))
    }
    fn view(&self, theme: &Theme, focused: bool) -> View {
        match self.display {
            PageDisplay::Arabic => View::styled(
                self.label(),
                if focused { theme.accent } else { theme.muted },
            ),
            PageDisplay::Dots => View::row(self.pages.visible_pages(self.dot_budget).map(|page| {
                if page == self.pages.page() {
                    View::styled(
                        &self.active_dot,
                        if focused { theme.focus } else { theme.accent },
                    )
                } else {
                    View::styled(&self.inactive_dot, theme.muted)
                }
            })),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use matea_core::{KeyKind, Modifiers};
    use matea_render::Size;
    fn key(code: KeyCode) -> Input {
        Input::Key(Key::new(code))
    }
    #[test]
    fn custom_bindings_repeat_release_focus_and_boundaries_share_one_state() {
        let mut pager = Paginator::default();
        pager.pages.set_total_pages(3);
        let event = key(KeyCode::Right);
        assert!(!pager.event(&event, false).consumed);
        assert!(
            !pager
                .event(
                    &Input::Key(Key::new(KeyCode::Right).with_kind(KeyKind::Release)),
                    true
                )
                .consumed
        );
        assert!(
            !pager
                .event(
                    &Input::Key(Key::new(KeyCode::Right).with_modifiers(Modifiers::CONTROL)),
                    true
                )
                .consumed
        );
        assert_eq!(pager.event(&event, true).message, Some(Action::Changed));
        let repeat = Input::Key(Key::new(KeyCode::Right).with_kind(KeyKind::Repeat));
        assert_eq!(pager.event(&repeat, true).message, Some(Action::Changed));
        assert!(pager.pages.is_last());
        let endpoint = pager.event(&repeat, true);
        assert!(endpoint.consumed && endpoint.message.is_none());
        pager.keys.previous = Binding::new([Key::new(KeyCode::Char('p'))], "p", "previous page");
        assert!(!pager.event(&key(KeyCode::Left), true).consumed);
        pager.event(&key(KeyCode::Char('p')), true);
        assert_eq!(pager.pages.page(), 1);
        pager.keys.previous.enabled = false;
        assert!(!pager.event(&key(KeyCode::Char('p')), true).consumed);
        assert!(pager.keys.previous.help().is_none());
    }
    #[test]
    fn custom_formatting_and_dots_use_bounded_coherent_views() {
        let mut pager = Paginator::new(Pagination::from_pages(4, NonZeroUsize::new(2).unwrap()))
            .with_formatter(|n| format!("page {:02} of {:02}", n.current, n.total));
        assert_eq!(pager.label(), "page 01 of 04");
        pager.pages.set_page(1);
        assert_eq!(pager.label(), "page 02 of 04");
        assert!(
            pager
                .view(&Theme::plain(), true)
                .render(Size::new(30, 1))
                .unwrap()
                .plain()
                .starts_with("page 02 of 04")
        );
        pager.reset_formatter();
        assert_eq!(pager.label(), "2/4");
        pager.display = PageDisplay::Dots;
        pager.active_dot = "X".into();
        pager.inactive_dot = ".".into();
        pager.dot_budget = 3;
        assert_eq!(pager.label(), ".X.");
        assert_eq!(
            pager
                .view(&Theme::plain(), true)
                .render(Size::new(3, 1))
                .unwrap()
                .plain(),
            ".X."
        );
        pager.pages.set_total_pages(0);
        assert_eq!(pager.label(), "");
        pager.display = PageDisplay::Arabic;
        assert_eq!(pager.label(), "0/0");
    }
}

#![forbid(unsafe_code)]
//! Pinned Rust widgets rendered into owned Matea cell scenes.
//!
//! This adapter never creates a terminal or retains a widget/state borrow in a
//! View. Unsupported raw buffer contracts return typed errors; they are not
//! silently converted into ordinary text. API/crate naming remains provisional.
use buffer::{Buffer, CellDiffOption, CellWidth};
use matea_render::{GeometryError, Size, Surface, View};
use matea_style::{BlinkStyle, Color, Style, TextAttribute, UnderlineStyle};
pub use ratatui_upstream as upstream;
#[cfg(feature = "macros")]
pub use ratatui_upstream::macros;
pub use ratatui_upstream::{buffer, layout, style, symbols, text, widgets};
use std::cell::{Ref, RefCell};
#[cfg(feature = "calendar")]
pub use time;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ImportError {
    Geometry(GeometryError),
    InvalidBufferLength,
    DiffOption {
        x: u16,
        y: u16,
        option: CellDiffOption,
    },
    DeprecatedSkip {
        x: u16,
        y: u16,
    },
    InvalidSymbol {
        x: u16,
        y: u16,
    },
    ZeroWidth {
        x: u16,
        y: u16,
    },
    WidthPolicy {
        x: u16,
        y: u16,
        source: u16,
        matea: usize,
    },
    ClippedGlyph {
        x: u16,
        y: u16,
    },
    OverlappingGlyph {
        x: u16,
        y: u16,
    },
    UnknownModifiers {
        x: u16,
        y: u16,
    },
    ConflictingBlink {
        x: u16,
        y: u16,
    },
    WidgetChangedGeometry,
    StateBorrowed,
}
impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Ratatui cell import: {self:?}")
    }
}
impl std::error::Error for ImportError {}

fn color(value: style::Color) -> Color {
    use style::Color as C;
    match value {
        C::Reset => Color::Default,
        C::Black => Color::Indexed(0),
        C::Red => Color::Indexed(1),
        C::Green => Color::Indexed(2),
        C::Yellow => Color::Indexed(3),
        C::Blue => Color::Indexed(4),
        C::Magenta => Color::Indexed(5),
        C::Cyan => Color::Indexed(6),
        C::Gray => Color::Indexed(7),
        C::DarkGray => Color::Indexed(8),
        C::LightRed => Color::Indexed(9),
        C::LightGreen => Color::Indexed(10),
        C::LightYellow => Color::Indexed(11),
        C::LightBlue => Color::Indexed(12),
        C::LightMagenta => Color::Indexed(13),
        C::LightCyan => Color::Indexed(14),
        C::White => Color::Indexed(15),
        C::Indexed(i) => Color::Indexed(i),
        C::Rgb(r, g, b) => Color::Rgb(r, g, b),
    }
}
fn cell_style(cell: &buffer::Cell, x: u16, y: u16) -> Result<Style, ImportError> {
    use style::Modifier as M;
    let flags = cell.modifier;
    if flags.bits() & !M::all().bits() != 0 {
        return Err(ImportError::UnknownModifiers { x, y });
    }
    if flags.contains(M::SLOW_BLINK | M::RAPID_BLINK) {
        return Err(ImportError::ConflictingBlink { x, y });
    }
    let mut result = Style::default()
        .fg(color(cell.fg))
        .bg(color(cell.bg))
        .underline_color(color(cell.underline_color));
    for (a, b) in [
        (TextAttribute::Bold, M::BOLD),
        (TextAttribute::Dim, M::DIM),
        (TextAttribute::Italic, M::ITALIC),
        (TextAttribute::Reverse, M::REVERSED),
        (TextAttribute::Hidden, M::HIDDEN),
        (TextAttribute::Strikethrough, M::CROSSED_OUT),
    ] {
        result = result.attribute(a, flags.contains(b));
    }
    result = result.underline_style(if flags.contains(M::UNDERLINED) {
        UnderlineStyle::Single
    } else {
        UnderlineStyle::None
    });
    result = result.blink(if flags.contains(M::RAPID_BLINK) {
        BlinkStyle::Rapid
    } else if flags.contains(M::SLOW_BLINK) {
        BlinkStyle::Slow
    } else {
        BlinkStyle::None
    });
    Ok(result)
}

/// Import a complete source grid, normalizing its origin to local (0,0).
/// Geometry, flags and symbols are checked before a scene can be returned.
/// Covered blank cells under a wide glyph have no independent visible style.
/// Raw diff/skip/forced-width contracts and differing width policies remain
/// unsupported and return errors. In particular this does not silently change
/// the source's halfwidth-katakana sound-mark policy into Matea's width policy.
#[allow(deprecated)]
pub fn surface(buffer: &Buffer) -> Result<Surface, ImportError> {
    let size = Size::new(buffer.area.width, buffer.area.height);
    let count = usize::from(size.width) * usize::from(size.height);
    if buffer.content.len() != count {
        return Err(ImportError::InvalidBufferLength);
    }
    let result = Surface::new(size).map_err(ImportError::Geometry)?;
    import_into(buffer, result)
}
#[allow(deprecated)]
fn import_into(buffer: &Buffer, mut result: Surface) -> Result<Surface, ImportError> {
    let size = result.size();
    if buffer.content.len() != usize::from(size.width) * usize::from(size.height) {
        return Err(ImportError::InvalidBufferLength);
    }
    // Resolved source styles commonly span many adjacent cells. Retain one
    // checked value without allocating a map or trusting an unchecked modifier.
    let mut previous_style = None;
    for y in 0..size.height {
        let mut covered = 0;
        let mut ascii = [0u8; 256];
        let mut run_length = 0;
        let mut run_x = 0;
        let mut run_style = Style::default();
        for x in 0..size.width {
            let cell = &buffer.content[usize::from(y) * usize::from(size.width) + usize::from(x)];
            if cell.diff_option != CellDiffOption::None {
                return Err(ImportError::DiffOption {
                    x,
                    y,
                    option: cell.diff_option,
                });
            }
            if cell.skip {
                return Err(ImportError::DeprecatedSkip { x, y });
            }
            let key = (cell.fg, cell.bg, cell.underline_color, cell.modifier);
            let resolved = match previous_style {
                Some((old, resolved)) if old == key => resolved,
                _ => {
                    let resolved = cell_style(cell, x, y)?;
                    previous_style = Some((key, resolved));
                    resolved
                }
            };
            let symbol = cell.symbol();
            if covered != 0 {
                if symbol != " " {
                    return Err(ImportError::OverlappingGlyph { x, y });
                }
                covered -= 1;
                continue;
            }
            // One printable ASCII byte is already one width-one EGC in both
            // pinned policies. Avoid re-running scalar, grapheme and source
            // halfwidth-katakana scans for the ordinary terminal grid.
            if let [byte @ b' '..=b'~'] = symbol.as_bytes() {
                if run_length != 0 && (run_length == ascii.len() || run_style != resolved) {
                    paint_ascii(&mut result, y, run_x, &ascii[..run_length], run_style);
                    run_length = 0;
                }
                if run_length == 0 {
                    run_x = x;
                    run_style = resolved;
                }
                ascii[run_length] = *byte;
                run_length += 1;
                continue;
            }
            if run_length != 0 {
                paint_ascii(&mut result, y, run_x, &ascii[..run_length], run_style);
                run_length = 0;
            }
            if symbol.is_empty() || symbol.chars().any(char::is_control) {
                return Err(ImportError::InvalidSymbol { x, y });
            }
            let mut clusters = symbol.graphemes(true);
            if clusters.next() != Some(symbol) || clusters.next().is_some() {
                return Err(ImportError::InvalidSymbol { x, y });
            }
            let width = symbol.width();
            let source = cell.cell_width();
            if width == 0 {
                return Err(ImportError::ZeroWidth { x, y });
            }
            if width != usize::from(source) {
                return Err(ImportError::WidthPolicy {
                    x,
                    y,
                    source,
                    matea: width,
                });
            }
            if width > usize::from(size.width - x) {
                return Err(ImportError::ClippedGlyph { x, y });
            }
            result.canvas(result.area()).text(x, y, symbol, resolved);
            covered = source - 1;
        }
        if run_length != 0 {
            paint_ascii(&mut result, y, run_x, &ascii[..run_length], run_style);
        }
    }
    Ok(result)
}
fn paint_ascii(result: &mut Surface, y: u16, x: u16, bytes: &[u8], style: Style) {
    // Filled exclusively from one-byte printable ASCII source cells. Joining
    // these cells cannot merge grapheme boundaries or change their width.
    let text = std::str::from_utf8(bytes).expect("validated printable ASCII run");
    result.canvas(result.area()).text(x, y, text, style);
}
/// Convert an owned source buffer into an independent immutable scene.
pub fn view(buffer: Buffer) -> Result<View, ImportError> {
    surface(&buffer).map(View::surface)
}
/// Render with the pinned Widget API without opening another terminal.
pub fn render<W: widgets::Widget>(size: Size, widget: W) -> Result<View, ImportError> {
    // Reject oversized allocations before constructing a foreign buffer.
    let target = Surface::new(size).map_err(ImportError::Geometry)?;
    let area = layout::Rect::new(0, 0, size.width, size.height);
    let mut buffer = Buffer::empty(area);
    widget.render(area, &mut buffer);
    if buffer.area != area {
        return Err(ImportError::WidgetChangedGeometry);
    }
    import_into(&buffer, target).map(View::surface)
}
/// Source widget state may change before an import error/panic. Generic state
/// has no implicit clone/rollback. Its borrow ends before the scene is returned.
pub fn render_stateful<W: widgets::StatefulWidget>(
    size: Size,
    widget: W,
    state: &mut W::State,
) -> Result<View, ImportError> {
    let target = Surface::new(size).map_err(ImportError::Geometry)?;
    let area = layout::Rect::new(0, 0, size.width, size.height);
    let mut buffer = Buffer::empty(area);
    widget.render(area, &mut buffer, state);
    if buffer.area != area {
        return Err(ImportError::WidgetChangedGeometry);
    }
    import_into(&buffer, target).map(View::surface)
}

/// UI-local render state for a read-only Model::view. Event handling can update
/// state through &mut self without a dynamic borrow. Only pixels escape render;
/// no Send/Sync bounds are imposed on the app's source widget state.
#[derive(Debug, Default)]
pub struct WidgetState<S>(RefCell<S>);
impl<S> WidgetState<S> {
    pub const fn new(state: S) -> Self {
        Self(RefCell::new(state))
    }
    pub fn state(&self) -> Result<Ref<'_, S>, ImportError> {
        self.0.try_borrow().map_err(|_| ImportError::StateBorrowed)
    }
    pub fn state_mut(&mut self) -> &mut S {
        self.0.get_mut()
    }
    pub fn render<W: widgets::StatefulWidget<State = S>>(
        &self,
        size: Size,
        widget: W,
    ) -> Result<View, ImportError> {
        let mut state = self
            .0
            .try_borrow_mut()
            .map_err(|_| ImportError::StateBorrowed)?;
        render_stateful(size, widget, &mut *state)
    }
}

/// A native decoration the pinned source Style cannot represent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum StyleError {
    Attribute(TextAttribute),
    Underline(UnderlineStyle),
    Frame(matea_style::TextFrame),
}
impl std::fmt::Display for StyleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unrepresentable Ratatui style: {self:?}")
    }
}
impl std::error::Error for StyleError {}
fn source_color(color: Color) -> style::Color {
    match color {
        Color::Default => style::Color::Reset,
        Color::Indexed(i) => style::Color::Indexed(i),
        Color::Rgb(r, g, b) => style::Color::Rgb(r, g, b),
    }
}
/// Convert a resolved Matea theme token to a complete explicit source override.
/// False/default properties clear the previous source style. Decorations without
/// a faithful source representation return an error rather than disappearing.
pub fn to_style(value: Style) -> Result<style::Style, StyleError> {
    use style::Modifier as M;
    for attribute in [TextAttribute::Fraktur, TextAttribute::Overline] {
        if value.has(attribute) {
            return Err(StyleError::Attribute(attribute));
        }
    }
    if value.attributes.frame() != matea_style::TextFrame::None {
        return Err(StyleError::Frame(value.attributes.frame()));
    }
    if !matches!(
        value.attributes.underline(),
        UnderlineStyle::None | UnderlineStyle::Single
    ) {
        return Err(StyleError::Underline(value.attributes.underline()));
    }
    let mut modifiers = M::empty();
    for (a, b) in [
        (TextAttribute::Bold, M::BOLD),
        (TextAttribute::Dim, M::DIM),
        (TextAttribute::Italic, M::ITALIC),
        (TextAttribute::Reverse, M::REVERSED),
        (TextAttribute::Hidden, M::HIDDEN),
        (TextAttribute::Strikethrough, M::CROSSED_OUT),
    ] {
        if value.has(a) {
            modifiers.insert(b);
        }
    }
    if value.attributes.underline() == UnderlineStyle::Single {
        modifiers.insert(M::UNDERLINED);
    }
    modifiers |= match value.attributes.blink() {
        BlinkStyle::None => M::empty(),
        BlinkStyle::Slow => M::SLOW_BLINK,
        BlinkStyle::Rapid => M::RAPID_BLINK,
    };
    Ok(style::Style::reset()
        .fg(source_color(value.fg))
        .bg(source_color(value.bg))
        .underline_color(source_color(value.underline_color))
        .add_modifier(modifiers))
}

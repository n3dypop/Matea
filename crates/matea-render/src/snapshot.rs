//! Immutable owned cell scenes for independent renderers and component adapters.
use super::{Canvas, Cursor, Node, Rect, Surface, View};
use std::sync::Arc;

impl View {
    /// Capture an owned cell surface, or share an existing immutable Arc.
    /// Cells keep their exact styles and grapheme ownership. Layout clips whole
    /// glyphs without scaling; the source's height supplies the natural height.
    /// A source cursor follows the same clipping/paint-order policy as with_cursor.
    /// The snapshot retains no mutable renderer/widget/state borrow.
    ///
    /// ```
    /// use matea_render::{Size, View};
    /// let cells = View::text("抹茶").render(Size::new(4, 1))?;
    /// let snapshot = View::surface(cells);
    /// let scene = View::column([snapshot.clone(), View::text("Matea")]);
    /// assert_eq!(scene.render(Size::new(5, 2))?.plain(), "抹茶\nMatea");
    /// # Ok::<(), matea_render::GeometryError>(())
    /// ```
    pub fn surface(surface: impl Into<Arc<Surface>>) -> Self {
        Self::new(Node::Snapshot(surface.into()))
    }

    /// Consume the scene. An unconstrained root snapshot of exactly the
    /// requested size transfers uniquely owned cells instead of copying them.
    /// Shared snapshots clone their cells; other scenes use ordinary rendering.
    pub fn into_surface(self, size: super::Size) -> Result<Surface, super::GeometryError> {
        match self {
            Self {
                node: Node::Snapshot(surface),
                width: None,
                height: None,
            } if surface.size() == size => {
                Ok(Arc::try_unwrap(surface).unwrap_or_else(|shared| shared.as_ref().clone()))
            }
            view => view.render(size),
        }
    }
}
impl Canvas<'_> {
    /// Copy an existing validated cell grid at x/y within the inherited clip.
    /// Source cells are opaque; partial source glyphs are skipped. Overwriting
    /// part of a destination wide glyph erases its complete old ownership, as
    /// ordinary text painting does. This copies cells only; cursor metadata is
    /// applied by View::surface or explicitly by Surface::set_cursor.
    pub fn surface(&mut self, x: u16, y: u16, source: &Surface) {
        if self.clip.width == 0 || self.clip.height == 0 {
            return;
        }
        let (x, y) = (u32::from(x), u32::from(y));
        let left = u32::from(self.clip.x).saturating_sub(x) as usize;
        let right = u32::from(self.clip.right())
            .saturating_sub(x)
            .min(u32::from(source.size.width)) as usize;
        let top = u32::from(self.clip.y).saturating_sub(y);
        let bottom = u32::from(self.clip.bottom())
            .saturating_sub(y)
            .min(u32::from(source.size.height));
        if left >= right || top >= bottom {
            return;
        }
        for source_y in top..bottom {
            let row = source.row(source_y as u16);
            let (mut start, mut end) = (left, right);
            // The immutable source already satisfies the cell ownership
            // invariant. Trim a clipped continuation back to whole glyphs.
            while start < end && row[start].is_continuation() {
                start += 1;
            }
            if end < row.len() {
                while end > start && row[end].is_continuation() {
                    end -= 1;
                }
            }
            if start == end {
                continue;
            }
            let dest_y = (y + source_y) as u16;
            let dest_x = (x + start as u32) as u16;
            let dest_end = (x + end as u32) as u16;
            // Only old glyphs touching the two edges can extend outside the
            // replaced interval. Repair those, then clone one contiguous row.
            self.surface.erase_glyph(dest_x, dest_y);
            self.surface.erase_glyph(dest_end - 1, dest_y);
            if self.surface.cursor.is_some_and(|cursor| {
                cursor.y == dest_y && cursor.x >= dest_x && cursor.x < dest_end
            }) {
                self.surface.cursor = None;
            }
            let index = self.surface.index(dest_x, dest_y);
            self.surface.cells[index..index + end - start].clone_from_slice(&row[start..end]);
        }
    }
}

pub(super) fn paint_cursor(surface: &mut Surface, area: Rect, clip: Rect, cursor: Cursor) {
    let absolute = Cursor::new(
        area.x.saturating_add(cursor.x),
        area.y.saturating_add(cursor.y),
    );
    if absolute.x >= clip.x
        && absolute.x < clip.right()
        && absolute.y >= clip.y
        && absolute.y < clip.bottom()
    {
        surface.set_cursor(Some(absolute));
        if surface.cursor().is_some_and(|cursor| cursor.x < clip.x) {
            surface.set_cursor(None);
        }
    }
}

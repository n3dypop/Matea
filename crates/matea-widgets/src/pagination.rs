//! Clamped pagination without zero divisors or public out-of-range state.
use std::{num::NonZeroUsize, ops::Range};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Extent {
    Items(usize),
    Pages(usize),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pagination {
    extent: Extent,
    per_page: NonZeroUsize,
    page: usize,
}
impl Pagination {
    pub fn new(total: usize, per_page: NonZeroUsize) -> Self {
        Self {
            extent: Extent::Items(total),
            per_page,
            page: 0,
        }
    }
    /// A page count independent of any collection length (documents, virtual
    /// pages, etc.). Changing per-page does not change this explicit count.
    pub fn from_pages(total_pages: usize, per_page: NonZeroUsize) -> Self {
        Self {
            extent: Extent::Pages(total_pages),
            per_page,
            page: 0,
        }
    }
    pub fn per_page(self) -> NonZeroUsize {
        self.per_page
    }
    pub fn total_items(self) -> Option<usize> {
        match self.extent {
            Extent::Items(total) => Some(total),
            Extent::Pages(_) => None,
        }
    }
    pub fn page(self) -> usize {
        self.page
    }
    pub fn total_pages(self) -> usize {
        match self.extent {
            Extent::Items(total) => total.div_ceil(self.per_page.get()),
            Extent::Pages(total) => total,
        }
    }
    pub fn is_first(self) -> bool {
        self.page == 0
    }
    pub fn is_last(self) -> bool {
        self.page + 1 >= self.total_pages()
    }
    pub fn set_page(&mut self, page: usize) {
        self.page = page.min(self.total_pages().saturating_sub(1));
    }
    pub fn set_total(&mut self, total: usize) {
        self.extent = Extent::Items(total);
        self.set_page(self.page);
    }
    pub fn set_total_pages(&mut self, pages: usize) {
        self.extent = Extent::Pages(pages);
        self.set_page(self.page);
    }
    pub fn set_per_page(&mut self, per_page: NonZeroUsize) {
        self.per_page = per_page;
        self.set_page(self.page);
    }
    pub fn next(&mut self) {
        self.set_page(self.page.saturating_add(1));
    }
    pub fn previous(&mut self) {
        self.page = self.page.saturating_sub(1);
    }
    #[inline]
    pub fn range(self) -> Range<usize> {
        match self.extent {
            Extent::Items(total) => {
                // Private, clamped state proves page*per_page <= total. Limit
                // the remaining count before addition, avoiding end overflow.
                let start = self.page * self.per_page.get();
                start..start + self.per_page.get().min(total - start)
            }
            Extent::Pages(pages) => self.range_for(pages.saturating_mul(self.per_page.get())),
        }
    }
    /// Clamp to the actual collection, even if a separately configured page
    /// count is stale or a virtual offset exceeds representable index space.
    pub fn range_for(self, length: usize) -> Range<usize> {
        let start = self.page.saturating_mul(self.per_page.get()).min(length);
        start..start.saturating_add(self.per_page.get()).min(length)
    }
    pub fn items_on_page(self, length: usize) -> usize {
        self.range_for(length).len()
    }
    pub fn arabic(self) -> String {
        format!(
            "{}/{}",
            if self.total_pages() == 0 {
                0
            } else {
                self.page + 1
            },
            self.total_pages()
        )
    }
    /// Bounded window containing the active page, or empty for a zero budget.
    pub fn visible_pages(self, budget: usize) -> Range<usize> {
        let pages = self.total_pages();
        let count = pages.min(budget);
        if count == 0 {
            return 0..0;
        }
        let start = self.page.saturating_sub(count / 2).min(pages - count);
        start..start + count
    }
    /// Dot output is bounded by the caller's budget and keeps the active page.
    pub fn dots(self, active: &str, inactive: &str, max_pages: usize) -> String {
        let range = self.visible_pages(max_pages);
        if range.is_empty() {
            return String::new();
        }
        // The window contains exactly one active dot. Reserve its exact byte
        // size once; overflow still falls back to ordinary String growth.
        let capacity = (range.len() - 1)
            .checked_mul(inactive.len())
            .and_then(|bytes| bytes.checked_add(active.len()))
            .unwrap_or(0);
        let mut result = String::with_capacity(capacity);
        for page in range {
            result.push_str(if page == self.page { active } else { inactive });
        }
        result
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn range_remains_valid_on_empty_data_and_resize() {
        for total in 0..64 {
            for per in 1..16 {
                let mut pages = Pagination::new(total, NonZeroUsize::new(per).unwrap());
                for selected in [0, 1, 63, usize::MAX] {
                    pages.set_page(selected);
                    let range = pages.range();
                    assert!(range.start <= range.end && range.end <= total);
                    assert!(range.len() <= per);
                    pages.next();
                    pages.previous();
                    assert!(pages.range().end <= total);
                }
                pages.set_total(0);
                assert_eq!(pages.range(), 0..0);
                assert_eq!(pages.arabic(), "0/0");
            }
        }
        let mut pages = Pagination::new(usize::MAX, NonZeroUsize::new(2).unwrap());
        pages.set_page(usize::MAX);
        assert_eq!(pages.range().end, usize::MAX);
        assert_eq!(pages.dots("●", "○", 3), "○○●");
        assert_eq!(pages.dots("●", "○", 0), "");
        assert_eq!(pages.visible_pages(1).len(), 1);
    }
    #[test]
    fn explicit_pages_are_independent_and_stale_collections_never_panic() {
        let mut pages = Pagination::from_pages(10, NonZeroUsize::new(3).unwrap());
        pages.set_page(9);
        assert_eq!(pages.total_items(), None);
        assert_eq!(pages.range(), 27..30);
        assert_eq!(pages.range_for(4), 4..4);
        assert_eq!(pages.items_on_page(4), 0);
        pages.set_per_page(NonZeroUsize::new(2).unwrap());
        assert_eq!(pages.total_pages(), 10);
        assert_eq!(pages.range(), 18..20);
        pages.set_total(5);
        assert_eq!(pages.total_items(), Some(5));
        assert_eq!(pages.page(), 2);
        assert_eq!(pages.items_on_page(5), 1);
        pages.set_total_pages(0);
        assert_eq!(pages.page(), 0);
        assert_eq!(pages.arabic(), "0/0");
        pages.set_total_pages(usize::MAX);
        pages.set_page(usize::MAX);
        assert!(pages.is_last());
        assert_eq!(pages.range_for(usize::MAX), usize::MAX..usize::MAX);
        pages.next();
        assert!(pages.is_last());
    }
    #[test]
    fn index_limits_preserve_slice_invariants_across_all_setters() {
        for total in [0, 1, usize::MAX / 2, usize::MAX - 1, usize::MAX] {
            for per in [1, 2, usize::MAX / 2, usize::MAX] {
                let mut pages = Pagination::new(total, NonZeroUsize::new(per).unwrap());
                for selected in [0, 1, usize::MAX / 2, usize::MAX] {
                    pages.set_page(selected);
                    let range = pages.range();
                    assert!(range.start <= range.end && range.end <= total);
                    assert!(range.len() <= per);
                    let visible = pages.visible_pages(3);
                    assert!(visible.len() <= 3);
                    if total > 0 {
                        assert!(visible.contains(&pages.page()));
                    }
                    pages.set_total(total / 2);
                    assert!(pages.range().end <= total / 2);
                    pages.set_per_page(NonZeroUsize::new(usize::MAX).unwrap());
                    assert!(pages.range().end <= total / 2);
                    pages.set_per_page(NonZeroUsize::new(per).unwrap());
                    pages.set_total(total);
                }
            }
        }
    }
}

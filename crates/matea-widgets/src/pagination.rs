//! Clamped pagination without zero divisors or public out-of-range state.
use std::{num::NonZeroUsize, ops::Range};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pagination {
    total: usize,
    per_page: NonZeroUsize,
    page: usize,
}
impl Pagination {
    pub fn new(total: usize, per_page: NonZeroUsize) -> Self {
        Self {
            total,
            per_page,
            page: 0,
        }
    }
    pub fn page(self) -> usize {
        self.page
    }
    pub fn total_pages(self) -> usize {
        self.total.div_ceil(self.per_page.get())
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
        self.total = total;
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
    pub fn range(self) -> Range<usize> {
        let start = self.page * self.per_page.get();
        start..start.saturating_add(self.per_page.get()).min(self.total)
    }
    pub fn arabic(self) -> String {
        format!(
            "{}/{}",
            if self.total == 0 { 0 } else { self.page + 1 },
            self.total_pages()
        )
    }
    /// Dot output is explicitly bounded by the caller's display budget.
    pub fn dots(self, active: &str, inactive: &str, max_pages: usize) -> String {
        (0..self.total_pages().min(max_pages))
            .map(|page| if page == self.page { active } else { inactive })
            .collect()
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
        assert_eq!(pages.dots("●", "○", 3), "○○○");
    }
}

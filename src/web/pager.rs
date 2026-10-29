//! Moving between the pages of a list: first, previous, the page numbers nearby, next, last and a
//! field to type the page

/// Page numbers shown on each side of the current page
const NEARBY: usize = 2;

/// A page number or a gap (`…`) among them
pub struct PageLink {
    /// `None` for a gap
    pub number: Option<usize>,
    pub href: String,
    pub current: bool,
}

/// The navigation below a list (`pager.html`)
pub struct Pager {
    pub page: usize,
    pub pages: usize,
    pub first: Option<String>,
    pub prev: Option<String>,
    pub next: Option<String>,
    pub last: Option<String>,
    pub links: Vec<PageLink>,
    /// Where the form with the typed page goes, and the fields that keep the list as it is
    pub action: &'static str,
    pub fields: Vec<(&'static str, String)>,
}

impl Pager {
    /// `href` gives the address of a page
    pub fn new(
        page: usize,
        pages: usize,
        href: impl Fn(usize) -> String,
        action: &'static str,
        fields: Vec<(&'static str, String)>,
    ) -> Self {
        let other = |p: usize| (p != page).then(|| href(p));
        Pager {
            page,
            pages,
            first: (page > 1).then(|| href(1)),
            prev: (page > 1).then(|| href(page - 1)),
            next: (page < pages).then(|| href(page + 1)),
            last: (page < pages).then(|| href(pages)),
            links: numbers(page, pages)
                .into_iter()
                .map(|number| PageLink {
                    number,
                    href: number.and_then(other).unwrap_or_default(),
                    current: number == Some(page),
                })
                .collect(),
            action,
            fields,
        }
    }
}

/// The page numbers to show: the first, the last and those near the current one, `None` for a gap
/// (a gap of a single page shows that page instead)
fn numbers(page: usize, pages: usize) -> Vec<Option<usize>> {
    let mut shown: Vec<usize> = (1..=pages)
        .filter(|p| *p == 1 || *p == pages || p.abs_diff(page) <= NEARBY)
        .collect();
    shown.dedup();
    let mut out = Vec::new();
    let mut before = 0;
    for p in shown {
        match p - before {
            1 => {}
            2 => out.push(Some(p - 1)),
            _ => out.push(None),
        }
        out.push(Some(p));
        before = p;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shown(page: usize, pages: usize) -> String {
        numbers(page, pages)
            .iter()
            .map(|n| n.map_or("…".to_owned(), |n| n.to_string()))
            .collect::<Vec<_>>()
            .join(" ")
    }

    #[test]
    fn the_numbers_near_the_page_and_the_ends_are_shown() {
        assert_eq!(shown(1, 1), "1");
        assert_eq!(shown(1, 3), "1 2 3");
        assert_eq!(shown(7, 34), "1 … 5 6 7 8 9 … 34");
        assert_eq!(shown(1, 34), "1 2 3 … 34");
        assert_eq!(shown(34, 34), "1 … 32 33 34");
        // a gap of one page shows the page
        assert_eq!(shown(5, 10), "1 2 3 4 5 6 7 … 10");
    }

    #[test]
    fn links_lead_to_the_other_pages() {
        let p = Pager::new(2, 3, |p| format!("/list?page={p}"), "/list", Vec::new());
        assert_eq!(p.first.as_deref(), Some("/list?page=1"));
        assert_eq!(p.last.as_deref(), Some("/list?page=3"));
        let current: Vec<_> = p.links.iter().filter(|l| l.current).collect();
        assert_eq!((current.len(), current[0].href.as_str()), (1, ""));
        let p = Pager::new(3, 3, |p| format!("/list?page={p}"), "/list", Vec::new());
        assert!(p.next.is_none() && p.last.is_none());
    }
}

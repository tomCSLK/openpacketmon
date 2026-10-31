//! The formatting of [`crate::view::format`], plus what only HTML pages need.

use crate::utils::types::timestamp::Timestamp;
pub use crate::view::format::*;

/// A timestamp for HTML pages: `<time>` with the exact instant in `datetime` and the UTC text as
/// content; `static/time.js` shows it in the browser's time zone. Contains only digits and fixed
/// text, so it may be rendered with `|safe`.
pub fn time_html(ts: Timestamp) -> String {
    let (y, mo, d, h, mi, s) = civil_utc(ts.secs());
    format!(
        "<time class=\"ts\" datetime=\"{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z\">{}</time>",
        time_utc(ts)
    )
}

/// The time element of a timestamp; templates call `{{ ts.html()|safe }}`
pub trait TimeHtml {
    fn html(&self) -> String;
}

impl TimeHtml for Timestamp {
    fn html(&self) -> String {
        time_html(*self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_element() {
        assert_eq!(
            time_html(Timestamp::new(1_781_551_930, 0)),
            "<time class=\"ts\" datetime=\"2026-06-15T19:32:10Z\">2026-06-15 19:32:10 UTC</time>"
        );
    }
}

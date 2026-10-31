//! The language of a web request. The catalogs themselves are in [`crate::view::i18n`].

pub use crate::view::i18n::Lang;

tokio::task_local! {
    static REQUEST_LANG: Lang;
}

/// Runs a request in its language (settings or `Accept-Language`), see [`current`]
pub async fn scope<F: Future>(lang: Lang, request: F) -> F::Output {
    REQUEST_LANG.scope(lang, request).await
}

/// The language of the current request, for responses built without the request at hand
/// (error pages); English outside of a request
pub fn current() -> Lang {
    REQUEST_LANG.try_with(|l| *l).unwrap_or_default()
}

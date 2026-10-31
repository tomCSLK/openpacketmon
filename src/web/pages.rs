//! Parts shared by all pages: the frame with the sidebar, flash messages and the pages
//! shown without a session or after an error.

use askama::Template;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

use crate::engine::state::EngineState;
use crate::view::Status;
use crate::web::auth::Session;
use crate::web::i18n::Lang;

/// Data every page with the sidebar needs
pub struct Layout {
    pub csrf_token: String,
    /// Notifications not seen yet (badge in the navigation)
    pub unread: usize,
    /// A newer release, if the update check found one
    pub update: Option<String>,
    /// There is no country database (hint on the pages that show countries)
    pub geo_missing: bool,
    /// A database of DB-IP is used: link to db-ip.com, as its license asks
    pub geo_dbip: bool,
}

impl Layout {
    pub fn new(state: &EngineState, session: &Session) -> Self {
        let Status {
            unread,
            update,
            geo_missing,
            geo_dbip,
        } = Status::read(state);
        Self {
            csrf_token: session.csrf_token.clone(),
            unread,
            update,
            geo_missing,
            geo_dbip,
        }
    }
}

/// Renders a template; a template error (a bug) becomes a plain 500
pub fn render(page: &impl Template) -> Response {
    match page.render() {
        Ok(html) => Html(html).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

/// Message shown after a successful action (`?msg=` is mapped to fixed texts, never echoed)
pub fn flash_message(t: Lang, key: Option<&str>) -> Option<String> {
    let id = match key? {
        "settings_saved" => "flash-settings-saved",
        "notifications_cleared" => "flash-notifications-cleared",
        "capture_stopped" => "flash-capture-stopped",
        "geo_downloaded" => "flash-geo-downloaded",
        _ => return None,
    };
    Some(t.get(id))
}

#[derive(Template)]
#[template(path = "signed_out.html")]
struct SignedOutPage {
    t: Lang,
}

/// The page for browsers without a session: how to open openpacketmon with the start link
pub fn signed_out(t: Lang) -> Response {
    (StatusCode::UNAUTHORIZED, render(&SignedOutPage { t })).into_response()
}

#[derive(Template)]
#[template(path = "error.html")]
struct ErrorPage {
    t: Lang,
    status: u16,
    message: String,
}

pub fn error_page(t: Lang, status: StatusCode, message: String) -> Response {
    render(&ErrorPage {
        t,
        status: status.as_u16(),
        message,
    })
}

//! The page of the notifications. Opening it marks them as read.

use askama::Template;
use axum::Form;
use axum::extract::{Query, State};
use axum::response::{Redirect, Response};
use serde::Deserialize;

use crate::view::notifications::{NotificationRow, notification_list};
use crate::web::auth::{CsrfForm, Session};
use crate::web::i18n::{self, Lang};
use crate::web::jump::Href;
use crate::web::pages::{Layout, flash_message, render};
use crate::web::{AppState, WebError};

#[derive(Template)]
#[template(path = "notifications.html")]
struct NotificationsPage {
    t: Lang,
    layout: Layout,
    flash: Option<String>,
    rows: Vec<NotificationRow>,
    total: usize,
    any_active: bool,
}

#[derive(Deserialize)]
pub struct PageQuery {
    msg: Option<String>,
}

/// `GET /notifications`
pub async fn page(
    State(state): State<AppState>,
    session: Session,
    Query(query): Query<PageQuery>,
) -> Response {
    let t = i18n::current();
    let mut engine = state.engine.lock();
    engine.mark_notifications_read();
    let list = notification_list(t, &engine);
    render(&NotificationsPage {
        t,
        layout: Layout::new(&engine, &session),
        flash: flash_message(t, query.msg.as_deref()),
        rows: list.rows,
        total: list.total,
        any_active: list.any_active,
    })
}

/// `POST /notifications/clear`
pub async fn clear(
    State(state): State<AppState>,
    session: Session,
    Form(form): Form<CsrfForm>,
) -> Result<Redirect, WebError> {
    session.check_csrf(&form.csrf_token)?;
    state.engine.lock().clear_notifications();
    Ok(Redirect::to("/notifications?msg=notifications_cleared"))
}

#[cfg(test)]
mod tests {
    use axum::http::{StatusCode, header};

    use crate::web::testutil::*;

    #[tokio::test]
    async fn the_page_marks_notifications_as_read() {
        let h = harness().await;
        h.state.engine.lock().unread_notifications = 3;
        let reply = send(&h.app, get("/notifications", Some(&h.cookie))).await;
        assert_eq!(reply.status, StatusCode::OK);
        assert_eq!(h.state.engine.lock().unread_notifications, 0);

        let form = format!("csrf_token={}", h.csrf);
        let reply = send(&h.app, post("/notifications/clear", &h.cookie, &form)).await;
        assert_eq!(reply.status, StatusCode::SEE_OTHER);
        assert_eq!(
            reply.headers[header::LOCATION],
            "/notifications?msg=notifications_cleared"
        );
    }
}

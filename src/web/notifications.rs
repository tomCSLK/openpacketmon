//! The notifications of the capture: traffic of favorites (hosts, programs, addresses).
//! Opening the page marks them as read.

use askama::Template;
use axum::Form;
use axum::extract::{Query, State};
use axum::response::{Redirect, Response};
use serde::Deserialize;

use crate::config::favorite::FavoriteItem;
use crate::networking::types::data_representation::DataRepr;
use crate::notifications::types::logged_notification::LoggedNotification;
use crate::web::auth::{CsrfForm, Session};
use crate::web::entries::{encode, host_sub, program_label, service_label};
use crate::web::i18n::{self, Lang};
use crate::web::pages::{Layout, flash_message, render};
use crate::web::{AppState, WebError, format};

pub struct NotificationRow {
    pub title: String,
    pub time: String,
    pub text: String,
    /// The connections of the favorite
    pub href: String,
}

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

fn row(t: Lang, n: &LoggedNotification) -> NotificationRow {
    let name = match &n.favorite {
        FavoriteItem::Host((host, _)) => {
            let sub = host_sub(host);
            if sub.is_empty() {
                host.domain.clone()
            } else {
                format!("{} ({sub})", host.domain)
            }
        }
        FavoriteItem::Service((service, _)) => service_label(t, service),
        FavoriteItem::Program((program, _)) => program_label(t, program),
        FavoriteItem::Address((ip, _)) => ip.to_string(),
    };
    NotificationRow {
        title: t.get("notification-favorite"),
        time: n.timestamp.clone(),
        text: t.get2(
            "notification-favorite-text",
            "name",
            name,
            "data",
            format::bytes(t, n.favorite.data_info().tot_data(DataRepr::bytes(false))),
        ),
        href: format!("/connections?f={}", encode(&n.favorite.connection_filter())),
    }
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
    let any_active = engine
        .conf
        .settings
        .notifications
        .favorite_notification
        .is_active;
    render(&NotificationsPage {
        t,
        layout: Layout::new(&engine, &session),
        flash: flash_message(t, query.msg.as_deref()),
        rows: engine
            .logged_notifications
            .notifications()
            .iter()
            .map(|n| row(t, n))
            .collect(),
        total: engine.logged_notifications.tot(),
        any_active,
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

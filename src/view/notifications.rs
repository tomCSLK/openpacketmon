//! The notifications of the capture: traffic of favorites (hosts, programs, addresses).

use crate::config::favorite::FavoriteItem;
use crate::engine::state::EngineState;
use crate::networking::types::data_representation::DataRepr;
use crate::notifications::types::logged_notification::LoggedNotification;
use crate::view::entries::{host_sub, program_label, service_label};
use crate::view::i18n::Lang;
use crate::view::{Jump, format};

pub struct NotificationRow {
    pub title: String,
    pub time: String,
    pub text: String,
    /// The connections of the favorite
    pub jump: Jump,
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
        jump: Jump::Connections {
            filter: n.favorite.connection_filter(),
        },
    }
}

/// The notifications kept
pub struct NotificationList {
    pub rows: Vec<NotificationRow>,
    /// All notifications so far, also those no longer kept
    pub total: usize,
    /// Notifications are switched on
    pub any_active: bool,
}

pub fn notification_list(t: Lang, engine: &EngineState) -> NotificationList {
    NotificationList {
        rows: engine
            .logged_notifications
            .notifications()
            .iter()
            .map(|n| row(t, n))
            .collect(),
        total: engine.logged_notifications.tot(),
        any_active: engine
            .conf
            .settings
            .notifications
            .favorite_notification
            .is_active,
    }
}

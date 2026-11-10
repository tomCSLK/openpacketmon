//! The notifications of the capture: traffic of favorites (hosts, programs, addresses).

use crate::config::favorite::FavoriteItem;
use crate::engine::state::EngineState;
use crate::networking::types::data_representation::DataRepr;
use crate::notifications::types::logged_notification::LoggedNotification;
use crate::view::entries::{program_label, service_label};
use crate::view::i18n::Lang;
use crate::view::{Jump, format};

/// A notification (a row of the table): time, event, the favorite and its data
pub struct NotificationRow {
    /// `14:48:02`, and with the day for the tooltip
    pub time: String,
    pub day_time: String,
    pub event: String,
    pub name: String,
    pub data: String,
    /// The connections of the favorite
    pub jump: Jump,
}

fn row(t: Lang, engine: &EngineState, n: &LoggedNotification) -> NotificationRow {
    let name = match &n.favorite {
        FavoriteItem::Host((host, _)) => host.domain.clone(),
        FavoriteItem::Service((service, _)) => service_label(t, service),
        FavoriteItem::Program((program, _)) => program_label(t, program),
        // an address with its name, if it has one (`203.0.113.44 (ssh.example.org)`)
        FavoriteItem::Address((ip, _)) => match engine.addresses_resolved.get(ip) {
            Some((name, _)) if !name.is_empty() && *name != ip.to_string() => {
                format!("{ip} ({name})")
            }
            _ => ip.to_string(),
        },
    };
    NotificationRow {
        time: n
            .timestamp
            .rsplit(' ')
            .next()
            .unwrap_or(&n.timestamp)
            .to_owned(),
        day_time: n.timestamp.clone(),
        event: t.get("notification-favorite"),
        name,
        data: format::bytes(t, n.favorite.data_info().tot_data(DataRepr::bytes(false))),
        jump: Jump::Connections {
            filter: n.favorite.connection_filter(),
        },
    }
}

/// The notifications kept
pub struct NotificationList {
    pub rows: Vec<NotificationRow>,
    /// Notifications are switched on
    pub any_active: bool,
}

pub fn notification_list(t: Lang, engine: &EngineState) -> NotificationList {
    NotificationList {
        rows: engine
            .logged_notifications
            .notifications()
            .iter()
            .map(|n| row(t, engine, n))
            .collect(),
        any_active: engine
            .conf
            .settings
            .notifications
            .favorite_notification
            .is_active,
    }
}

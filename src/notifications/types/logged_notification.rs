// Modified for openpacketmon (2026), see NOTICE. Based on `src/notifications/types/logged_notification.rs` of Sniffnet
// (https://github.com/GyulyVGC/sniffnet), Copyright (c) 2022 Giuliano Bellini, MIT OR Apache-2.0.

use crate::config::favorite::FavoriteItem;
use std::collections::VecDeque;

#[derive(Default)]
pub struct LoggedNotifications {
    /// Logged notifications during this capture session (max 30 kept)
    notifications: VecDeque<LoggedNotification>,
    /// Total number of notifications ever logged during this capture session
    total_notifications: usize,
}

impl LoggedNotifications {
    pub fn push(&mut self, notification: LoggedNotification) {
        self.total_notifications += 1;
        if self.notifications.len() >= 30 {
            self.notifications.pop_back();
        }
        self.notifications.push_front(notification);
    }

    pub fn tot(&self) -> usize {
        self.total_notifications
    }

    pub fn notifications(&self) -> &VecDeque<LoggedNotification> {
        &self.notifications
    }

    pub fn clear_notifications(&mut self) {
        self.notifications = VecDeque::new();
        self.total_notifications = 0;
    }

    #[cfg(test)]
    pub fn set_notifications(&mut self, notifications: VecDeque<LoggedNotification>) {
        self.notifications = notifications;
    }
}

/// A favorite (host, program or address) exchanged data
#[derive(Clone)]
pub struct LoggedNotification {
    pub(crate) favorite: FavoriteItem,
    pub(crate) timestamp: String,
}

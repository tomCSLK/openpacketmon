//! The notifications of the capture.

use std::time::Duration;

use eframe::egui::{Frame, Margin, RichText, ScrollArea, Ui};

use crate::gui::App;
use crate::gui::Page;
use crate::gui::load::Loader;
use crate::gui::pages::head;
use crate::gui::theme::palette;
use crate::gui::widgets::{self, Kind};
use crate::view::notifications::{NotificationList, notification_list};

#[derive(Default)]
pub struct State {
    list: Loader<usize, NotificationList>,
}

pub fn show(app: &mut App, ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    let t = app.t;
    let p = palette(&ctx);
    let key = app.live.capture_id;
    let state = &mut app.pages.notifications;
    state.list.poll();
    let engine = app.engine.clone();
    state
        .list
        .load(&ctx, &key, Some(Duration::from_secs(1)), move || {
            let mut state = engine.lock();
            // what this page shows has been seen
            state.mark_notifications_read();
            notification_list(t, &state)
        });
    let any = state.list.value().is_some_and(|l| !l.rows.is_empty());
    let (mut clear, mut settings) = (false, false);
    head(
        app,
        ui,
        &t.get("nav-notifications"),
        Some(&t.get("notifications-subtitle")),
        |ui| {
            if any {
                clear =
                    widgets::button(ui, t.get("notifications-clear"), Kind::Secondary).clicked();
            }
            settings =
                widgets::button(ui, t.get("notifications-settings"), Kind::Secondary).clicked();
        },
    );
    let mut jump = None;
    if let Some(list) = app.pages.notifications.list.value() {
        if !list.any_active {
            widgets::notice(ui, |ui| {
                ui.label(t.get("notifications-none-active"));
                settings |= widgets::link(ui, t.get("notifications-settings")).clicked();
            });
        }
        let shown = list.rows.len();
        widgets::flush_card(
            ui,
            &t.get1("notifications-count", "count", list.total),
            |ui| {
                if list.total > shown {
                    widgets::hint(ui, &t.get1("notifications-kept", "count", shown));
                }
            },
            |ui| {
                if list.rows.is_empty() {
                    widgets::empty(ui, &t.get("notifications-empty"));
                    return;
                }
                ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                    for n in &list.rows {
                        Frame::new()
                            .inner_margin(Margin::symmetric(14, 8))
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.spacing_mut().item_spacing.y = 2.0;
                                ui.horizontal(|ui| {
                                    ui.strong(&n.title);
                                    ui.label(RichText::new(&n.time).small().color(p.muted));
                                });
                                if widgets::link(ui, n.text.as_str()).clicked() {
                                    jump = Some(n.jump.clone());
                                }
                            });
                        ui.separator();
                    }
                });
            },
        );
    }
    if clear {
        app.engine.lock().clear_notifications();
        app.pages.notifications.list.reset();
    }
    if settings {
        app.go(Page::Settings);
    } else if let Some(jump) = jump {
        app.jump(jump);
    }
}

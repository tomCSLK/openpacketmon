//! The notifications of the capture (`pageNotifications` of the draft): a table of the traffic of
//! favorites, newest first.

use std::time::Duration;

use eframe::egui::{self, RichText, Sense, Ui};

use crate::gui::load::Loader;
use crate::gui::pages::head;
use crate::gui::theme::{self, palette};
use crate::gui::widgets::{self, Col, Kind, TableSpec, Width};
use crate::gui::{App, icons};
use crate::gui::{Page, SettingsPanel};
use crate::notifications::types::logged_notification::KEPT;
use crate::view::notifications::{NotificationList, NotificationRow, notification_list};

/// A row of the table (`--row-h`)
const ROW_H: f32 = 34.0;
/// The star before the event (`#c98a00` of the draft)
const STAR: egui::Color32 = egui::Color32::from_rgb(0xc9, 0x8a, 0x00);

#[derive(Default)]
pub struct State {
    list: Loader<usize, NotificationList>,
}

pub fn show(app: &mut App, ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    let t = app.t;
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
        ui,
        &t.get("nav-notifications"),
        Some(&t.get("notifications-subtitle")),
        |ui| {
            // right to left: the last button first
            clear = ui
                .add_enabled_ui(any, |ui| {
                    widgets::icon_text_button(
                        ui,
                        Some(icons::TRASH),
                        &t.get("notifications-clear"),
                        Kind::Secondary,
                        false,
                    )
                })
                .inner
                .clicked();
            settings = widgets::icon_text_button(
                ui,
                Some(icons::SETTINGS),
                &t.get("notifications-settings"),
                Kind::Ghost,
                false,
            )
            .clicked();
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
        widgets::flush_card(
            ui,
            &t.get1("notifications-count", "count", list.rows.len()),
            |ui| widgets::card_note(ui, &t.get1("notifications-kept", "count", KEPT)),
            |ui| {
                if list.rows.is_empty() {
                    widgets::empty_state(
                        ui,
                        Some(icons::BELL_XL),
                        &t.get("notifications-empty"),
                        &t.get("notifications-empty-text"),
                        None,
                    );
                    return;
                }
                jump = table(ui, t, &list.rows);
            },
        );
    }
    if clear && any {
        app.engine.lock().clear_notifications();
        app.pages.notifications.list.reset();
    }
    if settings {
        app.go(Page::Settings(SettingsPanel::Notifications));
    } else if let Some(jump) = jump {
        app.jump(jump);
    }
}

/// The table: time, event, the favorite as a link to its connections, its data
fn table(
    ui: &mut Ui,
    t: crate::view::i18n::Lang,
    rows: &[NotificationRow],
) -> Option<crate::view::Jump> {
    let p = palette(ui.ctx());
    let cols = vec![
        Col::new(t.get("notifications-time"), Width::Px(88.0)),
        Col::new(t.get("notifications-event"), Width::Share(0.3685)),
        Col::new(t.get("notifications-favorite"), Width::Rest(160.0)),
        Col::num(t.get("notifications-data"), Width::Px(120.0)),
    ];
    let mut jump = None;
    widgets::table(
        ui,
        &TableSpec::new("notifications", cols, ROW_H, rows.len()),
        |ui, r, c| {
            let row = &rows[r];
            match c {
                0 => {
                    ui.label(widgets::mono(&row.time))
                        .on_hover_text(&row.day_time);
                }
                1 => {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), Sense::hover());
                    widgets::paint_icon(ui, icons::STAR, rect, STAR);
                    ui.label(RichText::new(&row.event).color(p.text));
                }
                2 => {
                    let name = RichText::new(&row.name)
                        .size(theme::FS_BASE)
                        .color(p.accent);
                    if widgets::link_text(ui, name).clicked() {
                        jump = Some(row.jump.clone());
                    }
                }
                _ => {
                    ui.label(widgets::mono(&row.data));
                }
            }
        },
    );
    jump
}

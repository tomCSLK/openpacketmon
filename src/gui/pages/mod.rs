//! The pages of the window, each with what it remembers between frames.

pub mod capture;
mod connections;
mod entries;
mod licenses;
mod notifications;
mod overview;
mod packet;
pub mod packets;
mod settings;
mod statistics;
mod stream;

use eframe::egui::Ui;

use crate::gui::{App, Page, widgets};

#[derive(Default)]
pub struct Pages {
    pub capture: capture::State,
    pub packets: packets::State,
    pub packet: packet::State,
    pub stream: stream::State,
    pub overview: overview::State,
    pub connections: connections::State,
    pub entries: entries::State,
    pub statistics: statistics::State,
    pub notifications: notifications::State,
    pub settings: settings::State,
}

/// Draws the page in view
pub fn show(app: &mut App, ui: &mut Ui) {
    match app.page.clone() {
        Page::Capture => capture::show(app, ui),
        Page::Packets {
            filter,
            connections,
            goto,
        } => packets::show(app, ui, &filter, &connections, goto),
        Page::Packet(number) => packet::show(app, ui, number),
        Page::Stream(number) => stream::show(app, ui, number),
        Page::Overview => overview::show(app, ui),
        Page::Connections { filter } => connections::show(app, ui, &filter),
        Page::Connection(key) => connections::show_connection(app, ui, &key),
        Page::Statistics {
            filter,
            connections,
        } => statistics::show(app, ui, &filter, &connections),
        Page::Entries(kind) => entries::show(app, ui, kind),
        Page::Notifications => notifications::show(app, ui),
        Page::Settings => settings::show(app, ui),
        Page::Licenses => licenses::show(app, ui),
    }
}

/// The head of a page: the way back, title, subtitle and actions
pub fn head(
    app: &mut App,
    ui: &mut Ui,
    title: &str,
    subtitle: Option<&str>,
    actions: impl FnOnce(&mut Ui),
) {
    ui.horizontal(|ui| {
        app.back_arrow(ui);
        ui.vertical(|ui| widgets::page_head(ui, title, subtitle, actions));
    });
}

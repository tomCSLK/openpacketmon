//! The pages of the window, each with what it remembers between frames.

pub mod calls;
pub mod capture;
mod connections;
mod entries;
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
    pub calls: calls::State,
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
    if !matches!(app.page, Page::Call(_)) {
        app.pages.calls.player.stop();
    }
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
        Page::Settings(panel) => settings::show(app, ui, panel),
        Page::Calls => calls::show_list(app, ui),
        Page::Call(key) => calls::show_call(app, ui, key),
    }
}

/// The head of a page of the navigation (`.pagehead`): title, the text below it and actions
pub fn head(ui: &mut Ui, title: &str, subtitle: Option<&str>, actions: impl FnOnce(&mut Ui)) {
    head_rich(
        ui,
        title,
        |ui| {
            if let Some(subtitle) = subtitle {
                widgets::subtitle(ui, subtitle);
            }
        },
        actions,
    );
}

/// [`head`] with a text below the title that `sub` draws
pub fn head_rich(
    ui: &mut Ui,
    title: &str,
    sub: impl FnOnce(&mut Ui),
    actions: impl FnOnce(&mut Ui),
) {
    widgets::page_head(ui, &[], None, |ui| widgets::title(ui, title), sub, actions);
}

/// The head of a detail page: the crumbs of the pages above it and this page, the way back, the
/// title, the text below it and actions. The way back leads to the page before, or to the last
/// crumb.
pub fn detail_head(
    app: &mut App,
    ui: &mut Ui,
    crumbs: &[(String, Page)],
    here: &str,
    title: impl FnOnce(&mut Ui),
    sub: impl FnOnce(&mut Ui),
    actions: impl FnOnce(&mut Ui),
) {
    let mut names: Vec<String> = crumbs.iter().map(|(name, _)| name.clone()).collect();
    names.push(here.to_owned());
    let hint = app.t.get("back-hint");
    let clicks = widgets::page_head(ui, &names, Some(&hint), title, sub, actions);
    if let Some(i) = clicks.crumb {
        // the crumb of the page before goes back to it, as the way back does
        if app.history.last() == Some(&crumbs[i].1) {
            app.back();
        } else {
            app.go(crumbs[i].1.clone());
        }
    } else if clicks.back {
        if app.can_go_back() {
            app.back();
        } else if let Some((_, page)) = crumbs.last() {
            app.go(page.clone());
        }
    }
}

//! Hosts, services and programs: the list with traffic bars and favorites.

use std::time::Duration;

use eframe::egui::{self, Align, Layout, RichText, TextEdit, Ui};
use egui_extras::{Column, TableBuilder};

use crate::config::favorite::{Favorite, FavoriteKey};
use crate::gui::load::Loader;
use crate::gui::pages::head;
use crate::gui::theme::palette;
use crate::gui::widgets;
use crate::gui::{App, Page, icons};
use crate::networking::types::data_representation::DataUnit;
use crate::view::Jump;
use crate::view::entries::{EntryList, EntryRow, entry_list};
use crate::view::i18n::Lang;

/// What was clicked in a list
pub enum Action {
    Open(Jump),
    Favorite(FavoriteKey, bool),
}

impl Action {
    pub fn run(self, app: &mut App) {
        match self {
            Self::Open(jump) => app.jump(jump),
            Self::Favorite(key, add) => {
                app.engine.lock().add_or_remove_favorite(&key, add);
                app.engine.store_conf();
                app.pages.entries.list.reset();
                app.pages.overview.reset();
            }
        }
    }
}

pub const ROW_H: f32 = 40.0;

/// The rows; `compact` leaves out incoming and outgoing for a narrow card
pub fn table(
    ui: &mut Ui,
    id: &str,
    t: Lang,
    rows: &[EntryRow],
    compact: bool,
    scroll: bool,
) -> Option<Action> {
    let p = palette(ui.ctx());
    let stars = rows.iter().any(EntryRow::favorable);
    let mut action = None;
    ui.spacing_mut().item_spacing = egui::vec2(8.0, 0.0);
    let mut builder = TableBuilder::new(ui)
        .id_salt(id)
        .vscroll(scroll)
        .auto_shrink(false)
        .animate_scrolling(false)
        .max_scroll_height(f32::INFINITY)
        .min_scrolled_height(0.0)
        .cell_layout(Layout::left_to_right(Align::Center));
    if stars {
        builder = builder.column(Column::exact(28.0));
    }
    builder = builder.column(Column::remainder().at_least(90.0).clip(true));
    builder = builder.column(Column::exact(if compact { 64.0 } else { 150.0 }));
    if !compact {
        builder = builder
            .column(Column::exact(96.0))
            .column(Column::exact(96.0));
    }
    builder = builder.column(Column::exact(if compact { 82.0 } else { 96.0 }));
    builder
        .header(26.0, |mut header| {
            let mut head = |text: String, right: bool| {
                header.col(|ui| {
                    let text = RichText::new(text).small().color(p.muted);
                    if right {
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| ui.label(text));
                    } else {
                        ui.label(text);
                    }
                });
            };
            if stars {
                head(String::new(), false);
            }
            head(t.get("entry-name"), false);
            head(t.get("entry-traffic"), false);
            if !compact {
                head(t.get("entry-in"), true);
                head(t.get("entry-out"), true);
            }
            head(t.get("entry-total"), true);
        })
        .body(|body| {
            body.rows(ROW_H, rows.len(), |mut row| {
                let r = &rows[row.index()];
                if stars {
                    row.col(|ui| {
                        let Some(key) = &r.key else {
                            return;
                        };
                        let (icon, color, text) = if r.favorite {
                            (icons::STAR_ON, p.warn, t.get("favorite-remove"))
                        } else {
                            (icons::STAR, p.faint, t.get("favorite-add"))
                        };
                        if widgets::icon_button(ui, icon, color, &text).clicked() {
                            action = Some(Action::Favorite(key.clone(), !r.favorite));
                        }
                    });
                }
                row.col(|ui| {
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        ui.spacing_mut().interact_size.y = 18.0;
                        ui.horizontal(|ui| {
                            let link = egui::Link::new(RichText::new(&r.label).color(p.accent));
                            if ui.add(link).clicked() {
                                action = Some(Action::Open(r.jump.clone()));
                            }
                            for tag in &r.tags {
                                widgets::tag(ui, tag, p.neutral_soft, p.muted);
                            }
                        });
                        if !r.sub.is_empty() {
                            ui.add(
                                egui::Label::new(RichText::new(&r.sub).small().color(p.muted))
                                    .truncate(),
                            );
                        }
                    });
                });
                row.col(|ui| {
                    let width = ui.available_width();
                    #[allow(clippy::cast_precision_loss)] // percent
                    widgets::share_bar(
                        ui,
                        width,
                        r.bar_in as f32,
                        r.bar_out as f32,
                        p.accent,
                        p.ok,
                    );
                });
                let number = |ui: &mut Ui, text: &str, strong: bool| {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let text = RichText::new(text).monospace();
                        ui.label(if strong { text.strong() } else { text });
                    });
                };
                if !compact {
                    row.col(|ui| number(ui, &r.incoming, false));
                    row.col(|ui| number(ui, &r.outgoing, false));
                }
                row.col(|ui| number(ui, &r.total, true));
            });
        });
    action
}

type Key = (Favorite, bool, bool, String, usize, DataUnit);

#[derive(Default)]
pub struct State {
    list: Loader<Key, EntryList>,
    text: String,
    ascending: bool,
    only_favorites: bool,
}

pub fn unit_labels(t: Lang) -> [String; 3] {
    [
        t.get("unit-bytes"),
        t.get("unit-bits"),
        t.get("unit-packets"),
    ]
}

pub fn unit_index(unit: DataUnit) -> usize {
    match unit {
        DataUnit::Bytes => 0,
        DataUnit::Bits => 1,
        DataUnit::Packets => 2,
    }
}

/// Changes the unit of the numbers
pub fn set_unit(app: &mut App, index: usize) {
    let unit = [DataUnit::Bytes, DataUnit::Bits, DataUnit::Packets][index];
    app.engine.lock().conf.data_repr.data_unit = unit;
    app.engine.store_conf();
    app.live.unit = unit;
}

pub fn show(app: &mut App, ui: &mut Ui, kind: Favorite) {
    let ctx = ui.ctx().clone();
    let t = app.t;
    let state = &mut app.pages.entries;
    let key = (
        kind,
        state.ascending,
        state.only_favorites,
        state.text.trim().to_owned(),
        app.live.capture_id,
        app.live.unit,
    );
    state.list.poll();
    let engine = app.engine.clone();
    let job_key = key.clone();
    let every = app.live.running.then_some(Duration::from_secs(1));
    state.list.load(&ctx, &key, every, move || {
        entry_list(
            t,
            &engine.lock(),
            job_key.0,
            job_key.1,
            job_key.2,
            &job_key.3,
        )
    });
    let (title, subtitle) = app
        .pages
        .entries
        .list
        .value()
        .map_or_else(Default::default, |l| (l.title.clone(), l.subtitle.clone()));
    let mut unit = None;
    let index = unit_index(app.live.unit);
    head(app, ui, &title, Some(&subtitle), |ui| {
        unit = widgets::segmented(ui, &unit_labels(t), index);
    });
    let mut go = None;
    if !app.live.running {
        widgets::notice(ui, |ui| {
            ui.label(t.get("no-capture"));
            if widgets::link(ui, t.get("no-capture-start")).clicked() {
                go = Some(Page::Capture);
            }
        });
    }
    if kind == Favorite::Host && app.live.status.geo_missing {
        widgets::notice(ui, |ui| {
            ui.label(t.get("geo-missing"));
            if widgets::link(ui, t.get("geo-missing-link")).clicked() {
                go = Some(Page::Settings);
            }
        });
    }
    let state = &mut app.pages.entries;
    let mut action = None;
    if let Some(list) = state.list.value() {
        if list.unsupported {
            widgets::notice_text(ui, &t.get("programs-unsupported"));
        }
        let shown = list.rows.len();
        let total = list.total;
        let (text, ascending, only) = (
            &mut state.text,
            &mut state.ascending,
            &mut state.only_favorites,
        );
        widgets::flush_card(
            ui,
            &t.get1("entries-count", "count", total),
            |ui| {
                if kind != Favorite::Service {
                    ui.checkbox(only, t.get("only-favorites"));
                }
                let labels = [t.get("sort-desc"), t.get("sort-asc")];
                let mut at = usize::from(*ascending);
                egui::ComboBox::from_id_salt("entries-sort")
                    .selected_text(labels[at].clone())
                    .show_index(ui, &mut at, labels.len(), |i| labels[i].clone());
                *ascending = at == 1;
                ui.add(
                    TextEdit::singleline(text)
                        .hint_text(t.get("entries-filter"))
                        .desired_width(180.0),
                );
            },
            |ui| {
                if list.rows.is_empty() {
                    widgets::empty(ui, &t.get("entries-empty"));
                    return;
                }
                if total > shown {
                    ui.horizontal(|ui| {
                        ui.add_space(14.0);
                        widgets::hint(
                            ui,
                            &t.get2("entries-truncated", "shown", shown, "total", total),
                        );
                    });
                }
                action = widgets::table_area(ui, None, |ui| {
                    table(ui, "entries", t, &list.rows, false, true)
                });
            },
        );
    }
    if let Some(index) = unit {
        set_unit(app, index);
    }
    if let Some(action) = action {
        action.run(app);
    } else if let Some(page) = go {
        app.go(page);
    }
}

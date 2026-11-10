//! Hosts, services and programs (`pageEntries` of the draft): the list with search, order, unit,
//! favorites and traffic bars.

use std::time::Duration;

use eframe::egui::{self, Align, Layout, Rect, RichText, Ui};

use crate::config::favorite::{Favorite, FavoriteKey};
use crate::gui::load::Loader;
use crate::gui::pages::head;
use crate::gui::theme::{self, palette};
use crate::gui::widgets::{self, Col, TableSpec, Width};
use crate::gui::{App, Page, SettingsPanel};
use crate::networking::types::data_representation::DataUnit;
use crate::report::types::sort_type::SortType;
use crate::view::Jump;
use crate::view::entries::{EntryList, EntryRow, entry_list, kind_code};
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

/// A row with a second line (`--row-h` and `.sub`)
const ROW_H: f32 = 45.0;
/// The orders of the list: largest, smallest, newest first
const SORTS: [SortType; 3] = [SortType::Descending, SortType::Ascending, SortType::Neutral];

type Key = (Favorite, usize, bool, String, usize, DataUnit);

#[derive(Default)]
pub struct State {
    pub list: Loader<Key, EntryList>,
    text: String,
    sort: usize,
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
    let code = kind_code(kind);
    let title = t.get(&format!("nav-{code}s"));
    if !app.live.running {
        head(ui, &title, None, |_| {});
        if widgets::no_capture(ui, t) {
            app.go(Page::Capture);
        }
        return;
    }
    let state = &mut app.pages.entries;
    let key = (
        kind,
        state.sort,
        state.only_favorites && kind != Favorite::Service,
        state.text.trim().to_owned(),
        app.live.capture_id,
        app.live.unit,
    );
    state.list.poll();
    let engine = app.engine.clone();
    let job_key = key.clone();
    state
        .list
        .load(&ctx, &key, Some(Duration::from_secs(1)), move || {
            entry_list(
                t,
                &engine.lock(),
                job_key.0,
                SORTS[job_key.1],
                job_key.2,
                &job_key.3,
            )
        });
    head(
        ui,
        &title,
        Some(&t.get(&format!("{code}s-subtitle"))),
        |_| {},
    );
    let mut go = None;
    if kind == Favorite::Host && app.live.status.geo_missing {
        widgets::notice(ui, |ui| {
            ui.label(t.get("geo-missing"));
            if widgets::link(ui, t.get("geo-missing-link")).clicked() {
                go = Some(Page::Settings(SettingsPanel::Geo));
            }
        });
    }
    let unit_at = unit_index(app.live.unit);
    let state = &mut app.pages.entries;
    let mut unit = None;
    let mut new_sort = None;
    let mut action = None;
    let list = state.list.value_of(&key).or(state.list.value());
    widgets::bare_card(ui, |ui| {
        widgets::toolbar(ui, |ui| {
            let id = egui::Id::NULL.with(("entries-search", code));
            widgets::input(
                ui,
                &mut state.text,
                id,
                egui::vec2(240.0, 28.0),
                false,
                &t.get("entries-filter"),
            );
            let labels = [t.get("sort-desc"), t.get("sort-asc"), t.get("sort-recent")];
            new_sort = widgets::select(ui, &labels, state.sort, true);
            unit = widgets::segmented_sized(ui, &unit_labels(t), unit_at, true);
            if kind != Favorite::Service {
                widgets::switch(
                    ui,
                    &mut state.only_favorites,
                    &t.get("only-favorites"),
                    false,
                );
            }
            if let Some(list) = list {
                widgets::toolbar_sum(ui, &t.get1("entries-count", "count", list.total));
            }
        });
        let Some(list) = list else {
            return;
        };
        action = table(ui, t, &list.rows, kind != Favorite::Service);
        if list.rows.is_empty() {
            widgets::empty(ui, &t.get("entries-none"));
        }
    });
    if let Some(list) = list
        && list.total > list.rows.len()
    {
        widgets::hint(
            ui,
            &t.get2(
                "entries-truncated",
                "shown",
                list.rows.len(),
                "total",
                list.total,
            ),
        );
    }
    // below the card (`p.muted.small`)
    let note = match kind {
        Favorite::Program => Some(t.get("programs-unsupported")),
        Favorite::Service => Some(t.get("services-note")),
        Favorite::Host => None,
    };
    if let Some(note) = note {
        let p = palette(ui.ctx());
        ui.label(widgets::small(note).color(p.text_3));
    }
    if let Some(sort) = new_sort {
        app.pages.entries.sort = sort;
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

/// The table of the list (`#etbl`): the star (not for services), the name with its country and
/// second line, the traffic as a bar, incoming, outgoing, total and the number of connections
fn table(ui: &mut Ui, t: Lang, rows: &[EntryRow], stars: bool) -> Option<Action> {
    let p = palette(ui.ctx());
    let narrow = ui.ctx().content_rect().width() < 900.0;
    let mut cols = Vec::new();
    if stars {
        cols.push(Col::new("", Width::Px(48.0)));
    }
    cols.extend([
        Col::new(t.get("entry-name"), Width::Rest(160.0)),
        Col::new(t.get("entry-traffic"), Width::Share(0.22)),
        Col::num(t.get("entry-in"), Width::Px(130.0)),
        Col::num(t.get("entry-out"), Width::Px(135.0)),
        Col::num(t.get("entry-total"), Width::Px(120.0)),
    ]);
    if !narrow {
        cols.push(Col::num(t.get("entry-connections"), Width::Px(162.0)));
    }
    let mut spec = TableSpec::new("entries", cols, ROW_H, rows.len());
    spec.click = true;
    let mut action = None;
    let mut opened = None;
    let out = widgets::table(ui, &spec, |ui, r, c| {
        let row = &rows[r];
        let c = if stars { c } else { c + 1 };
        match c {
            0 => {
                let Some(key) = &row.key else {
                    return;
                };
                let text = if row.favorite {
                    t.get("favorite-remove")
                } else {
                    t.get("favorite-add")
                };
                if widgets::star(ui, row.favorite, &text).clicked() {
                    action = Some(Action::Favorite(key.clone(), !row.favorite));
                }
            }
            1 => {
                if name_cell(ui, row) {
                    opened = Some(r);
                }
            }
            2 => {
                let width = ui.available_width();
                widgets::tbar(
                    ui,
                    width,
                    8.0,
                    (row.bar_in + row.bar_out) / 100.0,
                    row.bar_in,
                    row.bar_out,
                );
            }
            3 => {
                ui.label(widgets::mono(&row.incoming));
            }
            4 => {
                ui.label(widgets::mono(&row.outgoing));
            }
            5 => {
                ui.label(
                    RichText::new(&row.total)
                        .font(theme::mono_semibold_font(theme::FS_SM))
                        .color(p.text),
                );
            }
            _ => {
                ui.label(widgets::mono(&row.connections));
            }
        }
    });
    if action.is_some() {
        return action;
    }
    out.clicked
        .or(opened)
        .map(|r| Action::Open(rows[r].jump.clone()))
}

/// The name (`td.name`): a link with the country after it, the second line below
fn name_cell(ui: &mut Ui, row: &EntryRow) -> bool {
    let p = palette(ui.ctx());
    let rect = ui.max_rect();
    let lines = if row.sub.is_empty() { 1.0 } else { 2.0 };
    let block = widgets::LH_BASE + (lines - 1.0) * theme::FS_XS * 1.3;
    let top = rect.center().y - block / 2.0;
    let mut first = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(Rect::from_min_size(
                egui::pos2(rect.left(), top),
                egui::vec2(rect.width(), widgets::LH_BASE),
            ))
            .layout(Layout::left_to_right(Align::Center)),
    );
    first.spacing_mut().item_spacing.x = 4.0;
    let name = RichText::new(&row.label)
        .font(theme::medium_font(theme::FS_BASE))
        .color(p.accent);
    let clicked = widgets::link_text(&mut first, name).clicked();
    if !row.cc.is_empty() {
        widgets::tag_cc(&mut first, &row.cc);
    }
    if !row.sub.is_empty() {
        let mut second = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_size(
            egui::pos2(rect.left(), top + widgets::LH_BASE),
            egui::vec2(rect.width().min(360.0), theme::FS_XS * 1.3),
        )));
        second.add(
            egui::Label::new(
                RichText::new(&row.sub)
                    .size(theme::FS_XS)
                    .color(p.text_3)
                    .line_height(Some(theme::FS_XS * 1.3)),
            )
            .selectable(false)
            .truncate(),
        );
    }
    clicked
}

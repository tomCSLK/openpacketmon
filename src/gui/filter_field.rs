//! The filter fields: a line of text with suggestions while typing, as on the browser pages.
//! Enter applies the filter; only a line chosen with the arrow keys is taken into the field.

use std::sync::Arc;

use eframe::egui::text::{CCursor, CCursorRange};
use eframe::egui::{
    self, Area, CornerRadius, Event, Frame, Id, Key, Margin, Order, RichText, ScrollArea, Sense,
    Stroke, TextEdit, Ui,
};

use crate::engine::Engine;
use crate::gui::load::Loader;
use crate::gui::theme::palette;
use crate::view::i18n::Lang;
use crate::view::packets::filter_problem;
use crate::view::suggest::{Catalog, Expect, Line, catalog, lines, position, value_rows};

/// A filter field with what it remembers
#[derive(Default)]
pub struct FilterField {
    pub text: String,
    catalog: Option<(Lang, Arc<Catalog>)>,
    /// Lines of the list, `None` while the list is closed
    list: Option<Vec<Line>>,
    /// What a value looks like, above the lines
    note: String,
    /// The line chosen with the arrow keys
    chosen: Option<usize>,
    /// Where the line taken goes, in bytes
    start: usize,
    /// The cursor is in this text at this place
    seen: Option<(String, usize)>,
    /// Values of the capture for the connection filter
    values: Loader<(String, String), Vec<Line>>,
    pointer_on_list: bool,
    /// The list was closed with Escape and stays closed until the text changes
    dismissed: bool,
    /// What is wrong with the text
    problem: Option<(String, Option<String>)>,
}

/// What happened to the field in this frame
#[derive(Default)]
pub struct Outcome {
    /// Enter was pressed
    pub apply: bool,
}

impl FilterField {
    /// A new text from outside, for example from a link
    pub fn set(&mut self, text: &str) {
        text.clone_into(&mut self.text);
        self.list = None;
        self.seen = None;
        self.problem = None;
    }

    /// What is wrong with the text as it is typed
    pub fn problem(&mut self, t: Lang, connections: bool) -> Option<&str> {
        let current = self.text.trim();
        if self
            .problem
            .as_ref()
            .is_none_or(|(text, _)| text != current)
        {
            let found = if current.is_empty() {
                None
            } else {
                filter_problem(t, current, connections)
            };
            self.problem = Some((current.to_owned(), found));
        }
        self.problem.as_ref().and_then(|(_, p)| p.as_deref())
    }

    fn catalog(&mut self, t: Lang, connections: bool) -> Arc<Catalog> {
        match &self.catalog {
            Some((lang, c)) if *lang == t => c.clone(),
            _ => {
                let c = Arc::new(catalog(t, connections));
                self.catalog = Some((t, c.clone()));
                c
            }
        }
    }

    /// The lines for the text before the cursor
    fn update(&mut self, ui: &Ui, t: Lang, engine: &Engine, connections: bool, cursor: usize) {
        let cat = self.catalog(t, connections);
        let before = byte_at(&self.text, cursor);
        let place = position(&self.text[..before], &cat);
        let mut shown = lines(&place, &cat);
        self.start = place.start;
        self.chosen = None;
        // a name typed to its end needs no line of its own
        if shown.len() == 1 && shown[0].insert == place.partial {
            shown.clear();
        }
        let wants_value = matches!(place.expect, Expect::Value | Expect::Set);
        self.note = place
            .field
            .filter(|_| wants_value)
            .and_then(|f| f.k)
            .and_then(|k| cat.kinds.get(k))
            .map(|k| k.hint.clone())
            .unwrap_or_default();
        if wants_value
            && connections
            && let Some(field) = place.field
        {
            let key = (
                field.n.to_owned(),
                place.partial.trim_start_matches('"').to_owned(),
            );
            let engine = engine.clone();
            let job_key = key.clone();
            self.values.load(ui.ctx(), &key, None, move || {
                value_rows(&engine.lock(), &job_key.0, &job_key.1)
                    .into_iter()
                    .map(|v| Line {
                        name: v.text,
                        insert: v.insert,
                        hint: Some(v.count.to_string()),
                    })
                    .collect()
            });
        }
        self.list = Some(shown);
    }

    fn take(&mut self, line: &Line, cursor: usize) -> usize {
        let caret = byte_at(&self.text, cursor);
        let head = format!("{}{} ", &self.text[..self.start.min(caret)], line.insert);
        let tail = self.text[caret..].trim_start().to_owned();
        let at = head.chars().count();
        self.text = format!("{head}{tail}");
        self.list = None;
        self.seen = None;
        at
    }

    /// Draws the field; `state` colors its frame: valid or not, `None` when it is empty
    #[allow(clippy::too_many_arguments)] // what one filter field differs in
    pub fn show(
        &mut self,
        ui: &mut Ui,
        t: Lang,
        engine: &Engine,
        connections: bool,
        placeholder: &str,
        valid: Option<bool>,
        width: f32,
    ) -> Outcome {
        let p = palette(ui.ctx());
        let mut outcome = Outcome::default();
        let id = ui.next_auto_id().with("filter-field");
        let focused = ui.memory(|m| m.has_focus(id));
        let shown_count =
            self.list.as_ref().map_or(0, Vec::len) + self.values.value().map_or(0, Vec::len);
        let mut take_chosen = false;
        if focused && self.list.is_some() {
            ui.input_mut(|i| {
                if i.consume_key(egui::Modifiers::NONE, Key::ArrowDown) && shown_count > 0 {
                    self.chosen = Some(self.chosen.map_or(0, |c| (c + 1) % shown_count));
                }
                if i.consume_key(egui::Modifiers::NONE, Key::ArrowUp) && shown_count > 0 {
                    self.chosen = Some(
                        self.chosen
                            .map_or(shown_count - 1, |c| (c + shown_count - 1) % shown_count),
                    );
                }
                if i.consume_key(egui::Modifiers::NONE, Key::Escape) {
                    self.list = None;
                    self.dismissed = true;
                }
                if self.chosen.is_some() && i.consume_key(egui::Modifiers::NONE, Key::Enter) {
                    take_chosen = true;
                }
                let partial_or_chosen = self.chosen.is_some() || shown_count > 0;
                if partial_or_chosen && i.consume_key(egui::Modifiers::NONE, Key::Tab) {
                    self.chosen.get_or_insert(0);
                    take_chosen = true;
                }
            });
        }

        let mut output = TextEdit::singleline(&mut self.text)
            .id(id)
            .hint_text(placeholder)
            .font(egui::TextStyle::Monospace)
            .margin(Margin::symmetric(8, 7))
            .desired_width(width)
            .show(ui);
        let response = output.response.response.clone();
        if let Some(valid) = valid {
            let color = if valid { p.ok } else { p.down };
            ui.painter().rect_stroke(
                response.rect,
                CornerRadius::same(8),
                Stroke::new(1.0, color),
                egui::StrokeKind::Inside,
            );
        }
        let enter = ui.input(|i| {
            i.events.iter().any(|e| {
                matches!(
                    e,
                    Event::Key {
                        key: Key::Enter,
                        pressed: true,
                        ..
                    }
                )
            })
        });
        if response.lost_focus() && enter && !take_chosen {
            outcome.apply = true;
            self.list = None;
        }

        let cursor = output
            .cursor_range
            .map_or_else(|| self.text.chars().count(), |r| r.primary.index.into());
        let mut moved_to = None;
        if take_chosen {
            let line = self.chosen.and_then(|c| self.line(c));
            if let Some(line) = line {
                moved_to = Some(self.take(&line, cursor));
            }
        }
        let has_focus = response.has_focus();
        if has_focus && moved_to.is_none() {
            let now = (self.text.clone(), cursor);
            if self.seen.as_ref() != Some(&now) {
                if self.seen.as_ref().is_some_and(|(text, _)| *text != now.0) {
                    self.dismissed = false;
                }
                self.seen = Some(now);
                if !self.dismissed {
                    self.update(ui, t, engine, connections, cursor);
                }
            }
        }
        self.values.poll();

        if has_focus || self.pointer_on_list {
            if let Some(clicked) = self.draw_list(ui, id, response.rect) {
                let line = self.line(clicked);
                if let Some(line) = line {
                    moved_to = Some(self.take(&line, cursor));
                }
            }
        } else {
            self.list = None;
            self.seen = None;
            self.chosen = None;
        }
        if let Some(at) = moved_to {
            output
                .state
                .cursor
                .set_char_range(Some(CCursorRange::one(CCursor::new(at))));
            output.state.store(ui.ctx(), id);
            response.request_focus();
            ui.ctx().request_repaint();
        }
        outcome
    }

    /// The line at this place of the list: first the values of the capture
    fn line(&self, at: usize) -> Option<Line> {
        let values = self.values.value().map_or(&[][..], Vec::as_slice);
        values
            .iter()
            .chain(self.list.iter().flatten())
            .nth(at)
            .cloned()
    }

    /// Draws the list below the field; returns the line clicked
    fn draw_list(&mut self, ui: &mut Ui, id: Id, field: egui::Rect) -> Option<usize> {
        let p = palette(ui.ctx());
        let list = self.list.as_ref()?;
        let values = if self.note.is_empty() {
            &[][..]
        } else {
            self.values.value().map_or(&[][..], Vec::as_slice)
        };
        if self.note.is_empty() && list.is_empty() {
            self.pointer_on_list = false;
            return None;
        }
        let mut clicked = None;
        let chosen = self.chosen;
        let note = self.note.clone();
        let area = Area::new(id.with("list"))
            .order(Order::Foreground)
            .fixed_pos(field.left_bottom() + egui::vec2(0.0, 4.0))
            .show(ui.ctx(), |ui| {
                Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_min_width(field.width().min(520.0));
                    ui.set_max_width(field.width().max(420.0));
                    ui.spacing_mut().item_spacing.y = 0.0;
                    if !note.is_empty() {
                        ui.label(RichText::new(&note).small().color(p.muted));
                        ui.add_space(4.0);
                    }
                    ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                        for (i, line) in values.iter().chain(list.iter()).enumerate() {
                            if suggestion(ui, line, chosen == Some(i)).clicked() {
                                clicked = Some(i);
                            }
                        }
                    });
                });
            });
        self.pointer_on_list = area.response.contains_pointer();
        clicked
    }
}

fn suggestion(ui: &mut Ui, line: &Line, chosen: bool) -> egui::Response {
    let p = palette(ui.ctx());
    let size = egui::vec2(ui.available_width(), 26.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    response
        .widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, chosen, &line.name));
    if chosen || response.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(6), p.accent_soft);
    }
    let name = ui.painter().layout_no_wrap(
        line.name.clone(),
        egui::TextStyle::Monospace.resolve(ui.style()),
        p.text,
    );
    let name_width = name.size().x;
    ui.painter().galley(
        rect.left_center() + egui::vec2(6.0, -name.size().y / 2.0),
        name,
        p.text,
    );
    if let Some(hint) = &line.hint {
        let left = rect.left() + 18.0 + name_width;
        let room = rect.right() - left - 6.0;
        if room > 20.0 {
            let mut job = egui::text::LayoutJob::simple_singleline(
                hint.clone(),
                egui::TextStyle::Small.resolve(ui.style()),
                p.muted,
            );
            job.wrap = egui::text::TextWrapping::truncate_at_width(room);
            let galley = ui.painter().layout_job(job);
            let y = rect.center().y - galley.size().y / 2.0;
            ui.painter().galley(egui::pos2(left, y), galley, p.muted);
        }
    }
    response
}

/// The help of a filter field; returns the example clicked
pub fn filter_help(
    ui: &mut Ui,
    title: &str,
    text: &str,
    examples: &[&'static str],
    lines: &[(String, String)],
    more: impl FnOnce(&mut Ui),
) -> Option<String> {
    let p = palette(ui.ctx());
    let mut clicked = None;
    egui::CollapsingHeader::new(RichText::new(title).small().color(p.muted))
        .id_salt(title)
        .show(ui, |ui| {
            ui.label(text);
            ui.horizontal_wrapped(|ui| {
                for example in examples {
                    let link = egui::Link::new(RichText::new(*example).monospace());
                    if ui.add(link).clicked() {
                        clicked = Some((*example).to_owned());
                    }
                }
            });
            for (label, names) in lines {
                ui.strong(label);
                ui.label(RichText::new(names).monospace().small());
            }
            more(ui);
        });
    clicked
}

/// The byte at a position in characters
fn byte_at(text: &str, chars: usize) -> usize {
    text.char_indices()
        .nth(chars)
        .map_or(text.len(), |(i, _)| i)
}

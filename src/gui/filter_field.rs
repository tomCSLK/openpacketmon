//! The filter fields: a line of text with suggestions while typing, as on the browser pages.
//! Enter applies the filter; only a line chosen with the arrow keys is taken into the field.

use std::sync::Arc;

use eframe::egui::text::{CCursor, CCursorRange};
use eframe::egui::{
    self, Area, CornerRadius, Event, Frame, Id, Key, Margin, Order, RichText, ScrollArea, Sense,
    Stroke, TextEdit, Ui,
};

use crate::engine::Engine;
use crate::gui::icons;
use crate::gui::load::Loader;
use crate::gui::theme::{self, palette};
use crate::gui::widgets::{self, Kind};
use crate::view::i18n::Lang;
use crate::view::packets::filter_problem;
use crate::view::suggest::{Catalog, Expect, Line, catalog, lines, position, value_rows};
use eframe::egui::emath::GuiRounding as _;

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
    /// The head of the list: what the lines are
    head: String,
    /// The help below the toolbar is open
    help_open: bool,
    /// A filter with a mistake was applied: the mistake shows below the toolbar
    error_open: bool,
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
        self.head = match place.expect {
            Expect::Term if self.text.trim().is_empty() => t.get("sugg-common"),
            Expect::Term => t.get("sugg-terms"),
            Expect::Operator => t.get("sugg-compare"),
            Expect::Value | Expect::Set => t.get("sugg-value"),
            Expect::Next => t.get("sugg-join"),
            Expect::Brace => String::new(),
        };
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

    /// The toolbar of a filter (`.toolbar` with `.ffield`, the help button and "Apply"). A filter
    /// with a mistake is not applied: its mistake shows below the toolbar.
    pub fn bar(
        &mut self,
        ui: &mut Ui,
        t: Lang,
        engine: &Engine,
        connections: bool,
        placeholder: &str,
        help: &str,
    ) -> Outcome {
        self.bar_with(ui, t, engine, connections, placeholder, help, 0.0, |_| {})
    }

    /// [`Self::bar`] with more after "Apply" that is `after_width` wide (the field to go to a
    /// packet)
    #[allow(clippy::too_many_arguments)]
    pub fn bar_with(
        &mut self,
        ui: &mut Ui,
        t: Lang,
        engine: &Engine,
        connections: bool,
        placeholder: &str,
        help: &str,
        after_width: f32,
        after: impl FnOnce(&mut Ui),
    ) -> Outcome {
        let mut outcome = Outcome::default();
        widgets::toolbar(ui, |ui| {
            let apply = t.get("apply");
            let mut beside = widgets::button_width(ui, &apply) + theme::CTL_H + 2.0 * theme::SP2;
            if after_width > 0.0 {
                beside += after_width + theme::SP2;
            }
            let width = (ui.available_width() - beside).max(240.0);
            let field = self.field(ui, t, engine, connections, placeholder, width);
            if widgets::iconbtn(ui, icons::HELP, help, self.help_open, false)
                .on_hover_text(help)
                .clicked()
            {
                self.help_open = !self.help_open;
            }
            let clicked = widgets::button(ui, apply, Kind::Primary).clicked();
            outcome.apply = field.apply || clicked;
            after(ui);
        });
        if outcome.apply && self.problem(t, connections).is_some() {
            self.error_open = true;
            outcome.apply = false;
        }
        outcome
    }

    /// Below the toolbar: the mistake of a filter that was not applied (`.ferr`) and the help
    /// (`.helppop`) with its examples and `more`; returns the example clicked
    pub fn extras(
        &mut self,
        ui: &mut Ui,
        t: Lang,
        connections: bool,
        text: &str,
        examples: &[&str],
        more: impl FnOnce(&mut Ui),
    ) -> Option<String> {
        let p = palette(ui.ctx());
        let problem = self.problem(t, connections).map(str::to_owned);
        if problem.is_none() {
            self.error_open = false;
        }
        if self.error_open
            && let Some(problem) = problem
        {
            Frame::new()
                .fill(p.err_bg)
                .inner_margin(Margin::symmetric(theme::SP3 as i8, 6))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        let (rect, _) =
                            ui.allocate_exact_size(egui::vec2(14.0, 14.0), Sense::hover());
                        widgets::paint_icon(ui, icons::X_CIRCLE, rect, p.err);
                        ui.add(egui::Label::new(widgets::small(&problem).color(p.err)).wrap());
                    });
                });
            widgets::rule(ui);
        }
        let mut clicked = None;
        if self.help_open {
            Frame::new()
                .fill(p.surface_2)
                .inner_margin(Margin::symmetric(theme::SP4 as i8, theme::SP3 as i8))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.spacing_mut().item_spacing.y = theme::SP2;
                    ui.add(
                        egui::Label::new(
                            widgets::small(text)
                                .color(p.text_2)
                                .line_height(Some(theme::FS_SM * 1.45)),
                        )
                        .wrap(),
                    );
                    // rows as high as the examples (24 px), not as other controls
                    ui.spacing_mut().interact_size.y = 24.0;
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                        for example in examples {
                            if example_button(ui, example).clicked() {
                                clicked = Some((*example).to_owned());
                            }
                        }
                    });
                    more(ui);
                });
            widgets::rule(ui);
        }
        clicked
    }

    /// The field itself (`.ffield`): the filter icon at the left, the text in the font of code,
    /// a button to empty it at the right; green when the filter is right, red when not
    fn field(
        &mut self,
        ui: &mut Ui,
        t: Lang,
        engine: &Engine,
        connections: bool,
        placeholder: &str,
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

        let (rect, _) = ui.allocate_exact_size(egui::vec2(width, theme::CTL_H), Sense::hover());
        let valid = (!self.text.trim().is_empty()).then(|| self.problem(t, connections).is_none());
        let (fill, line, ring, focus_line) = match valid {
            Some(true) => (p.ok_bg, p.ok_line, p.ok_bg, p.ok),
            Some(false) => (p.err_bg, p.err_line, p.err_bg, p.err),
            None => (p.surface, p.border_strong, p.accent_bg, p.accent),
        };
        let radius = CornerRadius::same(theme::RADIUS);
        if focused {
            ui.painter().rect_filled(
                rect.expand(3.0),
                CornerRadius::same(theme::RADIUS + 3),
                ring,
            );
        }
        ui.painter().rect(
            rect,
            radius,
            fill,
            Stroke::new(1.0, if focused { focus_line } else { line }),
            egui::StrokeKind::Inside,
        );
        let icon = egui::Rect::from_center_size(
            egui::pos2(rect.left() + 10.0 + 8.0, rect.center().y),
            egui::vec2(16.0, 16.0),
        );
        widgets::paint_icon(ui, icons::FILTER, icon, p.text_3);
        // the text in a line of its own height in the middle; children leave the cursor of the
        // toolbar where the field ends
        let line_h = (theme::FS_BASE * 1.45).round();
        let text_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left() + 32.0, rect.center().y - line_h / 2.0),
            egui::pos2(rect.right() - 34.0, rect.center().y + line_h / 2.0),
        );
        let mut inner = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(text_rect)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        // no taller than its line, or it starts at the top of the line and its text sits low
        inner.spacing_mut().interact_size.y = 0.0;
        let mut output = TextEdit::singleline(&mut self.text)
            .id(id)
            .frame(egui::Frame::NONE)
            .hint_text(
                RichText::new(placeholder)
                    .size(theme::FS_BASE)
                    .color(p.text_3),
            )
            .font(theme::mono_font(theme::FS_SM))
            .margin(Margin::ZERO)
            .align(egui::Align2::LEFT_CENTER)
            .min_size(text_rect.size())
            .desired_width(text_rect.width())
            .show(&mut inner);
        let response = output.response.response.clone();
        if !self.text.is_empty() {
            let clear = egui::Rect::from_min_size(
                egui::pos2(rect.right() - 4.0 - 26.0, rect.center().y - 13.0),
                egui::vec2(26.0, 26.0),
            );
            let label = t.get("filter-clear");
            let mut place = ui.new_child(egui::UiBuilder::new().max_rect(clear));
            let button =
                widgets::iconbtn(&mut place, icons::X, &label, false, true).on_hover_text(&label);
            if button.clicked() {
                self.text.clear();
                self.list = None;
                self.seen = None;
                outcome.apply = true;
                response.request_focus();
            }
        }
        // the key `/` asked for the filter field of the page
        if crate::gui::frame::take_focus_request(ui.ctx()) {
            response.request_focus();
            output.state.cursor.set_char_range(Some(CCursorRange::two(
                CCursor::new(0),
                CCursor::new(self.text.chars().count()),
            )));
            output.state.clone().store(ui.ctx(), id);
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
            if let Some(clicked) = self.draw_list(ui, id, rect) {
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

    /// Draws the list below the field (`.sugg`): a head, then the lines; returns the line clicked
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
        let head = self.head.clone();
        let note = self.note.clone();
        let area = Area::new(id.with("list"))
            .order(Order::Foreground)
            .fixed_pos(field.left_bottom() + egui::vec2(0.0, 4.0))
            .show(ui.ctx(), |ui| {
                Frame::new()
                    .fill(p.surface)
                    .stroke(Stroke::new(1.0, p.border))
                    .corner_radius(CornerRadius::same(theme::RADIUS))
                    .shadow(p.shadow_2())
                    .inner_margin(Margin::same(theme::SP1 as i8))
                    .show(ui, |ui| {
                        ui.set_width(field.width() - 2.0 * theme::SP1 - 2.0);
                        ui.spacing_mut().item_spacing.y = 0.0;
                        if !head.is_empty() {
                            Frame::new()
                                .inner_margin(Margin {
                                    left: 10,
                                    right: 10,
                                    top: 6,
                                    bottom: 2,
                                })
                                .show(ui, |ui| {
                                    ui.label(
                                        RichText::new(head.to_uppercase())
                                            .size(theme::FS_XS)
                                            .color(p.text_3)
                                            .extra_letter_spacing(0.48),
                                    );
                                });
                        }
                        if !note.is_empty() {
                            Frame::new()
                                .inner_margin(Margin::symmetric(10, 2))
                                .show(ui, |ui| {
                                    ui.label(widgets::small(&note).color(p.text_3));
                                });
                        }
                        ScrollArea::vertical()
                            .max_height(300.0 - 40.0)
                            .show(ui, |ui| {
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

/// A line of the suggestions: the name in the font of code, its meaning after it (at least 150 px
/// further), as in `.sugg button`
fn suggestion(ui: &mut Ui, line: &Line, chosen: bool) -> egui::Response {
    let p = palette(ui.ctx());
    let name =
        ui.painter()
            .layout_no_wrap(line.name.clone(), theme::mono_font(theme::FS_SM), p.text);
    let height = (theme::FS_BASE * 1.45 + 12.0).round();
    let size = egui::vec2(ui.available_width(), height);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    response
        .widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, chosen, &line.name));
    if chosen || response.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(theme::RADIUS_SM), p.selected);
    }
    let name_width = name.size().x;
    ui.painter().galley(
        egui::pos2(rect.left() + 10.0, rect.center().y - name.size().y / 2.0),
        name,
        p.text,
    );
    if let Some(hint) = &line.hint {
        let left = rect.left() + 10.0 + name_width.max(150.0) + theme::SP4;
        let room = rect.right() - left - 10.0;
        if room > 20.0 {
            let mut job = egui::text::LayoutJob::simple_singleline(
                hint.clone(),
                egui::FontId::proportional(theme::FS_SM),
                p.text_3,
            );
            job.wrap = egui::text::TextWrapping::truncate_at_width(room);
            let galley = ui.painter().layout_job(job);
            let y = rect.center().y - galley.size().y / 2.0;
            ui.painter().galley(egui::pos2(left, y), galley, p.text_3);
        }
    }
    response
}

/// An example of the help (`.helppop .ex button`): code in the color of links in a small frame
fn example_button(ui: &mut Ui, text: &str) -> egui::Response {
    let p = palette(ui.ctx());
    let galley =
        ui.painter()
            .layout_no_wrap(text.to_owned(), theme::mono_font(theme::FS_XS), p.accent);
    let size = egui::vec2(galley.size().x + 16.0, 24.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, text));
    let line = if response.hovered() {
        p.accent
    } else {
        p.border
    };
    ui.painter().rect(
        rect,
        CornerRadius::same(theme::RADIUS_SM),
        p.surface,
        Stroke::new(1.0, line),
        egui::StrokeKind::Inside,
    );
    let at = (rect.center() - galley.size() / 2.0).round_to_pixels(ui.ctx().pixels_per_point());
    ui.painter().galley(at, galley, p.accent);
    widgets::focus_ring(ui, &response, theme::RADIUS_SM);
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// The byte at a position in characters
fn byte_at(text: &str, chars: usize) -> usize {
    text.char_indices()
        .nth(chars)
        .map_or(text.len(), |(i, _)| i)
}

//! The licenses of openpacketmon and of what it uses.

use eframe::egui::{RichText, ScrollArea, Ui};

use crate::gui::App;
use crate::gui::pages::head;
use crate::gui::widgets;
use crate::view::LICENSES;

pub fn show(app: &mut App, ui: &mut Ui) {
    let t = app.t;
    head(app, ui, &t.get("licenses"), None, |_| {});
    let lines: Vec<&str> = LICENSES.lines().collect();
    let height = ui.text_style_height(&eframe::egui::TextStyle::Monospace) + 2.0;
    widgets::flush_card(
        ui,
        "NOTICE · THIRD-PARTY-LICENSES.txt",
        |_| {},
        |ui| {
            ScrollArea::both().auto_shrink(false).show_rows(
                ui,
                height,
                lines.len(),
                |ui, range| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    ui.spacing_mut().interact_size.y = height - 2.0;
                    for line in &lines[range] {
                        ui.horizontal(|ui| {
                            ui.add_space(14.0);
                            ui.label(RichText::new(*line).monospace());
                        });
                    }
                },
            );
        },
    );
}

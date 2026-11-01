//! The traffic charts of the overview and the statistics.

use eframe::egui::{Color32, RichText, Ui};
use egui_plot::{Corner, Legend, Line, Plot, PlotPoint};

use crate::gui::theme::palette;
use crate::utils::types::timestamp::Timestamp;

/// A line of a chart: seconds since `start` and a value
pub struct Series {
    pub name: String,
    pub color: Color32,
    pub points: Vec<[f64; 2]>,
}

/// The time of day of a second
fn clock(second: i64) -> String {
    jiff::Timestamp::from_second(second).map_or_else(
        |_| String::new(),
        |time| {
            time.to_zoned(jiff::tz::TimeZone::system())
                .strftime("%H:%M:%S")
                .to_string()
        },
    )
}

/// The value of a line at a place of the chart: the point before it
fn value_at(points: &[[f64; 2]], x: f64) -> Option<f64> {
    let after = points.partition_point(|p| p[0] <= x);
    after.checked_sub(1).map(|i| points[i][1])
}

/// Draws the lines; `value` writes a number of the vertical axis
pub fn chart(
    ui: &mut Ui,
    id: &str,
    height: f32,
    start: Timestamp,
    series: &[Series],
    value: impl Fn(f64) -> String + Clone + 'static,
) {
    let p = palette(ui.ctx());
    let first = start.secs();
    let axis_value = value.clone();
    let plot = Plot::new(id)
        .height(height)
        .allow_drag(false)
        .allow_zoom(false)
        .allow_scroll(false)
        .allow_boxed_zoom(false)
        .allow_axis_zoom_drag(false)
        .allow_double_click_reset(false)
        .show_x(false)
        .show_y(false)
        .show_background(false)
        .include_y(0.0)
        .grid_color(p.grid)
        .grid_fade(0.2)
        .grid_spacing(32.0..=160.0)
        .y_axis_min_width(64.0)
        .legend(Legend::default().position(Corner::LeftTop))
        .x_axis_formatter(move |mark, _| {
            #[allow(clippy::cast_possible_truncation)] // seconds of a capture
            clock(first + mark.value as i64)
        })
        .y_axis_formatter(move |mark, _| {
            if mark.value < 0.0 {
                String::new()
            } else {
                axis_value(mark.value)
            }
        });
    let shown = plot.show(ui, |plot| {
        for s in series {
            plot.line(
                Line::new(s.name.clone(), s.points.clone())
                    .color(s.color)
                    .width(1.5)
                    .fill(0.0)
                    .fill_alpha(0.12),
            );
        }
        plot.pointer_coordinate()
    });
    if let Some(PlotPoint { x, .. }) = shown.inner
        && shown.response.hovered()
    {
        shown.response.on_hover_ui_at_pointer(|ui| {
            #[allow(clippy::cast_possible_truncation)] // seconds of a capture
            ui.label(
                RichText::new(clock(first + x as i64))
                    .small()
                    .color(p.muted),
            );
            for s in series {
                if let Some(v) = value_at(&s.points, x) {
                    ui.label(RichText::new(format!("{}: {}", s.name, value(v))).color(p.text));
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_value_at_a_place_is_the_one_before_it() {
        let points = [[0.0, 1.0], [10.0, 2.0], [20.0, 3.0]];
        assert_eq!(value_at(&points, 15.0), Some(2.0));
        assert_eq!(value_at(&points, 20.0), Some(3.0));
        assert_eq!(value_at(&points, -1.0), None);
        assert_eq!(value_at(&[], 1.0), None);
    }
}

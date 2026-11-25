//! The traffic charts of the overview and the statistics, drawn after the draft
//! (`docs/entwurf/assets/pages.js`, `drawChart`) and as in opennetmon 0.7.1 (`PLAN.md` 5.10): the
//! total of both directions as a grey line over a grey area, incoming and outgoing as solid lines
//! over it, five grid lines, and under the pointer a line across, the points and their values in a
//! dark label. The legend ([`crate::gui::widgets::legend`]) hides and shows each series; the axis
//! reaches the highest value of the series shown.

use eframe::egui::epaint::{Mesh, PathStroke, Vertex, WHITE_UV};
use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontId, Frame, Margin, Pos2, Rect, Sense, Shape, Stroke,
    Ui, pos2, vec2,
};

use crate::gui::theme::{self, palette};

/// Room for the labels of the axes, in the units of the draft
const PAD_LEFT: f32 = 64.0;
const PAD_RIGHT: f32 = 12.0;
const PAD_TOP: f32 = 10.0;
const PAD_BOTTOM: f32 = 26.0;

/// The series of a chart: the total, incoming and outgoing
pub const TOTAL: usize = 0;

/// Which series a chart shows ([`TOTAL`], incoming, outgoing): all at first; the choice lasts as
/// long as the program runs, for each chart of its own
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Shown(pub [bool; 3]);

impl Default for Shown {
    fn default() -> Self {
        Self([true; 3])
    }
}

/// The total, incoming and outgoing value of a point
fn series(point: [f64; 2]) -> [f64; 3] {
    [point[0] + point[1], point[0], point[1]]
}

/// The top of the axis: the highest value of the series shown, a little above
fn axis_top(points: &[[f64; 2]], shown: Shown) -> f64 {
    let max = points
        .iter()
        .flat_map(|point| {
            let values = series(*point);
            (0..3).filter(move |k| shown.0[*k]).map(move |k| values[k])
        })
        .fold(0.0_f64, f64::max);
    nice_max(max * 1.08)
}

/// A chart of incoming and outgoing values
pub struct Chart<'a> {
    /// Height in the units of the draft (220 in the overview, 200 in the statistics)
    pub height: f32,
    /// Incoming and outgoing of each point
    pub points: &'a [[f64; 2]],
    /// Labels of the time axis: one more than this, the first at the start, the last at the end
    pub ticks: usize,
    /// "Total", "Incoming", "Outgoing"
    pub names: [&'a str; 3],
    pub shown: Shown,
}

/// The top of the vertical axis: 1, 2, 2.5, 5 or 10 times a power of ten (`niceMax`)
fn nice_max(m: f64) -> f64 {
    if m <= 0.0 {
        return 1.0;
    }
    let e = 10f64.powf(m.log10().floor());
    let f = m / e;
    let step = if f <= 1.0 {
        1.0
    } else if f <= 2.0 {
        2.0
    } else if f <= 2.5 {
        2.5
    } else if f <= 5.0 {
        5.0
    } else {
        10.0
    };
    step * e
}

/// The point of index `i` of `n` nearest to `x` (in the units of the draft)
fn nearest(x: f32, width: f32, n: usize) -> usize {
    let step = (width - PAD_LEFT - PAD_RIGHT) / (n.max(2) - 1) as f32;
    let i = ((x - PAD_LEFT) / step).round();
    (i.max(0.0) as usize).min(n.saturating_sub(1))
}

/// Draws the chart in a box with the room of the draft (`.chartbox`: 12 px above, 16 px at the
/// sides, 8 px below). `x_label` and `tip` name a point, `value` writes a value.
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)] // points of a chart
pub fn chart(
    ui: &mut Ui,
    c: &Chart,
    x_label: impl Fn(usize) -> String,
    tip: impl Fn(usize) -> String,
    value: impl Fn(f64) -> String,
) {
    Frame::new()
        .inner_margin(Margin {
            left: 16,
            right: 16,
            top: 12,
            bottom: 8,
        })
        .show(ui, |ui| draw(ui, c, &x_label, &tip, &value));
}

#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
#[allow(clippy::many_single_char_names, clippy::similar_names)] // coordinates of the drawing
fn draw(
    ui: &mut Ui,
    c: &Chart,
    x_label: &dyn Fn(usize) -> String,
    tip: &dyn Fn(usize) -> String,
    value: &dyn Fn(f64) -> String,
) {
    let p = palette(ui.ctx());
    let shown = ui.available_width();
    // the draft draws in a box as wide as the card and shrinks it into the room inside
    let width = (shown + 32.0).max(320.0);
    let scale = shown / width;
    let height = c.height;
    let (rect, response) = ui.allocate_exact_size(vec2(shown, height * scale), Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }
    let n = c.points.len();
    let top = axis_top(c.points, c.shown);
    let at = |x: f32, y: f32| pos2(rect.left() + x * scale, rect.top() + y * scale);
    let x = |i: usize| {
        PAD_LEFT
            + (width - PAD_LEFT - PAD_RIGHT)
                * if n > 1 {
                    i as f32 / (n - 1) as f32
                } else {
                    0.0
                }
    };
    let y = |v: f64| PAD_TOP + (height - PAD_TOP - PAD_BOTTOM) * (1.0 - (v / top) as f32);
    let painter = ui.painter_at(rect.expand(8.0));
    let font = FontId::proportional(11.0 * scale);

    for k in 0..=4 {
        let v = top * f64::from(k) / 4.0;
        let a = at(PAD_LEFT, y(v));
        let b = at(width - PAD_RIGHT, y(v));
        painter.line_segment([a, b], Stroke::new(1.0, p.border));
        painter.text(
            at(PAD_LEFT - 8.0, y(v) + 4.0 - 3.9),
            Align2::RIGHT_CENTER,
            value(v),
            font.clone(),
            p.text_3,
        );
    }
    if n > 0 {
        for k in 0..=c.ticks {
            let i = ((n - 1) as f32 * k as f32 / c.ticks.max(1) as f32).round() as usize;
            let align = if k == 0 {
                Align2::LEFT_CENTER
            } else if k == c.ticks {
                Align2::RIGHT_CENTER
            } else {
                Align2::CENTER_CENTER
            };
            painter.text(
                at(x(i), height - 6.0 - 3.9),
                align,
                x_label(i),
                font.clone(),
                p.text_3,
            );
        }
    }
    if n == 0 {
        return;
    }

    let colors = [p.s_total, p.s1, p.s2];
    let lines: Vec<Vec<Pos2>> = (0..3)
        .map(|k| {
            (0..n)
                .map(|i| at(x(i), y(series(c.points[i])[k])))
                .collect()
        })
        .collect();
    if c.shown.0[TOTAL] {
        // the area below the total, as one mesh (an area is not convex)
        let base = at(0.0, y(0.0)).y;
        let fill = p.s_total_fill;
        let mut mesh = Mesh::default();
        for pt in &lines[TOTAL] {
            mesh.vertices.push(Vertex {
                pos: *pt,
                uv: WHITE_UV,
                color: fill,
            });
            mesh.vertices.push(Vertex {
                pos: pos2(pt.x, base),
                uv: WHITE_UV,
                color: fill,
            });
        }
        for i in 0..n.saturating_sub(1) as u32 {
            let (a, b) = (2 * i, 2 * i + 2);
            mesh.add_triangle(a, a + 1, b);
            mesh.add_triangle(b, a + 1, b + 1);
        }
        painter.add(Shape::mesh(mesh));
    }
    for k in (0..3).filter(|k| c.shown.0[*k]) {
        let width = if k == TOTAL { 1.2 } else { 1.6 } * scale;
        painter.add(Shape::line(
            lines[k].clone(),
            PathStroke::new(width, colors[k]),
        ));
    }

    let Some(pointer) = response.hover_pos() else {
        return;
    };
    let i = nearest((pointer.x - rect.left()) / scale, width, n);
    let cross = at(x(i), 0.0).x;
    painter.line_segment(
        [
            pos2(cross, at(0.0, PAD_TOP).y),
            pos2(cross, at(0.0, height - PAD_BOTTOM).y),
        ],
        Stroke::new(1.0, p.border_strong),
    );
    for k in (0..3).filter(|k| c.shown.0[*k]) {
        painter.circle(
            lines[k][i],
            4.0 * scale,
            colors[k],
            Stroke::new(2.0 * scale, p.surface),
        );
    }
    label(ui, rect, scale, x(i), c, i, tip, value);
}

/// The dark label with the values of point `i` (`.ctip`), 14 px right of the point and 18 px below
/// the top of the box, kept inside it
#[allow(clippy::too_many_arguments)]
#[allow(clippy::many_single_char_names)] // coordinates of the drawing
fn label(
    ui: &Ui,
    rect: Rect,
    scale: f32,
    x: f32,
    c: &Chart,
    i: usize,
    tip: &dyn Fn(usize) -> String,
    value: &dyn Fn(f64) -> String,
) {
    let p = palette(ui.ctx());
    let size = theme::FS_XS;
    let line_h = size * 1.45;
    let painter = ui.ctx().layer_painter(egui::LayerId::new(
        egui::Order::Tooltip,
        egui::Id::NULL.with("chart-tip"),
    ));
    let plain = FontId::proportional(size);
    let bold = theme::mono_medium_font(size);
    let title = painter.layout_no_wrap(tip(i), plain.clone(), p.surface);
    let values = series(c.points[i]);
    let colors = [p.s_total, p.s1, p.s2];
    let rows: Vec<_> = (0..3)
        .filter(|k| c.shown.0[*k])
        .map(|k| (c.names[k], values[k], colors[k]))
        .map(|(name, v, color)| {
            let dot = painter.layout_no_wrap("●".to_owned(), plain.clone(), color);
            let name = painter.layout_no_wrap(format!(" {name} "), plain.clone(), p.surface);
            let v = painter.layout_no_wrap(value(v), bold.clone(), p.surface);
            (dot, name, v)
        })
        .collect();
    let inner_w = rows
        .iter()
        .map(|(d, n, v)| d.size().x + n.size().x + v.size().x)
        .fold(title.size().x, f32::max);
    let size_box = vec2(inner_w + 16.0, (1 + rows.len()) as f32 * line_h + 12.0);
    // the draft places the label from the left edge of the box around the chart (16 px left of it)
    let left = (rect.left() - 16.0 + x * scale + 14.0).min(rect.right() - size_box.x - 4.0);
    let top = rect.top() - 12.0 + 18.0;
    let frame = Rect::from_min_size(pos2(left, top), size_box);
    painter.add(
        p.shadow_2()
            .as_shape(frame, CornerRadius::same(theme::RADIUS_SM)),
    );
    painter.rect_filled(frame, CornerRadius::same(theme::RADIUS_SM), p.text);
    let mut row_top = frame.top() + 6.0;
    let center = |galley_h: f32, row_top: f32| row_top + (line_h - galley_h) / 2.0;
    painter.galley(
        pos2(frame.left() + 8.0, center(title.size().y, row_top)),
        title,
        p.surface,
    );
    for (dot, name, v) in rows {
        row_top += line_h;
        let mut px = frame.left() + 8.0;
        for g in [dot, name, v] {
            let w = g.size().x;
            painter.galley(
                pos2(px, center(g.size().y, row_top)),
                g,
                Color32::PLACEHOLDER,
            );
            px += w;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_top_of_the_axis_is_a_round_number() {
        assert!((nice_max(0.0) - 1.0).abs() < f64::EPSILON);
        assert!((nice_max(83_000.0) - 100_000.0).abs() < 1e-6);
        assert!((nice_max(1_900.0) - 2_000.0).abs() < 1e-6);
        assert!((nice_max(2_300.0) - 2_500.0).abs() < 1e-6);
        assert!((nice_max(41.0) - 50.0).abs() < 1e-9);
    }

    #[test]
    fn the_total_is_the_sum_and_the_axis_follows_the_series_shown() {
        assert_eq!(series([300.0, 50.0]), [350.0, 300.0, 50.0]);
        let points = [[300.0, 50.0], [700.0, 900.0]];
        // the total reaches 1 600: up to 2 000
        assert!((axis_top(&points, Shown::default()) - 2_000.0).abs() < 1e-9);
        // without it, outgoing reaches 900: up to 1 000
        assert!((axis_top(&points, Shown([false, true, true])) - 1_000.0).abs() < 1e-9);
        // incoming alone reaches 700
        assert!((axis_top(&points, Shown([false, true, false])) - 1_000.0).abs() < 1e-9);
        assert!((axis_top(&points, Shown([false, false, true])) - 1_000.0).abs() < 1e-9);
        // nothing shown: an axis up to 1
        assert!((axis_top(&points, Shown([false; 3])) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn the_pointer_takes_the_nearest_point() {
        // 300 points between 64 and 1218
        assert_eq!(nearest(0.0, 1230.0, 300), 0);
        assert_eq!(nearest(1229.0, 1230.0, 300), 299);
        assert_eq!(nearest(64.0 + 1154.0 / 299.0 * 10.4, 1230.0, 300), 10);
        assert_eq!(nearest(500.0, 1230.0, 1), 0);
    }
}

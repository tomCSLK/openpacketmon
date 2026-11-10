//! The traffic charts of the overview and the statistics, drawn as in the draft
//! (`docs/entwurf/assets/pages.js`, `drawChart`): incoming as a line over a pale area, outgoing as
//! a dashed line, five grid lines, and under the pointer a line across, the two points and their
//! values in a dark label.

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

/// A chart of incoming and outgoing values
pub struct Chart<'a> {
    /// Height in the units of the draft (220 in the overview, 200 in the statistics)
    pub height: f32,
    /// Incoming and outgoing of each point
    pub points: &'a [[f64; 2]],
    /// Labels of the time axis: one more than this, the first at the start, the last at the end
    pub ticks: usize,
    /// "Incoming", "Outgoing"
    pub names: [&'a str; 2],
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
    let max = c
        .points
        .iter()
        .flat_map(|p| [p[0], p[1]])
        .fold(0.0_f64, f64::max);
    let top = nice_max(max * 1.08);
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

    let line: Vec<Pos2> = (0..n).map(|i| at(x(i), y(c.points[i][0]))).collect();
    let dashed: Vec<Pos2> = (0..n).map(|i| at(x(i), y(c.points[i][1]))).collect();
    // the area below the incoming line, as one mesh (an area is not convex)
    let base = at(0.0, y(0.0)).y;
    let fill = p.s1.gamma_multiply(0.10);
    let mut mesh = Mesh::default();
    for pt in &line {
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
    let width_1 = 1.6 * scale;
    painter.add(Shape::line(line.clone(), PathStroke::new(width_1, p.s1)));
    painter.extend(Shape::dashed_line(
        &dashed,
        Stroke::new(width_1, p.s2),
        5.0 * scale,
        3.0 * scale,
    ));

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
    for (point, color) in [(line[i], p.s1), (dashed[i], p.s2)] {
        painter.circle(
            point,
            4.0 * scale,
            color,
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
    let rows: Vec<_> = [
        (c.names[0], c.points[i][0], p.s1),
        (c.names[1], c.points[i][1], p.s2),
    ]
    .into_iter()
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
    let size_box = vec2(inner_w + 16.0, 3.0 * line_h + 12.0);
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
    fn the_pointer_takes_the_nearest_point() {
        // 300 points between 64 and 1218
        assert_eq!(nearest(0.0, 1230.0, 300), 0);
        assert_eq!(nearest(1229.0, 1230.0, 300), 299);
        assert_eq!(nearest(64.0 + 1154.0 / 299.0 * 10.4, 1230.0, 300), 10);
        assert_eq!(nearest(500.0, 1230.0, 1), 0);
    }
}

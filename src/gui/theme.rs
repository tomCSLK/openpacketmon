//! Colors, fonts and the look of the widgets.

// the colors read as in the stylesheet
#![allow(clippy::unreadable_literal)]

use std::sync::Arc;

use eframe::egui::{
    self, Color32, Context, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Stroke,
    TextStyle, Theme, Visuals,
};

/// The family of the bold text
const SEMIBOLD: &str = "semibold";

const fn rgb(hex: u32) -> Color32 {
    Color32::from_rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

/// White with the given opacity in percent
const fn white(percent: u32) -> Color32 {
    let a = percent * 255 / 100;
    let v = a as u8;
    Color32::from_rgba_premultiplied(v, v, v, v)
}

/// Background colors of the packet list by the kind of packet
#[derive(Clone, Copy)]
pub struct RowColors {
    pub bad: Color32,
    pub tcp_control: Color32,
    pub dns: Color32,
    pub local: Color32,
    pub tls: Color32,
    pub http: Color32,
    pub icmp: Color32,
    pub udp: Color32,
}

/// All colors of one design
#[derive(Clone, Copy)]
pub struct Palette {
    pub bg: Color32,
    pub surface: Color32,
    pub surface_2: Color32,
    pub surface_hover: Color32,
    pub border: Color32,
    pub border_strong: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub faint: Color32,
    pub accent: Color32,
    pub accent_soft: Color32,
    pub ok: Color32,
    pub ok_soft: Color32,
    pub warn: Color32,
    pub warn_soft: Color32,
    pub down: Color32,
    pub down_soft: Color32,
    pub neutral_soft: Color32,
    pub grid: Color32,
    pub side_top: Color32,
    pub side_bottom: Color32,
    pub side_text: Color32,
    pub side_muted: Color32,
    pub side_active: Color32,
    pub side_active_text: Color32,
    pub side_hover: Color32,
    pub rows: RowColors,
}

pub const LIGHT: Palette = Palette {
    bg: rgb(0xf4f6fa),
    surface: rgb(0xffffff),
    surface_2: rgb(0xf1f4f9),
    surface_hover: rgb(0xf6f8fc),
    border: rgb(0xe3e8ef),
    border_strong: rgb(0xcfd7e3),
    text: rgb(0x0f172a),
    muted: rgb(0x5b6b80),
    faint: rgb(0x8a98ab),
    accent: rgb(0x2563eb),
    accent_soft: rgb(0xe8efff),
    ok: rgb(0x16a34a),
    ok_soft: rgb(0xe7f6ec),
    warn: rgb(0xd97706),
    warn_soft: rgb(0xfdf3e2),
    down: rgb(0xdc2626),
    down_soft: rgb(0xfdecec),
    neutral_soft: rgb(0xeef1f5),
    grid: rgb(0xd9dee7),
    side_top: rgb(0x5d12af),
    side_bottom: rgb(0x220f4b),
    side_text: rgb(0xffffff),
    side_muted: white(62),
    side_active: rgb(0xefe7f8),
    side_active_text: rgb(0x5d12af),
    side_hover: white(12),
    rows: RowColors {
        bad: rgb(0xfde2e2),
        tcp_control: rgb(0xe9ecf1),
        dns: rgb(0xe1edff),
        local: rgb(0xfdf3cf),
        tls: rgb(0xefe5ff),
        http: rgb(0xdff5e6),
        icmp: rgb(0xfde7f3),
        udp: rgb(0xe2f5f7),
    },
};

pub const DARK: Palette = Palette {
    bg: rgb(0x130d24),
    surface: rgb(0x1b1430),
    surface_2: rgb(0x221a3b),
    surface_hover: rgb(0x241c40),
    border: rgb(0x2c2347),
    border_strong: rgb(0x3c3260),
    text: rgb(0xece8f4),
    muted: rgb(0xaaa1c3),
    faint: rgb(0x7b7197),
    accent: rgb(0x3b82f6),
    accent_soft: rgb(0x24234f),
    ok: rgb(0x22c55e),
    ok_soft: rgb(0x10301f),
    warn: rgb(0xf59e0b),
    warn_soft: rgb(0x36270b),
    down: rgb(0xef4444),
    down_soft: rgb(0x3a1518),
    neutral_soft: rgb(0x251d3e),
    grid: rgb(0x2b2344),
    side_top: rgb(0x2a0b52),
    side_bottom: rgb(0x0e0620),
    side_text: rgb(0xe6dcf5),
    side_muted: white(52),
    side_active: white(10),
    side_active_text: rgb(0xffffff),
    side_hover: white(7),
    rows: RowColors {
        bad: rgb(0x4a1a1e),
        tcp_control: rgb(0x1f2738),
        dns: rgb(0x15294a),
        local: rgb(0x36290b),
        tls: rgb(0x2b1f4a),
        http: rgb(0x11331f),
        icmp: rgb(0x3d1733),
        udp: rgb(0x0f3236),
    },
};

/// The colors of the design in use
pub fn palette(ctx: &Context) -> &'static Palette {
    match ctx.theme() {
        Theme::Dark => &DARK,
        Theme::Light => &LIGHT,
    }
}

impl Palette {
    /// The background of a row of the packet list; `class` is the color class of the row
    pub fn row(&self, class: &str) -> Option<Color32> {
        match class {
            "bad" => Some(self.rows.bad),
            "tcp-control" => Some(self.rows.tcp_control),
            "dns" => Some(self.rows.dns),
            "local" => Some(self.rows.local),
            "tls" => Some(self.rows.tls),
            "http" => Some(self.rows.http),
            "icmp" => Some(self.rows.icmp),
            "udp" => Some(self.rows.udp),
            _ => None,
        }
    }
}

fn visuals(p: &Palette, dark: bool) -> Visuals {
    let mut v = if dark {
        Visuals::dark()
    } else {
        Visuals::light()
    };
    v.panel_fill = p.bg;
    v.window_fill = p.surface;
    v.window_stroke = Stroke::new(1.0, p.border_strong);
    v.window_corner_radius = CornerRadius::same(12);
    v.menu_corner_radius = CornerRadius::same(8);
    v.extreme_bg_color = p.surface;
    v.text_edit_bg_color = Some(p.surface);
    v.faint_bg_color = p.surface_2;
    v.code_bg_color = p.surface_2;
    v.hyperlink_color = p.accent;
    v.warn_fg_color = p.warn;
    v.error_fg_color = p.down;
    v.selection.bg_fill = if dark { rgb(0x2f3f7a) } else { rgb(0xc9dafd) };
    v.selection.stroke = Stroke::new(1.5, p.accent);

    let radius = CornerRadius::same(8);
    let w = &mut v.widgets;
    w.noninteractive.bg_fill = p.surface;
    w.noninteractive.weak_bg_fill = p.surface;
    w.noninteractive.bg_stroke = Stroke::new(1.0, p.border);
    w.noninteractive.fg_stroke = Stroke::new(1.0, p.text);
    w.noninteractive.corner_radius = radius;
    for (state, fill, border) in [
        (&mut w.inactive, p.surface, p.border_strong),
        (&mut w.hovered, p.surface_hover, p.accent),
        (&mut w.active, p.surface_2, p.accent),
        (&mut w.open, p.surface_2, p.border_strong),
    ] {
        state.bg_fill = fill;
        state.weak_bg_fill = fill;
        state.bg_stroke = Stroke::new(1.0, border);
        state.fg_stroke = Stroke::new(1.0, p.text);
        state.corner_radius = radius;
        state.expansion = 0.0;
    }
    // the rail of a slider and the box of a checkbox stand out from the card
    w.inactive.bg_fill = p.neutral_soft;
    v.handle_shape = egui::style::HandleShape::Circle;
    v.slider_trailing_fill = true;
    v
}

/// Sets fonts, colors and spacing once, for both designs
pub fn install(ctx: &Context) {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "Inter".to_owned(),
        Arc::new(FontData::from_static(include_bytes!(
            "../../resources/fonts/Inter-Regular.ttf"
        ))),
    );
    fonts.font_data.insert(
        "Inter-SemiBold".to_owned(),
        Arc::new(FontData::from_static(include_bytes!(
            "../../resources/fonts/Inter-SemiBold.ttf"
        ))),
    );
    let fallback = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    let mut semibold = vec!["Inter-SemiBold".to_owned()];
    semibold.extend(fallback.iter().cloned());
    fonts
        .families
        .insert(FontFamily::Name(SEMIBOLD.into()), semibold);
    if let Some(family) = fonts.families.get_mut(&FontFamily::Proportional) {
        family.insert(0, "Inter".to_owned());
    }
    ctx.set_fonts(fonts);
    // packets can carry any character: what no font has is drawn as a box
    ctx.set_missing_glyph_policy(egui::MissingGlyphPolicy::Tofu);

    ctx.set_visuals_of(Theme::Light, visuals(&LIGHT, false));
    ctx.set_visuals_of(Theme::Dark, visuals(&DARK, true));
    ctx.all_styles_mut(|style| {
        style.text_styles = [
            (TextStyle::Small, FontId::proportional(12.0)),
            (TextStyle::Body, FontId::proportional(14.0)),
            (TextStyle::Button, FontId::proportional(14.0)),
            (TextStyle::Monospace, FontId::monospace(13.0)),
            (TextStyle::Heading, semibold_font(22.0)),
        ]
        .into();
        style.spacing.item_spacing = egui::vec2(8.0, 8.0);
        style.spacing.button_padding = egui::vec2(12.0, 6.0);
        style.spacing.interact_size.y = 32.0;
        style.spacing.combo_height = 320.0;
        style.spacing.slider_width = 200.0;
        style.spacing.scroll = egui::style::ScrollStyle::thin();
        style.spacing.scroll.foreground_color = true;
        // a jump to a place in a list that just grew must stay there
        style.spacing.scroll.kinetic.rubber_band = false;
        style.spacing.scroll.dormant_handle_opacity = 0.3;
        style.spacing.scroll.active_handle_opacity = 0.45;
        style.spacing.scroll.interact_handle_opacity = 0.6;
        style.interaction.selectable_labels = true;
    });
}

/// The bold font in this size
pub fn semibold_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(SEMIBOLD.into()))
}

//! Colors, fonts and the look of the widgets.
//!
//! The values are those of the accepted draft (`docs/entwurf/assets/tokens.css`): grey surfaces with a calm blue,
//! Fira Sans and Fira Code, dense rows.

// the colors read as in the stylesheet
#![allow(clippy::unreadable_literal)]

use std::sync::Arc;

use eframe::egui::{
    self, Color32, Context, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Margin,
    Shadow, Stroke, TextStyle, Theme, Visuals,
};

/// Fira Sans 500
const MEDIUM: &str = "medium";
/// Fira Sans 600
const SEMIBOLD: &str = "semibold";
/// Fira Code 500
const MONO_MEDIUM: &str = "mono-medium";
/// Fira Code 600
const MONO_SEMIBOLD: &str = "mono-semibold";

/// Sizes of the text (`--fs-*`)
pub const FS_XS: f32 = 12.0;
pub const FS_SM: f32 = 13.0;
pub const FS_BASE: f32 = 14.0;
pub const FS_MD: f32 = 15.0;
pub const FS_LG: f32 = 18.0;
pub const FS_XL: f32 = 22.0;
pub const FS_2XL: f32 = 28.0;

/// Spacing (`--sp-1` to `--sp-6`)
pub const SP1: f32 = 4.0;
pub const SP2: f32 = 8.0;
pub const SP3: f32 = 12.0;
pub const SP4: f32 = 16.0;
pub const SP5: f32 = 24.0;
pub const SP6: f32 = 32.0;
/// Height of a table row
pub const ROW_H: f32 = 34.0;
/// Height of buttons and fields
pub const CTL_H: f32 = 32.0;
/// Height of the header
pub const HEADER_H: f32 = 48.0;
/// Height of the status bar
pub const STATUS_H: f32 = 26.0;
/// Rounding of cards, buttons and fields
pub const RADIUS: u8 = 6;
/// Rounding of small parts (tags, menu entries)
pub const RADIUS_SM: u8 = 4;

const fn rgb(hex: u32) -> Color32 {
    Color32::from_rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

/// A color with the given opacity in percent
const fn rgba(hex: u32, percent: u32) -> Color32 {
    let a = percent * 255 / 100;
    let r = ((hex >> 16) & 0xff) * a / 255;
    let g = ((hex >> 8) & 0xff) * a / 255;
    let b = (hex & 0xff) * a / 255;
    Color32::from_rgba_premultiplied(r as u8, g as u8, b as u8, a as u8)
}

/// Background colors of the packet list by the kind of packet (`--row-*`)
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
    pub voip: Color32,
}

/// All colors of one design, named as in the stylesheet of the draft
#[derive(Clone, Copy)]
pub struct Palette {
    pub bg: Color32,
    pub surface: Color32,
    pub surface_2: Color32,
    pub surface_3: Color32,
    pub hover: Color32,
    pub selected: Color32,
    pub border: Color32,
    pub border_strong: Color32,
    pub text: Color32,
    /// Secondary text (8.6:1 on white)
    pub text_2: Color32,
    /// Hints, headers of tables (5.3:1 on white)
    pub text_3: Color32,
    pub accent: Color32,
    pub accent_hover: Color32,
    pub accent_bg: Color32,
    pub accent_text: Color32,
    /// States: text color, tint and line; always with a symbol and text
    pub ok: Color32,
    pub ok_bg: Color32,
    pub ok_line: Color32,
    pub warn: Color32,
    pub warn_bg: Color32,
    pub warn_line: Color32,
    pub err: Color32,
    pub err_bg: Color32,
    pub err_line: Color32,
    pub off: Color32,
    pub off_bg: Color32,
    pub off_line: Color32,
    /// Charts: incoming blue, outgoing orange
    pub s1: Color32,
    pub s2: Color32,
    /// Follow stream: client and server
    pub cli: Color32,
    pub cli_bg: Color32,
    pub srv: Color32,
    pub srv_bg: Color32,
    /// The chosen layer in the hex dump
    pub hl: Color32,
    /// A favorite
    pub star: Color32,
    /// Shadow of menus and dialogs (`--shadow-2`)
    pub shadow: Color32,
    /// Shadow of a tile under the pointer (`--shadow-1`)
    pub shadow_soft: Color32,
    /// Behind a dialog (`--scrim`)
    pub scrim: Color32,
    pub rows: RowColors,
}

pub const LIGHT: Palette = Palette {
    bg: rgb(0xf4f5f7),
    surface: rgb(0xffffff),
    surface_2: rgb(0xf0f2f4),
    surface_3: rgb(0xe6e9ed),
    hover: rgb(0xeef1f4),
    selected: rgb(0xe7eef8),
    border: rgb(0xdde1e6),
    border_strong: rgb(0xc3cad2),
    text: rgb(0x15181c),
    text_2: rgb(0x444d59),
    text_3: rgb(0x636c78),
    accent: rgb(0x1c5ea8),
    accent_hover: rgb(0x164d8b),
    accent_bg: rgb(0xe6eef8),
    accent_text: rgb(0xffffff),
    ok: rgb(0x1d6b3c),
    ok_bg: rgb(0xe6f3ea),
    ok_line: rgb(0xa8d2b8),
    warn: rgb(0x7a5600),
    warn_bg: rgb(0xfbf3d9),
    warn_line: rgb(0xdcc275),
    err: rgb(0xa1132c),
    err_bg: rgb(0xfbe8eb),
    err_line: rgb(0xe7a3ae),
    off: rgb(0x46596d),
    off_bg: rgb(0xedf1f5),
    off_line: rgb(0xb5c2cf),
    s1: rgb(0x1c5ea8),
    s2: rgb(0xb45309),
    cli: rgb(0x174f8f),
    cli_bg: rgb(0xe8f0fa),
    srv: rgb(0x8a3c00),
    srv_bg: rgb(0xfbeee0),
    hl: rgb(0xc9dcf3),
    star: rgb(0xc98a00),
    shadow: rgba(0x101820, 16),
    shadow_soft: rgba(0x101820, 6),
    scrim: rgba(0x10141a, 38),
    rows: RowColors {
        bad: rgb(0xf9dfe3),
        tcp_control: rgb(0xeceef1),
        dns: rgb(0xe2ecf8),
        local: rgb(0xf8efd2),
        tls: rgb(0xece7f6),
        http: rgb(0xe1f1e6),
        icmp: rgb(0xf7e5ee),
        udp: rgb(0xdff0f1),
        voip: rgb(0xfbe4d2),
    },
};

pub const DARK: Palette = Palette {
    bg: rgb(0x101215),
    surface: rgb(0x171a1e),
    surface_2: rgb(0x1d2126),
    surface_3: rgb(0x262b31),
    hover: rgb(0x1f242a),
    selected: rgb(0x1b2838),
    border: rgb(0x2a2f36),
    border_strong: rgb(0x3a414a),
    text: rgb(0xe5e8ec),
    text_2: rgb(0xb0b8c2),
    text_3: rgb(0x8d96a1),
    accent: rgb(0x7aaeea),
    accent_hover: rgb(0x9cc3f0),
    accent_bg: rgb(0x19283a),
    accent_text: rgb(0x0d1622),
    ok: rgb(0x73d29a),
    ok_bg: rgb(0x15291e),
    ok_line: rgb(0x2f5d43),
    warn: rgb(0xe8c45f),
    warn_bg: rgb(0x2f2814),
    warn_line: rgb(0x6b5a26),
    err: rgb(0xff8d9d),
    err_bg: rgb(0x361920),
    err_line: rgb(0x7a3542),
    off: rgb(0xa3b5c8),
    off_bg: rgb(0x1f262e),
    off_line: rgb(0x44505d),
    s1: rgb(0x7aaeea),
    s2: rgb(0xf0a24f),
    cli: rgb(0x9cc3f0),
    cli_bg: rgb(0x19283a),
    srv: rgb(0xf5b77a),
    srv_bg: rgb(0x33240f),
    hl: rgb(0x24405f),
    star: rgb(0xe8c45f),
    shadow: rgba(0x000000, 50),
    shadow_soft: rgba(0x000000, 30),
    scrim: rgba(0x000000, 55),
    rows: RowColors {
        bad: rgb(0x3a1d24),
        tcp_control: rgb(0x23272d),
        dns: rgb(0x182a3e),
        local: rgb(0x2f2813),
        tls: rgb(0x262139),
        http: rgb(0x172a1f),
        icmp: rgb(0x321b2a),
        udp: rgb(0x142c2e),
        voip: rgb(0x3a2516),
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
            "voip" => Some(self.rows.voip),
            _ => None,
        }
    }

    /// `--shadow-1` of a tile under the pointer
    pub fn shadow_1(&self) -> Shadow {
        Shadow {
            offset: [0, 1],
            blur: 2,
            spread: 0,
            color: self.shadow_soft,
        }
    }

    /// `--shadow-2` of menus, pop-ups and dialogs
    pub fn shadow_2(&self) -> Shadow {
        Shadow {
            offset: [0, 8],
            blur: 28,
            spread: 0,
            color: self.shadow,
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
    v.window_stroke = Stroke::new(1.0, p.border);
    // dialogs round by 8, menus like cards
    v.window_corner_radius = CornerRadius::same(8);
    v.menu_corner_radius = CornerRadius::same(RADIUS);
    v.window_shadow = p.shadow_2();
    v.popup_shadow = p.shadow_2();
    v.extreme_bg_color = p.surface;
    v.text_edit_bg_color = Some(p.surface);
    v.faint_bg_color = p.surface_2;
    v.code_bg_color = p.surface_2;
    v.hyperlink_color = p.accent;
    v.warn_fg_color = p.warn;
    v.error_fg_color = p.err;
    // marked text; the focus of a field is drawn in the accent like in the draft
    v.selection.bg_fill = p.hl;
    v.selection.stroke = Stroke::new(1.0, p.accent);
    v.text_cursor.stroke = Stroke::new(2.0, p.accent);

    let radius = CornerRadius::same(RADIUS);
    let w = &mut v.widgets;
    w.noninteractive.bg_fill = p.surface;
    w.noninteractive.weak_bg_fill = p.surface;
    w.noninteractive.bg_stroke = Stroke::new(1.0, p.border);
    w.noninteractive.fg_stroke = Stroke::new(1.0, p.text);
    w.noninteractive.corner_radius = radius;
    // `.btn`: white with a strong border, on hover only the background changes
    for (state, fill, border) in [
        (&mut w.inactive, p.surface, p.border_strong),
        (&mut w.hovered, p.hover, p.border_strong),
        (&mut w.active, p.surface_3, p.border_strong),
        (&mut w.open, p.surface, p.accent),
    ] {
        state.bg_fill = fill;
        state.weak_bg_fill = fill;
        state.bg_stroke = Stroke::new(1.0, border);
        state.fg_stroke = Stroke::new(1.0, p.text);
        state.corner_radius = radius;
        state.expansion = 0.0;
    }
    // the rail of a slider and the box of a checkbox stand out from the card
    w.inactive.bg_fill = p.surface_3;
    v.handle_shape = egui::style::HandleShape::Circle;
    v.slider_trailing_fill = true;
    v
}

/// One font file, built in
fn font(fonts: &mut FontDefinitions, name: &str, bytes: &'static [u8]) {
    fonts.font_data.insert(
        name.to_owned(),
        Arc::new(FontData::from_owned(without_ligatures(bytes))),
    );
}

/// The font without ligatures, as the draft sets it (`font-variant-ligatures: none`): egui shapes
/// text with harfrust, and Fira Code would turn `0x` into `×` and `==` into one sign. Renames
/// these features in the table of substitutions (GSUB), so the shaper does not find them; the
/// file itself stays as it is.
pub(crate) fn without_ligatures(bytes: &[u8]) -> Vec<u8> {
    const OFF: [&[u8; 4]; 4] = [b"liga", b"clig", b"dlig", b"calt"];
    let mut font = bytes.to_vec();
    let u16_at = |f: &[u8], at: usize| {
        f.get(at..at + 2)
            .map(|b| usize::from(u16::from_be_bytes([b[0], b[1]])))
    };
    let u32_at = |f: &[u8], at: usize| {
        f.get(at..at + 4)
            .and_then(|b| usize::try_from(u32::from_be_bytes([b[0], b[1], b[2], b[3]])).ok())
    };
    let tables = u16_at(&font, 4).unwrap_or(0);
    let gsub = (0..tables)
        .map(|i| 12 + i * 16)
        .find(|record| font.get(*record..*record + 4) == Some(b"GSUB".as_slice()))
        .and_then(|record| u32_at(&font, record + 8));
    let Some(gsub) = gsub else {
        return font;
    };
    let Some(list) = u16_at(&font, gsub + 6).map(|offset| gsub + offset) else {
        return font;
    };
    for i in 0..u16_at(&font, list).unwrap_or(0) {
        let at = list + 2 + i * 6;
        if let Some(tag) = font.get_mut(at..at + 4)
            && OFF.iter().any(|off| tag == off.as_slice())
        {
            tag[0] = b'_';
        }
    }
    font
}

/// Sets fonts, colors and spacing once, for both designs
pub fn install(ctx: &Context) {
    let mut fonts = FontDefinitions::default();
    font(
        &mut fonts,
        "FiraSans-Regular",
        include_bytes!("../../resources/fonts/FiraSans-Regular.ttf"),
    );
    font(
        &mut fonts,
        "FiraSans-Medium",
        include_bytes!("../../resources/fonts/FiraSans-Medium.ttf"),
    );
    font(
        &mut fonts,
        "FiraSans-SemiBold",
        include_bytes!("../../resources/fonts/FiraSans-SemiBold.ttf"),
    );
    font(
        &mut fonts,
        "FiraCode-Regular",
        include_bytes!("../../resources/fonts/FiraCode-Regular.ttf"),
    );
    font(
        &mut fonts,
        "FiraCode-Medium",
        include_bytes!("../../resources/fonts/FiraCode-Medium.ttf"),
    );
    font(
        &mut fonts,
        "FiraCode-SemiBold",
        include_bytes!("../../resources/fonts/FiraCode-SemiBold.ttf"),
    );
    // what Fira Sans lacks (such as ↔) comes from Fira Code, the rest from the fonts egui builds in
    // (symbols, emoji)
    let mut sans = vec!["FiraCode-Regular".to_owned()];
    sans.extend(
        fonts
            .families
            .get(&FontFamily::Proportional)
            .cloned()
            .unwrap_or_default(),
    );
    let mono = fonts
        .families
        .get(&FontFamily::Monospace)
        .cloned()
        .unwrap_or_default();
    let family = |first: &str, rest: &[String]| {
        let mut list = vec![first.to_owned()];
        list.extend(rest.iter().cloned());
        list
    };
    fonts
        .families
        .insert(FontFamily::Proportional, family("FiraSans-Regular", &sans));
    fonts.families.insert(
        FontFamily::Name(MEDIUM.into()),
        family("FiraSans-Medium", &sans),
    );
    fonts.families.insert(
        FontFamily::Name(SEMIBOLD.into()),
        family("FiraSans-SemiBold", &sans),
    );
    fonts
        .families
        .insert(FontFamily::Monospace, family("FiraCode-Regular", &mono));
    fonts.families.insert(
        FontFamily::Name(MONO_MEDIUM.into()),
        family("FiraCode-Medium", &mono),
    );
    fonts.families.insert(
        FontFamily::Name(MONO_SEMIBOLD.into()),
        family("FiraCode-SemiBold", &mono),
    );
    ctx.set_fonts(fonts);
    // packets can carry any character: what no font has is drawn as a box
    ctx.set_missing_glyph_policy(egui::MissingGlyphPolicy::Tofu);

    ctx.set_visuals_of(Theme::Light, visuals(&LIGHT, false));
    ctx.set_visuals_of(Theme::Dark, visuals(&DARK, true));
    ctx.all_styles_mut(|style| {
        style.text_styles = [
            (TextStyle::Small, FontId::proportional(FS_XS)),
            (TextStyle::Body, FontId::proportional(FS_BASE)),
            (TextStyle::Button, medium_font(FS_BASE)),
            (TextStyle::Monospace, FontId::monospace(FS_SM)),
            (TextStyle::Heading, semibold_font(FS_XL)),
        ]
        .into();
        style.spacing.item_spacing = egui::vec2(SP2, SP2);
        style.spacing.button_padding = egui::vec2(SP3, 6.0);
        style.spacing.interact_size.y = CTL_H;
        style.spacing.icon_width = 16.0;
        style.spacing.icon_width_inner = 8.0;
        style.spacing.icon_spacing = SP2;
        style.spacing.menu_margin = Margin::same(SP1 as i8);
        style.spacing.window_margin = Margin::same(SP5 as i8);
        style.spacing.combo_height = 320.0;
        style.spacing.slider_width = 200.0;
        style.spacing.scroll = egui::style::ScrollStyle::thin();
        style.spacing.scroll.foreground_color = true;
        // the draft has no fading at the edges of scrolled content
        style.spacing.scroll.fade.strength = 0.0;
        // a jump to a place in a list that just grew must stay there
        style.spacing.scroll.kinetic.rubber_band = false;
        style.spacing.scroll.dormant_handle_opacity = 0.3;
        style.spacing.scroll.active_handle_opacity = 0.45;
        style.spacing.scroll.interact_handle_opacity = 0.6;
        style.interaction.selectable_labels = true;
    });
}

/// Fira Sans 500 in this size (`font-weight: 500`)
pub fn medium_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(MEDIUM.into()))
}

/// Fira Sans 600 in this size (headings)
pub fn semibold_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(SEMIBOLD.into()))
}

/// Fira Code 400 in this size (addresses, times, hex, filters)
pub fn mono_font(size: f32) -> FontId {
    FontId::monospace(size)
}

/// Fira Code 500 in this size (numbers in bold, the pager)
pub fn mono_medium_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(MONO_MEDIUM.into()))
}

/// Fira Code 600 in this size (the values of the tiles)
pub fn mono_semibold_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(MONO_SEMIBOLD.into()))
}

#[cfg(test)]
mod tests {
    use super::without_ligatures;

    /// The tags of the features in the table of substitutions
    fn features(font: &[u8]) -> Vec<String> {
        let at16 = |at: usize| usize::from(u16::from_be_bytes([font[at], font[at + 1]]));
        let tables = at16(4);
        let record = (0..tables)
            .map(|i| 12 + i * 16)
            .find(|r| &font[*r..*r + 4] == b"GSUB")
            .unwrap();
        let gsub = u32::from_be_bytes(font[record + 8..record + 12].try_into().unwrap()) as usize;
        let list = gsub + at16(gsub + 6);
        (0..at16(list))
            .map(|i| {
                String::from_utf8_lossy(&font[list + 2 + i * 6..list + 6 + i * 6]).into_owned()
            })
            .collect()
    }

    #[test]
    fn the_fonts_have_no_ligatures() {
        for font in [
            &include_bytes!("../../resources/fonts/FiraCode-Regular.ttf")[..],
            &include_bytes!("../../resources/fonts/FiraSans-Regular.ttf")[..],
        ] {
            let before = features(font);
            assert!(
                before.iter().any(|f| f == "liga" || f == "calt"),
                "{before:?}"
            );
            let after = features(&without_ligatures(font));
            assert_eq!(after.len(), before.len());
            for off in ["liga", "clig", "dlig", "calt"] {
                assert!(!after.iter().any(|f| f == off), "{off} in {after:?}");
            }
            // the rest stays, such as the kerning of Fira Sans
            assert!(
                after
                    .iter()
                    .all(|f| before.contains(f) || f.starts_with('_'))
            );
        }
        // anything else passes as it is
        assert_eq!(without_ligatures(b"not a font"), b"not a font");
    }
}

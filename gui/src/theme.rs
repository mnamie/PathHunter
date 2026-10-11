//! Colors, spacing, and typography. Dark is the default; light mirrors it.

use eframe::egui::{
    self, Color32, CornerRadius, FontId, Stroke, TextStyle, Theme, Visuals, epaint::Shadow,
};

use crate::app::Kind;

#[derive(Clone, Copy)]
pub struct Palette {
    pub bg: Color32,
    pub panel: Color32,
    pub card: Color32,
    pub border: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub accent: Color32,
    pub ok: Color32,
    pub symlink: Color32,
    pub duplicate: Color32,
    pub dead: Color32,
    pub empty: Color32,
}

const DARK: Palette = Palette {
    bg: Color32::from_rgb(0x0f, 0x11, 0x15),
    panel: Color32::from_rgb(0x15, 0x18, 0x1d),
    card: Color32::from_rgb(0x1b, 0x1f, 0x26),
    border: Color32::from_rgb(0x2a, 0x2f, 0x38),
    text: Color32::from_rgb(0xe6, 0xe8, 0xeb),
    muted: Color32::from_rgb(0x8b, 0x93, 0xa1),
    accent: Color32::from_rgb(0x22, 0xd3, 0xee),
    ok: Color32::from_rgb(0x34, 0xd3, 0x99),
    symlink: Color32::from_rgb(0x38, 0xbd, 0xf8),
    duplicate: Color32::from_rgb(0xfb, 0xbf, 0x24),
    dead: Color32::from_rgb(0xf8, 0x71, 0x71),
    empty: Color32::from_rgb(0xe8, 0x79, 0xf9),
};

const LIGHT: Palette = Palette {
    bg: Color32::from_rgb(0xf6, 0xf7, 0xf9),
    panel: Color32::from_rgb(0xee, 0xf0, 0xf3),
    card: Color32::from_rgb(0xff, 0xff, 0xff),
    border: Color32::from_rgb(0xdd, 0xe1, 0xe6),
    text: Color32::from_rgb(0x1a, 0x1d, 0x21),
    muted: Color32::from_rgb(0x6b, 0x72, 0x80),
    accent: Color32::from_rgb(0x08, 0x91, 0xb2),
    ok: Color32::from_rgb(0x05, 0x96, 0x69),
    symlink: Color32::from_rgb(0x02, 0x84, 0xc7),
    duplicate: Color32::from_rgb(0xb4, 0x53, 0x09),
    dead: Color32::from_rgb(0xdc, 0x26, 0x26),
    empty: Color32::from_rgb(0xc0, 0x26, 0xd3),
};

impl Palette {
    pub fn of(ctx: &egui::Context) -> Palette {
        match ctx.theme() {
            Theme::Dark => DARK,
            Theme::Light => LIGHT,
        }
    }

    pub fn kind(&self, kind: Kind) -> Color32 {
        match kind {
            Kind::Ok => self.ok,
            Kind::Symlink => self.symlink,
            Kind::Duplicate => self.duplicate,
            Kind::Dead => self.dead,
            Kind::Empty => self.empty,
        }
    }
}

/// Translucent tint of `c`, for pill/badge backgrounds.
pub fn tint(c: Color32) -> Color32 {
    c.gamma_multiply(0.16)
}

fn visuals(p: &Palette, base: Visuals) -> Visuals {
    let mut v = base;
    v.panel_fill = p.bg;
    v.window_fill = p.card;
    v.window_stroke = Stroke::new(1.0, p.border);
    v.window_corner_radius = CornerRadius::same(12);
    v.window_shadow = Shadow {
        offset: [0, 8],
        blur: 24,
        spread: 0,
        color: Color32::from_black_alpha(80),
    };
    v.menu_corner_radius = CornerRadius::same(8);
    v.extreme_bg_color = p.card;
    v.faint_bg_color = p.panel;
    v.code_bg_color = p.panel;
    v.hyperlink_color = p.accent;
    v.override_text_color = Some(p.text);
    v.selection.bg_fill = p.accent.gamma_multiply(0.35);
    v.selection.stroke = Stroke::new(1.0, p.accent);

    let w = &mut v.widgets;
    w.noninteractive.bg_stroke = Stroke::new(1.0, p.border);
    w.noninteractive.fg_stroke = Stroke::new(1.0, p.text);
    for (state, fill) in [
        (&mut w.inactive, p.card),
        (&mut w.hovered, p.border),
        (&mut w.active, p.border),
        (&mut w.open, p.card),
    ] {
        state.weak_bg_fill = fill;
        state.bg_fill = fill;
        state.corner_radius = CornerRadius::same(8);
    }
    // bg_fill is the checkbox/radio box; keep it distinct from cards.
    w.inactive.bg_fill = p.bg;
    w.hovered.bg_fill = p.panel;
    w.inactive.bg_stroke = Stroke::new(1.0, p.muted.gamma_multiply(0.6));
    w.inactive.fg_stroke = Stroke::new(1.0, p.text);
    w.hovered.bg_stroke = Stroke::new(1.0, p.accent.gamma_multiply(0.6));
    w.active.bg_stroke = Stroke::new(1.0, p.accent);
    v
}

pub fn install(ctx: &egui::Context) {
    ctx.set_visuals_of(Theme::Dark, visuals(&DARK, Visuals::dark()));
    ctx.set_visuals_of(Theme::Light, visuals(&LIGHT, Visuals::light()));
    ctx.all_styles_mut(|style| {
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.button_padding = egui::vec2(12.0, 6.0);
        style.spacing.interact_size.y = 28.0;
        use egui::FontFamily::{Monospace, Proportional};
        style.text_styles = [
            (TextStyle::Heading, FontId::new(22.0, Proportional)),
            (TextStyle::Body, FontId::new(14.0, Proportional)),
            (TextStyle::Button, FontId::new(14.0, Proportional)),
            (TextStyle::Monospace, FontId::new(13.5, Monospace)),
            (TextStyle::Small, FontId::new(11.5, Proportional)),
        ]
        .into();
    });
    ctx.set_theme(Theme::Dark);
}

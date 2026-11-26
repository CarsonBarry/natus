use eframe::egui::{self, Color32, CornerRadius, FontFamily, FontId, Stroke, TextStyle};

#[derive(Clone, Copy)]
pub struct Palette {
    pub dark: bool,
    pub bg: Color32,
    pub panel: Color32,
    pub panel2: Color32,
    pub panel3: Color32,
    pub border: Color32,
    pub text: Color32,
    pub dim: Color32,
    pub faint: Color32,
    pub accent: Color32,
    pub accent_dim: Color32,
    pub accent_ink: Color32,
    pub teal: Color32,
    pub warn: Color32,
    pub del: Color32,
    pub add: Color32,
}

pub fn palette(dark: bool) -> Palette {
    if dark {
        Palette {
            dark,
            bg: Color32::from_rgb(0, 0, 0),
            panel: Color32::from_rgb(10, 10, 12),
            panel2: Color32::from_rgb(18, 18, 22),
            panel3: Color32::from_rgb(26, 26, 32),
            border: Color32::from_rgb(30, 30, 37),
            text: Color32::from_rgb(231, 231, 234),
            dim: Color32::from_rgb(154, 154, 162),
            faint: Color32::from_rgb(95, 95, 104),
            accent: Color32::from_rgb(124, 140, 255),
            accent_dim: Color32::from_rgb(36, 40, 76),
            accent_ink: Color32::from_rgb(10, 10, 14),
            teal: Color32::from_rgb(78, 201, 176),
            warn: Color32::from_rgb(210, 153, 34),
            del: Color32::from_rgb(248, 81, 73),
            add: Color32::from_rgb(46, 160, 67),
        }
    } else {
        Palette {
            dark,
            bg: Color32::from_rgb(255, 255, 255),
            panel: Color32::from_rgb(245, 245, 247),
            panel2: Color32::from_rgb(236, 236, 240),
            panel3: Color32::from_rgb(224, 224, 230),
            border: Color32::from_rgb(222, 222, 230),
            text: Color32::from_rgb(28, 28, 33),
            dim: Color32::from_rgb(82, 82, 92),
            faint: Color32::from_rgb(128, 128, 140),
            accent: Color32::from_rgb(67, 83, 214),
            accent_dim: Color32::from_rgb(223, 226, 252),
            accent_ink: Color32::from_rgb(255, 255, 255),
            teal: Color32::from_rgb(11, 143, 120),
            warn: Color32::from_rgb(154, 103, 0),
            del: Color32::from_rgb(207, 34, 46),
            add: Color32::from_rgb(26, 127, 55),
        }
    }
}

pub fn apply(ctx: &egui::Context, p: Palette) {
    ctx.data_mut(|d| d.insert_temp(egui::Id::NULL, p));
    let mut v = if p.dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    v.panel_fill = p.panel;
    v.window_fill = p.panel;
    v.extreme_bg_color = p.bg;
    v.faint_bg_color = p.panel2;
    v.code_bg_color = p.panel2;
    v.override_text_color = None;
    v.hyperlink_color = p.accent;
    v.warn_fg_color = p.warn;
    v.error_fg_color = p.del;
    v.selection.bg_fill = p.accent_dim;
    v.selection.stroke = Stroke::new(1.0, p.accent);
    v.window_stroke = Stroke::new(1.0, p.border);
    v.window_corner_radius = CornerRadius::same(8);
    v.menu_corner_radius = CornerRadius::same(6);
    let r = CornerRadius::same(5);
    let w = &mut v.widgets;
    w.noninteractive.bg_fill = p.panel;
    w.noninteractive.weak_bg_fill = p.panel;
    w.noninteractive.bg_stroke = Stroke::new(1.0, p.border);
    w.noninteractive.fg_stroke = Stroke::new(1.0, p.text);
    w.noninteractive.corner_radius = r;
    w.inactive.bg_fill = p.panel2;
    w.inactive.weak_bg_fill = p.panel2;
    w.inactive.bg_stroke = Stroke::NONE;
    w.inactive.fg_stroke = Stroke::new(1.0, p.text);
    w.inactive.corner_radius = r;
    w.hovered.bg_fill = p.panel3;
    w.hovered.weak_bg_fill = p.panel3;
    w.hovered.bg_stroke = Stroke::NONE;
    w.hovered.fg_stroke = Stroke::new(1.0, p.text);
    w.hovered.corner_radius = r;
    w.hovered.expansion = 0.0;
    w.active.bg_fill = p.accent_dim;
    w.active.weak_bg_fill = p.accent_dim;
    w.active.bg_stroke = Stroke::new(1.0, p.accent);
    w.active.fg_stroke = Stroke::new(1.0, p.text);
    w.active.corner_radius = r;
    w.active.expansion = 0.0;
    w.open.bg_fill = p.panel3;
    w.open.weak_bg_fill = p.panel3;
    w.open.bg_stroke = Stroke::NONE;
    w.open.corner_radius = r;
    ctx.set_visuals(v);
    ctx.style_mut(|s| {
        s.spacing.item_spacing = egui::vec2(8.0, 6.0);
        s.spacing.button_padding = egui::vec2(10.0, 4.0);
        s.spacing.interact_size.y = 24.0;
        s.spacing.menu_margin = egui::Margin::same(6);
        s.spacing.scroll.bar_width = 6.0;
        s.spacing.scroll.floating = true;
        s.text_styles = [
            (
                TextStyle::Heading,
                FontId::new(17.0, FontFamily::Proportional),
            ),
            (TextStyle::Body, FontId::new(13.5, FontFamily::Proportional)),
            (
                TextStyle::Button,
                FontId::new(13.0, FontFamily::Proportional),
            ),
            (
                TextStyle::Small,
                FontId::new(11.5, FontFamily::Proportional),
            ),
            (
                TextStyle::Monospace,
                FontId::new(12.5, FontFamily::Monospace),
            ),
        ]
        .into();
    });
}

pub fn pal(ctx: &egui::Context) -> Palette {
    ctx.data(|d| d.get_temp::<Palette>(egui::Id::NULL))
        .unwrap_or_else(|| palette(true))
}

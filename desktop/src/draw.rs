use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontId, Painter, Pos2, Rect, Stroke, StrokeKind, Vec2,
};

pub const BG: Color32 = Color32::from_rgb(7, 8, 10);
pub const SURFACE: Color32 = Color32::from_rgb(14, 16, 21);
pub const HOVER: Color32 = Color32::from_rgb(20, 24, 31);
pub const BORDER: Color32 = Color32::from_rgb(26, 30, 38);
pub const BORDER_STRONG: Color32 = Color32::from_rgb(39, 45, 56);
pub const TEXT: Color32 = Color32::from_rgb(229, 232, 239);
pub const MUTED: Color32 = Color32::from_rgb(130, 138, 152);
pub const FAINT: Color32 = Color32::from_rgb(75, 82, 94);
pub const GREEN: Color32 = Color32::from_rgb(74, 214, 150);
pub const RED: Color32 = Color32::from_rgb(226, 96, 82);
pub const AMBER: Color32 = Color32::from_rgb(232, 186, 92);
pub const BLUE: Color32 = Color32::from_rgb(110, 164, 245);
pub const CYAN: Color32 = Color32::from_rgb(110, 214, 214);
pub const PURPLE: Color32 = Color32::from_rgb(186, 146, 245);

pub fn status_color(status: dollar_dashboard::model::Status) -> Color32 {
    use dollar_dashboard::model::Status::*;
    match status {
        Calm => GREEN,
        Neutral => MUTED,
        Elevated => AMBER,
        Stressed => RED,
    }
}

pub fn label(
    painter: &Painter,
    pos: Pos2,
    align: Align2,
    text: &str,
    size: f32,
    color: Color32,
    mono: bool,
) {
    let font = if mono {
        FontId::monospace(size)
    } else {
        FontId::proportional(size)
    };
    painter.text(pos, align, text, font, color);
}

pub fn dot(painter: &Painter, center: Pos2, color: Color32) {
    painter.circle_filled(center, 3.5, color);
}

pub fn panel(painter: &Painter, rect: Rect, accent: Option<Color32>) {
    painter.rect_filled(rect, CornerRadius::ZERO, SURFACE);
    painter.rect_stroke(
        rect,
        CornerRadius::ZERO,
        Stroke::new(1.0, BORDER_STRONG),
        StrokeKind::Inside,
    );
    if let Some(color) = accent {
        painter.rect_filled(
            Rect::from_min_size(rect.left_top(), Vec2::new(3.0, rect.height())),
            CornerRadius::ZERO,
            color,
        );
    }
}

pub fn sparkline(painter: &Painter, rect: Rect, values: &[f64], color: Color32) {
    let values: Vec<f64> = values.iter().copied().filter(|v| v.is_finite()).collect();
    if values.len() < 2 || rect.width() < 4.0 || rect.height() < 4.0 {
        return;
    }
    let min = values.iter().copied().fold(f64::INFINITY, f64::min);
    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let span = (max - min).max(1e-9);
    let mut previous = None;
    for (index, value) in values.iter().enumerate() {
        let t = index as f32 / (values.len() - 1) as f32;
        let point = pos(rect, t, ((*value - min) / span) as f32);
        if let Some(last) = previous {
            painter.line_segment([last, point], Stroke::new(1.5, color));
        }
        previous = Some(point);
    }
}

pub fn curve(painter: &Painter, rect: Rect, points: &[(String, f64)]) {
    if points.len() < 2 {
        label(
            painter,
            rect.center(),
            Align2::CENTER_CENTER,
            "Yield curve loads with the Treasury feed",
            12.0,
            MUTED,
            true,
        );
        return;
    }
    let plot = rect.shrink2(Vec2::new(8.0, 4.0));
    let plot = Rect::from_min_max(plot.left_top(), pos2(plot.right(), plot.bottom() - 18.0));
    let min = points.iter().map(|(_, y)| *y).fold(f64::INFINITY, f64::min) - 0.15;
    let max = points
        .iter()
        .map(|(_, y)| *y)
        .fold(f64::NEG_INFINITY, f64::max)
        + 0.15;
    let span = (max - min).max(0.2);
    let mut previous = None;
    for (index, (name, value)) in points.iter().enumerate() {
        let t = if points.len() == 1 {
            0.5
        } else {
            index as f32 / (points.len() - 1) as f32
        };
        let y = ((*value - min) / span) as f32;
        let point = pos(plot, t, y);
        if let Some(last) = previous {
            painter.line_segment([last, point], Stroke::new(1.8, BLUE));
        }
        painter.circle_filled(point, 3.0, BLUE);
        label(
            painter,
            pos2(point.x, rect.bottom() - 2.0),
            Align2::CENTER_BOTTOM,
            name,
            10.0,
            FAINT,
            true,
        );
        label(
            painter,
            point + Vec2::new(0.0, -10.0),
            Align2::CENTER_BOTTOM,
            &format!("{value:.2}"),
            10.0,
            TEXT,
            true,
        );
        previous = Some(point);
    }
}

pub fn power_law(painter: &Painter, rect: Rect, samples: &[(f64, f64, f64)]) {
    if samples.len() < 2 {
        label(
            painter,
            rect.center(),
            Align2::CENTER_CENTER,
            "Power-law history loads on the first refresh",
            12.0,
            MUTED,
            true,
        );
        return;
    }
    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;
    for (_, price, fair) in samples {
        for value in [*price, *fair] {
            if value > 0.0 {
                min = min.min(value.log10());
                max = max.max(value.log10());
            }
        }
    }
    if !min.is_finite() || max - min < 1e-6 {
        return;
    }
    min -= 0.05;
    max += 0.05;
    let span = max - min;
    let mut price_prev = None;
    let mut fair_prev = None;
    for (index, (_, price, fair)) in samples.iter().enumerate() {
        let t = index as f32 / (samples.len() - 1) as f32;
        if *price > 0.0 {
            let point = pos(rect, t, ((price.log10() - min) / span) as f32);
            if let Some(last) = price_prev {
                painter.line_segment([last, point], Stroke::new(1.6, AMBER));
            }
            price_prev = Some(point);
        }
        if *fair > 0.0 {
            let point = pos(rect, t, ((fair.log10() - min) / span) as f32);
            if let Some(last) = fair_prev {
                painter.line_segment([last, point], Stroke::new(1.2, FAINT));
            }
            fair_prev = Some(point);
        }
    }
}

fn pos(rect: Rect, t: f32, y: f32) -> Pos2 {
    pos2(
        rect.left() + rect.width() * t.clamp(0.0, 1.0),
        rect.bottom() - rect.height() * y.clamp(0.0, 1.0),
    )
}

fn pos2(x: f32, y: f32) -> Pos2 {
    Pos2::new(x, y)
}

pub fn truncate(text: &str, limit: usize) -> String {
    let count = text.chars().count();
    if count <= limit {
        text.to_string()
    } else {
        let trimmed: String = text.chars().take(limit.saturating_sub(1)).collect();
        format!("{trimmed}…")
    }
}

pub fn apply_style(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = BG;
    visuals.window_fill = BG;
    visuals.extreme_bg_color = SURFACE;
    visuals.faint_bg_color = SURFACE;
    visuals.code_bg_color = SURFACE;
    visuals.override_text_color = Some(TEXT);
    visuals.window_corner_radius = CornerRadius::ZERO;
    visuals.menu_corner_radius = CornerRadius::ZERO;
    visuals.hyperlink_color = BLUE;
    for widget in [
        &mut visuals.widgets.noninteractive,
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        widget.corner_radius = CornerRadius::ZERO;
        widget.bg_stroke = Stroke::new(1.0, BORDER);
        widget.fg_stroke = Stroke::new(1.0, TEXT);
    }
    visuals.widgets.noninteractive.bg_fill = BG;
    visuals.widgets.inactive.bg_fill = SURFACE;
    visuals.widgets.hovered.bg_fill = HOVER;
    visuals.widgets.active.bg_fill = HOVER;
    visuals.selection.bg_fill = HOVER;
    visuals.selection.stroke = Stroke::new(1.0, BLUE);
    ctx.set_visuals(visuals);
    ctx.style_mut_of(egui::Theme::Dark, |style| {
        style.spacing.item_spacing = Vec2::new(8.0, 6.0);
        style.spacing.button_padding = Vec2::new(10.0, 6.0);
    });
}

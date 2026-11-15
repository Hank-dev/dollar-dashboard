//! Building blocks shared by every desk: panels, headers, stat grids and tables.

use crate::theme::{
    self, delta_color, medium, mono, sans, BORDER, FAINT, GAP, HOVER, MUTED, RADIUS, RAISED,
    SURFACE, TEXT,
};
use dollar_dashboard::model::Delta;
use eframe::egui::{
    self,
    text::{LayoutJob, TextFormat, TextWrapping},
    Align, Align2, Color32, CursorIcon, FontId, Frame, Galley, Margin, Painter, Pos2, Rect,
    Response, Sense, Stroke, StrokeKind, Ui, Vec2,
};
use std::sync::Arc;

/// Uppercase label with a little tracking, for section and column headers.
pub fn caps_job(text: &str, size: f32, color: Color32) -> LayoutJob {
    let mut job = LayoutJob::default();
    job.append(
        &text.to_uppercase(),
        0.0,
        TextFormat {
            font_id: sans(size),
            color,
            extra_letter_spacing: 0.6,
            ..Default::default()
        },
    );
    job
}

pub fn caps(ui: &mut Ui, text: &str, color: Color32) -> Response {
    ui.label(caps_job(text, 10.5, color))
}

/// Single-line galley elided with an ellipsis to fit `width`.
pub fn fit_galley(
    painter: &Painter,
    text: &str,
    font: FontId,
    color: Color32,
    width: f32,
) -> Arc<Galley> {
    let mut job = LayoutJob::single_section(
        text.to_string(),
        TextFormat {
            font_id: font,
            color,
            ..Default::default()
        },
    );
    job.wrap = TextWrapping {
        max_width: width.max(1.0),
        max_rows: 1,
        break_anywhere: true,
        overflow_character: Some('…'),
    };
    painter.layout_job(job)
}

/// Paint text inside `rect`, vertically centred, elided to its width.
pub fn cell_text(
    painter: &Painter,
    rect: Rect,
    text: &str,
    font: FontId,
    color: Color32,
    align: Align,
) {
    let galley = fit_galley(painter, text, font, color, rect.width());
    let x = match align {
        Align::Min => rect.left(),
        Align::Center => rect.center().x - galley.size().x / 2.0,
        Align::Max => rect.right() - galley.size().x,
    };
    let pos = Pos2::new(x, rect.center().y - galley.size().y / 2.0);
    painter.galley(pos, galley, color);
}

pub fn panel_frame() -> Frame {
    Frame::new()
        .fill(SURFACE())
        .stroke(Stroke::new(1.0, BORDER()))
        .corner_radius(RADIUS())
        .inner_margin(Margin::symmetric(14, 12))
}

/// A bordered panel with a title row; `meta` sits right-aligned in the title row.
pub fn panel<R>(ui: &mut Ui, title: &str, meta: &str, add: impl FnOnce(&mut Ui) -> R) -> R {
    panel_frame()
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            panel_title(ui, title, meta);
            ui.add_space(8.0);
            add(ui)
        })
        .inner
}

pub fn panel_title(ui: &mut Ui, title: &str, meta: &str) {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 18.0), Sense::hover());
    let painter = ui.painter();
    let title_galley = painter.layout_no_wrap(title.to_string(), medium(13.5), TEXT());
    let title_w = title_galley.size().x;
    painter.galley(
        Pos2::new(rect.left(), rect.center().y - title_galley.size().y / 2.0),
        title_galley,
        TEXT(),
    );
    if !meta.is_empty() {
        let room = (width - title_w - 16.0).max(0.0);
        let meta_rect = Rect::from_min_max(Pos2::new(rect.right() - room, rect.top()), rect.max);
        cell_text(painter, meta_rect, meta, sans(11.0), FAINT(), Align::Max);
    }
}

/// Small section label with a hairline under it.
pub fn section(ui: &mut Ui, title: &str, meta: &str) {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 20.0), Sense::hover());
    let painter = ui.painter();
    let galley = painter.layout_job(caps_job(title, 10.5, FAINT()));
    let title_w = galley.size().x;
    painter.galley(
        Pos2::new(rect.left(), rect.center().y - galley.size().y / 2.0),
        galley,
        FAINT(),
    );
    if !meta.is_empty() {
        let meta_rect = Rect::from_min_max(
            Pos2::new(rect.left() + title_w + 16.0, rect.top()),
            rect.max,
        );
        cell_text(painter, meta_rect, meta, sans(11.0), FAINT(), Align::Max);
    }
    painter.hline(rect.x_range(), rect.bottom(), Stroke::new(1.0, BORDER()));
    ui.add_space(6.0);
}

/// Desk title block: accent kicker, title, and one line of provenance.
pub fn desk_header(ui: &mut Ui, kicker: &str, accent: Color32, title: &str, meta: &str) {
    desk_header_tone(ui, kicker, accent, title, meta, FAINT());
}

/// Desk title block whose provenance line takes a colour, e.g. amber when stale.
pub fn desk_header_tone(
    ui: &mut Ui,
    kicker: &str,
    accent: Color32,
    title: &str,
    meta: &str,
    meta_color: Color32,
) {
    ui.label(caps_job(kicker, 10.5, accent));
    ui.add_space(2.0);
    ui.label(egui::RichText::new(title).font(medium(21.0)).color(TEXT()));
    if !meta.is_empty() {
        ui.label(egui::RichText::new(meta).font(sans(11.5)).color(meta_color));
    }
    ui.add_space(GAP);
}

/// A paragraph with an accent rule on the left, for verdicts and readouts.
pub fn quote_block(ui: &mut Ui, accent: Color32, label: &str, body: &str) {
    let response = Frame::new()
        .inner_margin(Margin {
            left: 14,
            right: 4,
            top: 2,
            bottom: 2,
        })
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            if !label.is_empty() {
                ui.label(caps_job(label, 10.0, accent));
                ui.add_space(2.0);
            }
            ui.label(egui::RichText::new(body).font(sans(13.5)).color(TEXT()));
        })
        .response;
    let rect = response.rect;
    ui.painter().rect_filled(
        Rect::from_min_size(rect.left_top(), Vec2::new(2.0, rect.height())),
        crate::theme::corner(1),
        accent,
    );
}

/// A short research verdict, with the full dated narrative available on demand.
pub fn research_summary(ui: &mut Ui, id: &str, accent: Color32, body: &str) {
    let first = body
        .split_once(". ")
        .map(|(s, _)| format!("{s}."))
        .unwrap_or_else(|| body.to_string());
    let summary = if first.chars().count() > 220 {
        format!("{}…", first.chars().take(217).collect::<String>())
    } else {
        first
    };
    quote_block(ui, accent, "Snapshot verdict", &summary);
    if summary != body {
        egui::CollapsingHeader::new(
            egui::RichText::new("Read the full research note")
                .font(sans(11.5))
                .color(MUTED()),
        )
        .id_salt(id)
        .default_open(false)
        .show(ui, |ui| {
            ui.label(egui::RichText::new(body).font(sans(13.0)).color(TEXT()));
        });
    }
}

pub fn footnote(ui: &mut Ui, text: &str) {
    ui.add_space(4.0);
    ui.label(egui::RichText::new(text).font(sans(11.0)).color(FAINT()));
}

pub fn empty(ui: &mut Ui, height: f32, text: &str) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), height), Sense::hover());
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        text,
        sans(12.0),
        FAINT(),
    );
}

/// Formatted change and its colour; a dash when the feed has no move.
pub fn delta(delta: Option<Delta>) -> (String, Color32) {
    match delta {
        Some(d) => (d.text(), delta_color(Some(d))),
        None => ("—".into(), FAINT()),
    }
}

pub fn dot(painter: &Painter, center: Pos2, color: Color32) {
    painter.circle_filled(center, 3.0, color);
}

/// One cell of a stat grid.
#[derive(Clone, Debug)]
pub struct Stat {
    pub label: String,
    pub value: String,
    pub tone: Color32,
    pub sub: String,
    pub sub_tone: Color32,
    pub hint: String,
}

impl Stat {
    pub fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
            tone: TEXT(),
            sub: String::new(),
            sub_tone: FAINT(),
            hint: String::new(),
        }
    }

    pub fn tone(mut self, tone: Color32) -> Self {
        self.tone = tone;
        self
    }

    pub fn sub(mut self, sub: impl Into<String>, tone: Color32) -> Self {
        self.sub = sub.into();
        self.sub_tone = tone;
        self
    }

    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = hint.into();
        self
    }
}

/// Dense grid of label / value / note cells separated by hairlines.
pub fn stat_grid(ui: &mut Ui, stats: &[Stat], min_cell: f32) {
    if stats.is_empty() {
        return;
    }
    let width = ui.available_width();
    // Fill rows evenly: 15 stats in at most 7 columns become 3 rows of 5.
    let max_cols = ((width / min_cell).floor() as usize).clamp(1, stats.len());
    let cols = stats.len().div_ceil(stats.len().div_ceil(max_cols));
    let cell_w = width / cols as f32;
    let has_sub = stats.iter().any(|s| !s.sub.is_empty());
    let cell_h = if has_sub { 62.0 } else { 48.0 };
    let rows = stats.len().div_ceil(cols);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, cell_h * rows as f32), Sense::hover());
    let painter = ui.painter();
    for (i, stat) in stats.iter().enumerate() {
        let (r, c) = (i / cols, i % cols);
        let cell = Rect::from_min_size(
            rect.min + Vec2::new(c as f32 * cell_w, r as f32 * cell_h),
            Vec2::new(cell_w, cell_h),
        );
        if c > 0 {
            painter.vline(
                cell.left(),
                cell.y_range().shrink(8.0),
                Stroke::new(1.0, BORDER()),
            );
        }
        if r > 0 {
            painter.hline(cell.x_range(), cell.top(), Stroke::new(1.0, BORDER()));
        }
        let inner = cell.shrink2(Vec2::new(if c == 0 { 0.0 } else { 12.0 }, 8.0));
        let label = painter.layout_job(caps_job(&stat.label, 9.5, FAINT()));
        painter.galley(inner.left_top(), label, FAINT());
        let value_rect = Rect::from_min_size(
            inner.left_top() + Vec2::new(0.0, 14.0),
            Vec2::new(inner.width(), 20.0),
        );
        cell_text(
            painter,
            value_rect,
            &stat.value,
            mono(14.5),
            stat.tone,
            Align::Min,
        );
        if !stat.sub.is_empty() {
            let sub_rect = Rect::from_min_size(
                inner.left_top() + Vec2::new(0.0, 35.0),
                Vec2::new(inner.width(), 14.0),
            );
            cell_text(
                painter,
                sub_rect,
                &stat.sub,
                sans(11.0),
                stat.sub_tone,
                Align::Min,
            );
        }
        if !stat.hint.is_empty() {
            let id = ui.id().with(("stat", i, &stat.label));
            ui.interact(cell, id, Sense::hover())
                .on_hover_text(&stat.hint);
        }
    }
}

/// Column widths: fixed pixels or a share of the remaining width.
#[derive(Clone, Copy, Debug)]
pub enum W {
    Px(f32),
    Flex(f32),
}

#[derive(Clone, Copy, Debug)]
pub struct Col {
    pub title: &'static str,
    pub width: W,
    pub align: Align,
}

pub const fn col(title: &'static str, width: W, align: Align) -> Col {
    Col {
        title,
        width,
        align,
    }
}

/// Resolved horizontal extents of each column for one table.
pub struct Layout {
    pub xs: Vec<(f32, f32)>,
}

impl Layout {
    pub fn new(left: f32, width: f32, cols: &[Col]) -> Self {
        const PAD: f32 = 12.0;
        let fixed: f32 = cols
            .iter()
            .map(|c| match c.width {
                W::Px(px) => px,
                W::Flex(_) => 0.0,
            })
            .sum();
        let shares: f32 = cols
            .iter()
            .map(|c| match c.width {
                W::Flex(f) => f,
                W::Px(_) => 0.0,
            })
            .sum();
        let pads = PAD * cols.len().saturating_sub(1) as f32;
        let flex = (width - fixed - pads).max(0.0);
        let mut x = left;
        let xs = cols
            .iter()
            .map(|c| {
                let w = match c.width {
                    W::Px(px) => px,
                    W::Flex(f) if shares > 0.0 => flex * f / shares,
                    W::Flex(_) => 0.0,
                };
                let span = (x, x + w);
                x += w + PAD;
                span
            })
            .collect();
        Self { xs }
    }

    pub fn cell(&self, row: Rect, index: usize) -> Rect {
        let (a, b) = self.xs[index];
        Rect::from_x_y_ranges(a..=b, row.y_range())
    }
}

/// Header row; returns the layout and the column index that was clicked.
pub fn table_header(
    ui: &mut Ui,
    cols: &[Col],
    sort: Option<(usize, bool)>,
) -> (Layout, Option<usize>) {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 26.0), Sense::hover());
    let layout = Layout::new(rect.left() + 8.0, width - 16.0, cols);
    let mut clicked = None;
    for (i, c) in cols.iter().enumerate() {
        if c.title.is_empty() {
            continue;
        }
        let cell = layout.cell(rect, i);
        let mut title = c.title.to_string();
        if let Some((_, desc)) = sort.filter(|(col, _)| *col == i) {
            title.push_str(if desc { " ↓" } else { " ↑" });
        }
        let galley = ui.painter().layout_job(caps_job(&title, 9.5, FAINT()));
        let x = match c.align {
            Align::Min => cell.left(),
            Align::Center => cell.center().x - galley.size().x / 2.0,
            Align::Max => cell.right() - galley.size().x,
        };
        ui.painter().galley(
            Pos2::new(x, cell.center().y - galley.size().y / 2.0),
            galley,
            FAINT(),
        );
        if sort.is_some() {
            let hit = ui.interact(cell, ui.id().with(("sort", i)), Sense::click());
            if hit.hovered() {
                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            }
            if hit.clicked() {
                clicked = Some(i);
            }
        }
    }
    ui.painter()
        .hline(rect.x_range(), rect.bottom(), Stroke::new(1.0, BORDER()));
    (layout, clicked)
}

/// Allocates one row with hover and selection fills and a bottom hairline.
pub fn table_row(ui: &mut Ui, height: f32, selected: bool, clickable: bool) -> (Rect, Response) {
    let sense = if clickable {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(Vec2::new(ui.available_width(), height), sense);
    let painter = ui.painter();
    if selected {
        painter.rect_filled(rect, crate::theme::corner(3), HOVER());
    } else if response.hovered() {
        painter.rect_filled(rect, crate::theme::corner(3), RAISED());
    }
    painter.hline(
        rect.x_range(),
        rect.bottom(),
        Stroke::new(1.0, BORDER().gamma_multiply(0.7)),
    );
    if clickable && response.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    (rect, response)
}

/// Small caps group label inside a table, e.g. RATES.
pub fn table_group(ui: &mut Ui, text: &str) {
    ui.add_space(4.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 22.0), Sense::hover());
    let galley = ui
        .painter()
        .layout_job(caps_job(text, 9.5, theme::ACCENT().gamma_multiply(0.85)));
    ui.painter().galley(
        Pos2::new(
            rect.left() + 8.0,
            rect.center().y - galley.size().y / 2.0 + 2.0,
        ),
        galley,
        FAINT(),
    );
}

/// Compact segmented control; returns true when the value changed.
pub fn segmented<T: PartialEq + Copy>(ui: &mut Ui, value: &mut T, options: &[(&str, T)]) -> bool {
    let mut changed = false;
    let frame = Frame::new()
        .fill(theme::INPUT())
        .stroke(Stroke::new(1.0, BORDER()))
        .corner_radius(crate::theme::corner(5))
        .inner_margin(Margin::same(2));
    frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            for (label, option) in options {
                let active = *value == *option;
                let galley = ui
                    .painter()
                    .layout_no_wrap(label.to_string(), sans(11.5), TEXT());
                let size = Vec2::new(galley.size().x + 16.0, 20.0);
                let (rect, response) = ui.allocate_exact_size(size, Sense::click());
                let fill = if active {
                    HOVER()
                } else if response.hovered() {
                    RAISED()
                } else {
                    Color32::TRANSPARENT
                };
                ui.painter()
                    .rect_filled(rect, crate::theme::corner(4), fill);
                if active {
                    ui.painter().rect_stroke(
                        rect,
                        crate::theme::corner(4),
                        Stroke::new(1.0, theme::BORDER_STRONG()),
                        StrokeKind::Inside,
                    );
                }
                ui.painter().galley(
                    rect.center() - galley.size() / 2.0,
                    galley,
                    if active { TEXT() } else { MUTED() },
                );
                if response.hovered() {
                    ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                }
                if response.clicked() && !active {
                    *value = *option;
                    changed = true;
                }
            }
        });
    });
    changed
}

/// A small on/off text toggle with a square indicator.
pub fn toggle(ui: &mut Ui, on: &mut bool, label: &str) -> Response {
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), sans(11.5), TEXT());
    let size = Vec2::new(galley.size().x + 22.0, 22.0);
    let (rect, mut response) = ui.allocate_exact_size(size, Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    let painter = ui.painter();
    let mark = Rect::from_center_size(
        Pos2::new(rect.left() + 6.0, rect.center().y),
        Vec2::splat(10.0),
    );
    painter.rect_stroke(
        mark,
        crate::theme::corner(2),
        Stroke::new(
            1.0,
            if *on {
                theme::ACCENT()
            } else {
                theme::BORDER_STRONG()
            },
        ),
        StrokeKind::Inside,
    );
    if *on {
        painter.rect_filled(mark.shrink(2.5), crate::theme::corner(1), theme::ACCENT());
    }
    let color = if *on || response.hovered() {
        TEXT()
    } else {
        MUTED()
    };
    painter.galley(
        Pos2::new(rect.left() + 16.0, rect.center().y - galley.size().y / 2.0),
        galley,
        color,
    );
    if response.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    response
}

/// Two columns at `ratio`, stacked when narrower than `min_width`.
/// `add` runs once per side (0 = left, 1 = right) so both may borrow the same state.
pub fn split(ui: &mut Ui, ratio: f32, min_width: f32, mut add: impl FnMut(&mut Ui, usize)) {
    let total = ui.available_width();
    if total < min_width {
        add(ui, 0);
        ui.add_space(GAP);
        add(ui, 1);
        return;
    }
    let lw = ((total - GAP) * ratio).floor();
    let rw = total - GAP - lw;
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = GAP;
        for (side, width) in [(0, lw), (1, rw)] {
            ui.vertical(|ui| {
                ui.set_width(width);
                ui.spacing_mut().item_spacing.x = 8.0;
                add(ui, side);
            });
        }
    });
}

/// Panels in two columns; each goes under the shorter column, using the heights
/// measured on the previous frame. Stacks into one column below `min_width`.
pub fn masonry(
    ui: &mut Ui,
    id: egui::Id,
    count: usize,
    min_width: f32,
    mut add: impl FnMut(&mut Ui, usize),
) {
    let total = ui.available_width();
    if total < min_width {
        for i in 0..count {
            add(ui, i);
            if i + 1 < count {
                ui.add_space(GAP);
            }
        }
        return;
    }
    let mut heights: Vec<f32> = ui.ctx().data(|d| d.get_temp(id)).unwrap_or_default();
    heights.resize(count, 240.0);
    let mut cols: [Vec<usize>; 2] = [Vec::new(), Vec::new()];
    let mut acc = [0.0_f32; 2];
    for (i, h) in heights.iter().enumerate() {
        let c = usize::from(acc[1] < acc[0]);
        cols[c].push(i);
        acc[c] += h;
    }
    let width = ((total - GAP) / 2.0).floor();
    let mut measured = heights.clone();
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = GAP;
        for col in &cols {
            ui.vertical(|ui| {
                ui.set_width(width);
                ui.spacing_mut().item_spacing.x = 8.0;
                for (k, &i) in col.iter().enumerate() {
                    let top = ui.cursor().top();
                    add(ui, i);
                    if k + 1 < col.len() {
                        ui.add_space(GAP);
                    }
                    measured[i] = ui.cursor().top() - top;
                }
            });
        }
    });
    if measured
        .iter()
        .zip(&heights)
        .any(|(a, b)| (a - b).abs() > 1.0)
    {
        ui.ctx().data_mut(|d| d.insert_temp(id, measured));
        ui.ctx().request_repaint();
    }
}

/// Several styled runs in one elided line, e.g. a name and a muted suffix.
pub fn cell_runs(painter: &Painter, rect: Rect, runs: &[(&str, FontId, Color32)], align: Align) {
    let mut job = LayoutJob::default();
    for (text, font, color) in runs {
        job.append(
            text,
            0.0,
            TextFormat {
                font_id: font.clone(),
                color: *color,
                valign: Align::Center,
                ..Default::default()
            },
        );
    }
    job.wrap = TextWrapping {
        max_width: rect.width().max(1.0),
        max_rows: 1,
        break_anywhere: true,
        overflow_character: Some('…'),
    };
    let galley = painter.layout_job(job);
    let x = match align {
        Align::Min => rect.left(),
        Align::Center => rect.center().x - galley.size().x / 2.0,
        Align::Max => rect.right() - galley.size().x,
    };
    painter.galley(
        Pos2::new(x, rect.center().y - galley.size().y / 2.0),
        galley,
        TEXT(),
    );
}

/// Text button that looks like a link.
pub fn link(ui: &mut Ui, text: &str, color: Color32) -> Response {
    let response = ui.add(
        egui::Label::new(egui::RichText::new(text).font(sans(11.5)).color(color))
            .sense(Sense::click()),
    );
    if response.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    response
}

//! Hand-painted charts with round-number axes and calendar-aware date ticks.

use crate::theme::{mono, sans, BG, BORDER, BORDER_STRONG, FAINT, GREEN, MUTED, RAISED, RED};
use chrono::{Datelike, TimeZone, Timelike, Utc};
use dollar_dashboard::model::{value_at, Point};
use eframe::egui::{
    Align2, Color32, CornerRadius, Mesh, Painter, Pos2, Rect, Response, Sense, Shape, Stroke,
    StrokeKind, Ui, Vec2,
};

pub struct Line<'a> {
    pub name: &'a str,
    pub points: &'a [Point],
    pub color: Color32,
    pub width: f32,
    pub fill: bool,
}

impl<'a> Line<'a> {
    pub fn new(name: &'a str, points: &'a [Point], color: Color32) -> Self {
        Self {
            name,
            points,
            color,
            width: 1.6,
            fill: false,
        }
    }

    pub fn fill(mut self) -> Self {
        self.fill = true;
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }
}

pub struct TimeChart<'a> {
    pub height: f32,
    pub lines: Vec<Line<'a>>,
    pub log: bool,
    /// Axis labels; ticks are round numbers so a compact format reads well.
    pub axis: &'a dyn Fn(f64) -> String,
    /// Tooltip and last-value tag.
    pub value: &'a dyn Fn(f64) -> String,
    pub zero_line: bool,
    pub hourly: bool,
    /// Log-time x axis: position by log10(days since this unix time). Year ticks.
    pub log_time: Option<i64>,
}

impl<'a> TimeChart<'a> {
    pub fn new(
        height: f32,
        axis: &'a dyn Fn(f64) -> String,
        value: &'a dyn Fn(f64) -> String,
    ) -> Self {
        Self {
            height,
            lines: Vec::new(),
            log: false,
            axis,
            value,
            zero_line: false,
            hourly: false,
            log_time: None,
        }
    }

    pub fn line(mut self, line: Line<'a>) -> Self {
        self.lines.push(line);
        self
    }

    pub fn log(mut self, log: bool) -> Self {
        self.log = log;
        self
    }

    pub fn zero_line(mut self) -> Self {
        self.zero_line = true;
        self
    }

    pub fn hourly(mut self) -> Self {
        self.hourly = true;
        self
    }

    /// Log-log charts: x is log10(days since `genesis`).
    pub fn log_time(mut self, genesis: i64) -> Self {
        self.log_time = Some(genesis);
        self
    }
}

pub fn time_chart(ui: &mut Ui, chart: TimeChart) -> Response {
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, chart.height), Sense::hover());
    let painter = ui.painter_at(rect);
    let min_t = chart.log_time.map_or(i64::MIN, |g| g + DAY_SECS);
    let valid = |p: &&Point| p.v.is_finite() && (!chart.log || p.v > 0.0) && p.t >= min_t;
    let lines: Vec<(&Line, Vec<Point>)> = chart
        .lines
        .iter()
        .map(|line| (line, line.points.iter().filter(valid).cloned().collect()))
        .filter(|(_, points): &(&Line, Vec<Point>)| points.len() >= 2)
        .collect();
    if lines.is_empty() {
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            "No history yet · refresh sources",
            sans(12.0),
            FAINT(),
        );
        return response;
    }
    let t0 = lines.iter().map(|(_, p)| p[0].t).min().unwrap_or(0);
    let t1 = lines.iter().map(|(_, p)| p[p.len() - 1].t).max().unwrap_or(1);
    let tf = |v: f64| if chart.log { v.ln() } else { v };
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for (_, points) in &lines {
        for p in points {
            lo = lo.min(tf(p.v));
            hi = hi.max(tf(p.v));
        }
    }
    if chart.zero_line && !chart.log {
        lo = lo.min(0.0);
        hi = hi.max(0.0);
    }
    if hi - lo < 1e-9 {
        let pad = if chart.log { 0.05 } else { hi.abs().max(1.0) * 0.05 };
        lo -= pad;
        hi += pad;
    }
    let pad = (hi - lo) * 0.07;
    lo -= pad;
    hi += pad;
    let inv = |v: f64| if chart.log { v.exp() } else { v };
    let ticks = if chart.log {
        log_ticks(inv(lo), inv(hi))
    } else {
        nice_ticks(lo, hi, ((chart.height - 26.0) / 40.0).clamp(3.0, 6.0) as usize)
    };
    let labels: Vec<(f64, String)> = ticks.iter().map(|v| (*v, (chart.axis)(*v))).collect();
    let (primary, primary_points) = &lines[0];
    let last = primary_points[primary_points.len() - 1];
    let tag = (chart.value)(last.v);
    let label_w = labels
        .iter()
        .map(|(_, text)| text.as_str())
        .chain(std::iter::once(tag.as_str()))
        .map(|text| painter.layout_no_wrap(text.to_string(), mono(10.5), FAINT()).size().x)
        .fold(0.0_f32, f32::max);
    let gutter = (label_w + 18.0).max(44.0);
    let plot = Rect::from_min_max(
        rect.min + Vec2::new(0.0, 6.0),
        rect.max - Vec2::new(gutter, 20.0),
    );
    // Linear time, or log10(days since genesis) for log-log charts.
    let xf = |t: i64| match chart.log_time {
        Some(g) => (((t - g) as f64 / DAY_SECS as f64).max(1.0)).log10(),
        None => (t - t0) as f64,
    };
    let (x0, x1) = (xf(t0), xf(t1));
    let xspan = (x1 - x0).max(1e-9);
    let x_of = |t: i64| plot.left() + plot.width() * ((xf(t) - x0) / xspan) as f32;
    let t_of = |x: f32| {
        let u = x0 + xspan * ((x - plot.left()) / plot.width()) as f64;
        match chart.log_time {
            Some(g) => g + (10f64.powf(u) * DAY_SECS as f64) as i64,
            None => t0 + u as i64,
        }
    };
    let y_of = |v: f64| plot.bottom() - plot.height() * ((tf(v) - lo) / (hi - lo)) as f32;

    let y_last = y_of(last.v).clamp(plot.top(), plot.bottom());
    for (v, text) in &labels {
        let y = y_of(*v);
        if y < plot.top() - 0.5 || y > plot.bottom() + 0.5 {
            continue;
        }
        painter.hline(plot.x_range(), y, Stroke::new(1.0, BORDER()));
        // The last-value tag sits on the axis; drop labels it would cover.
        if (y - y_last).abs() < 13.0 {
            continue;
        }
        painter.text(
            Pos2::new(plot.right() + 8.0, y),
            Align2::LEFT_CENTER,
            text,
            mono(10.5),
            FAINT(),
        );
    }
    if chart.zero_line && !chart.log {
        painter.hline(plot.x_range(), y_of(0.0), Stroke::new(1.0, BORDER_STRONG()));
    }
    let max_ticks = (plot.width() / 86.0).max(2.0) as usize;
    let ticks = match chart.log_time {
        Some(_) => year_ticks(t0, t1, |t| x_of(t), 40.0),
        None => time_ticks(t0, t1, max_ticks, chart.hourly),
    };
    for (t, label) in ticks {
        let x = x_of(t);
        if x < plot.left() + 12.0 || x > plot.right() - 12.0 {
            continue;
        }
        painter.vline(
            x,
            plot.bottom()..=plot.bottom() + 3.0,
            Stroke::new(1.0, BORDER_STRONG()),
        );
        painter.text(
            Pos2::new(x, plot.bottom() + 5.0),
            Align2::CENTER_TOP,
            label,
            mono(10.5),
            FAINT(),
        );
    }

    for (line, points) in lines.iter().rev() {
        let pts: Vec<Pos2> = points.iter().map(|p| Pos2::new(x_of(p.t), y_of(p.v))).collect();
        if line.fill {
            painter.add(area(&pts, plot.bottom(), line.color));
        }
        painter.add(Shape::line(pts, Stroke::new(line.width, line.color)));
    }

    // Last print tag on the axis, in the series colour.
    let galley = painter.layout_no_wrap(tag, mono(10.5), BG());
    let tag_rect = Rect::from_min_size(
        Pos2::new(plot.right() + 3.0, y_last - galley.size().y / 2.0 - 2.0),
        galley.size() + Vec2::new(10.0, 4.0),
    );
    painter.rect_filled(tag_rect, crate::theme::corner(3), primary.color);
    painter.galley(tag_rect.min + Vec2::new(5.0, 2.0), galley, BG());

    if let Some(mouse) = response.hover_pos().filter(|p| plot.expand(4.0).contains(*p)) {
        let t = t_of(mouse.x);
        let index = primary_points
            .partition_point(|p| p.t < t)
            .min(primary_points.len() - 1);
        let near = if index > 0 && (primary_points[index].t - t).abs() > (t - primary_points[index - 1].t).abs() {
            index - 1
        } else {
            index
        };
        let at = primary_points[near].t;
        let x = x_of(at);
        painter.vline(x, plot.y_range(), Stroke::new(1.0, MUTED().gamma_multiply(0.5)));
        let mut rows = vec![(date_label(at, chart.hourly), FAINT())];
        for (line, points) in &lines {
            if let Some(v) = value_at(points, at) {
                painter.circle_filled(Pos2::new(x, y_of(v)), 3.5, line.color);
                let name = if line.name.is_empty() {
                    String::new()
                } else {
                    format!("{}  ", line.name)
                };
                rows.push((format!("{name}{}", (chart.value)(v)), line.color));
            }
        }
        tooltip(&painter, plot, Pos2::new(x, mouse.y), &rows);
    }
    response
}

/// Bars per observation (e.g. daily ETF flows), green above zero and red below.
pub fn bars(
    ui: &mut Ui,
    height: f32,
    points: &[Point],
    axis: &dyn Fn(f64) -> String,
    value: &dyn Fn(f64) -> String,
) -> Response {
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    let painter = ui.painter_at(rect);
    let points: Vec<Point> = points.iter().filter(|p| p.v.is_finite()).cloned().collect();
    if points.len() < 2 {
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            "No history yet · refresh sources",
            sans(12.0),
            FAINT(),
        );
        return response;
    }
    let mut lo = points.iter().map(|p| p.v).fold(0.0_f64, f64::min);
    let mut hi = points.iter().map(|p| p.v).fold(0.0_f64, f64::max);
    if hi - lo < 1e-9 {
        hi += 1.0;
    }
    let pad = (hi - lo) * 0.08;
    lo -= if lo < 0.0 { pad } else { 0.0 };
    hi += pad;
    let ticks = nice_ticks(lo, hi, ((height - 26.0) / 34.0).clamp(2.0, 5.0) as usize);
    let labels: Vec<(f64, String)> = ticks.iter().map(|v| (*v, axis(*v))).collect();
    let label_w = labels
        .iter()
        .map(|(_, t)| painter.layout_no_wrap(t.clone(), mono(10.5), FAINT()).size().x)
        .fold(0.0_f32, f32::max);
    let plot = Rect::from_min_max(
        rect.min + Vec2::new(0.0, 6.0),
        rect.max - Vec2::new(label_w + 14.0, 20.0),
    );
    let y_of = |v: f64| plot.bottom() - plot.height() * ((v - lo) / (hi - lo)) as f32;
    for (v, text) in &labels {
        let y = y_of(*v);
        if y < plot.top() - 0.5 || y > plot.bottom() + 0.5 {
            continue;
        }
        painter.hline(plot.x_range(), y, Stroke::new(1.0, BORDER()));
        painter.text(
            Pos2::new(plot.right() + 8.0, y),
            Align2::LEFT_CENTER,
            text,
            mono(10.5),
            FAINT(),
        );
    }
    let zero = y_of(0.0);
    painter.hline(plot.x_range(), zero, Stroke::new(1.0, BORDER_STRONG()));
    let slot = plot.width() / points.len() as f32;
    let bar_w = (slot * 0.68).max(1.0);
    let hover_index = response
        .hover_pos()
        .filter(|p| plot.expand(4.0).contains(*p))
        .map(|p| (((p.x - plot.left()) / slot) as usize).min(points.len() - 1));
    for (i, p) in points.iter().enumerate() {
        let x = plot.left() + slot * (i as f32 + 0.5);
        let y = y_of(p.v);
        let color = if p.v >= 0.0 { GREEN() } else { RED() };
        let alpha = if hover_index.is_some_and(|h| h != i) { 0.45 } else { 0.85 };
        painter.rect_filled(
            Rect::from_min_max(
                Pos2::new(x - bar_w / 2.0, y.min(zero)),
                Pos2::new(x + bar_w / 2.0, y.max(zero).max(y.min(zero) + 1.0)),
            ),
            CornerRadius::ZERO,
            color.gamma_multiply(alpha),
        );
    }
    let short = |t: i64| {
        Utc.timestamp_opt(t, 0)
            .single()
            .map(|d| d.format("%-d %b").to_string())
            .unwrap_or_default()
    };
    let label_every = ((points.len() as f32 * 64.0 / plot.width()).ceil() as usize).max(1);
    // Several bars can share a day (8-hour funding); label each day once.
    let mut last_label = String::new();
    for i in (0..points.len()).rev().step_by(label_every) {
        let x = plot.left() + slot * (i as f32 + 0.5);
        let label = short(points[i].t);
        if x < plot.left() + 18.0 || label == last_label {
            continue;
        }
        painter.text(
            Pos2::new(x, plot.bottom() + 5.0),
            Align2::CENTER_TOP,
            &label,
            mono(10.5),
            FAINT(),
        );
        last_label = label;
    }
    if let Some(i) = hover_index {
        let p = points[i];
        let x = plot.left() + slot * (i as f32 + 0.5);
        tooltip(
            &painter,
            plot,
            Pos2::new(x, y_of(p.v)),
            &[
                (date_label(p.t, false), FAINT()),
                (value(p.v), if p.v >= 0.0 { GREEN() } else { RED() }),
            ],
        );
    }
    response
}

/// Minimal trend line scaled to its own range, with an end dot.
pub fn sparkline(painter: &Painter, rect: Rect, values: &[f64], color: Color32) {
    let values: Vec<f64> = values.iter().copied().filter(|v| v.is_finite()).collect();
    if values.len() < 2 || rect.width() < 8.0 {
        return;
    }
    let lo = values.iter().copied().fold(f64::INFINITY, f64::min);
    let hi = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let span = hi - lo;
    let pts: Vec<Pos2> = values
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let x = rect.left() + rect.width() * i as f32 / (values.len() - 1) as f32;
            let y = if span < 1e-12 {
                rect.center().y
            } else {
                rect.bottom() - rect.height() * ((v - lo) / span) as f32
            };
            Pos2::new(x, y)
        })
        .collect();
    let end = pts[pts.len() - 1];
    painter.add(area(&pts, rect.bottom(), color.gamma_multiply(0.7)));
    painter.add(Shape::line(pts, Stroke::new(1.2, color)));
    painter.circle_filled(end, 2.0, color);
}

/// 52-week style range: a track from low to high with a marker at `value`.
pub fn range_bar(painter: &Painter, rect: Rect, lo: f64, hi: f64, value: f64, color: Color32) {
    if hi.is_nan() || lo.is_nan() || hi <= lo || rect.width() < 12.0 {
        return;
    }
    let y = rect.center().y;
    painter.hline(rect.x_range(), y, Stroke::new(2.0, BORDER_STRONG()));
    let t = ((value - lo) / (hi - lo)).clamp(0.0, 1.0) as f32;
    let x = rect.left() + rect.width() * t;
    painter.rect_filled(
        Rect::from_center_size(Pos2::new(x, y), Vec2::new(3.0, 9.0)),
        crate::theme::corner(1),
        color,
    );
}

/// Translucent area from a polyline down to `bottom`.
fn area(pts: &[Pos2], bottom: f32, color: Color32) -> Shape {
    let mut mesh = Mesh::default();
    let top = color.gamma_multiply(0.16);
    let base = color.gamma_multiply(0.0);
    for pair in pts.windows(2) {
        let i = mesh.vertices.len() as u32;
        mesh.colored_vertex(pair[0], top);
        mesh.colored_vertex(pair[1], top);
        mesh.colored_vertex(Pos2::new(pair[1].x, bottom), base);
        mesh.colored_vertex(Pos2::new(pair[0].x, bottom), base);
        mesh.add_triangle(i, i + 1, i + 2);
        mesh.add_triangle(i, i + 2, i + 3);
    }
    Shape::mesh(mesh)
}

/// Floating readout drawn inside the plot, flipped away from the edges.
pub fn tooltip(painter: &Painter, plot: Rect, anchor: Pos2, rows: &[(String, Color32)]) {
    let galleys: Vec<_> = rows
        .iter()
        .enumerate()
        .map(|(i, (text, color))| {
            let font = if i == 0 { mono(10.5) } else { mono(12.0) };
            painter.layout_no_wrap(text.clone(), font, *color)
        })
        .collect();
    let w = galleys.iter().map(|g| g.size().x).fold(0.0_f32, f32::max) + 16.0;
    let h = galleys.iter().map(|g| g.size().y + 2.0).sum::<f32>() + 10.0;
    let mut x = anchor.x + 12.0;
    if x + w > plot.right() {
        x = anchor.x - 12.0 - w;
    }
    let y = (anchor.y - h - 10.0).clamp(plot.top(), (plot.bottom() - h).max(plot.top()));
    let rect = Rect::from_min_size(Pos2::new(x.max(plot.left()), y), Vec2::new(w, h));
    painter.rect_filled(rect, crate::theme::corner(4), RAISED());
    painter.rect_stroke(
        rect,
        crate::theme::corner(4),
        Stroke::new(1.0, BORDER_STRONG()),
        StrokeKind::Inside,
    );
    let mut cursor = rect.min + Vec2::new(8.0, 5.0);
    for (galley, (_, color)) in galleys.into_iter().zip(rows) {
        let height = galley.size().y;
        painter.galley(cursor, galley, *color);
        cursor.y += height + 2.0;
    }
}

/// Round-number ticks (1, 2, 5 × 10^k) covering [lo, hi].
pub fn nice_ticks(lo: f64, hi: f64, target: usize) -> Vec<f64> {
    if !lo.is_finite() || !hi.is_finite() || hi <= lo {
        return Vec::new();
    }
    let step = nice_step((hi - lo) / target.max(1) as f64);
    let mut v = (lo / step).ceil() * step;
    let mut out = Vec::new();
    while v <= hi + step * 1e-9 && out.len() < 50 {
        out.push(if v.abs() < step * 1e-9 { 0.0 } else { v });
        v += step;
    }
    out
}

pub fn nice_step(raw: f64) -> f64 {
    let mag = 10f64.powf(raw.abs().max(1e-12).log10().floor());
    let norm = raw.abs() / mag;
    let nice = if norm < 1.5 {
        1.0
    } else if norm < 3.0 {
        2.0
    } else if norm < 7.0 {
        5.0
    } else {
        10.0
    };
    nice * mag
}

/// Log-axis ticks at 1-2-5 decades, thinned or densified to 3–8 labels.
pub fn log_ticks(lo: f64, hi: f64) -> Vec<f64> {
    if hi.is_nan() || lo.is_nan() || hi <= lo || lo <= 0.0 {
        return Vec::new();
    }
    let gather = |multipliers: &[f64]| {
        let mut out = Vec::new();
        let start = lo.log10().floor() as i32;
        let end = hi.log10().ceil() as i32;
        for exp in start..=end {
            for m in multipliers {
                let v = m * 10f64.powi(exp);
                if v >= lo && v <= hi {
                    out.push(v);
                }
            }
        }
        out
    };
    let mut ticks = gather(&[1.0, 2.0, 5.0]);
    if ticks.len() > 8 {
        ticks = gather(&[1.0]);
    }
    if ticks.len() < 3 {
        ticks = gather(&[1.0, 1.5, 2.0, 3.0, 5.0, 7.0]);
    }
    if ticks.len() < 3 {
        ticks = nice_ticks(lo, hi, 4).into_iter().filter(|v| *v > 0.0).collect();
    }
    ticks
}

const DAY_SECS: i64 = 86_400;

/// 1 January of each year in range, thinned so labels sit at least `min_gap` px apart.
/// Walks from the newest year back, because a log-time axis crowds the recent end.
pub fn year_ticks(t0: i64, t1: i64, x_of: impl Fn(i64) -> f32, min_gap: f32) -> Vec<(i64, String)> {
    let (Some(a), Some(b)) = (Utc.timestamp_opt(t0, 0).single(), Utc.timestamp_opt(t1, 0).single()) else {
        return Vec::new();
    };
    let mut out: Vec<(i64, String)> = Vec::new();
    let mut last_x = f32::INFINITY;
    for year in ((a.year() + 1)..=b.year()).rev() {
        let Some(t) = Utc.with_ymd_and_hms(year, 1, 1, 0, 0, 0).single().map(|d| d.timestamp()) else {
            continue;
        };
        let x = x_of(t);
        if last_x - x >= min_gap {
            out.push((t, year.to_string()));
            last_x = x;
        }
    }
    out.reverse();
    out
}

/// Calendar ticks between two times: hours, days, weeks, months or years.
pub fn time_ticks(t0: i64, t1: i64, max: usize, hourly: bool) -> Vec<(i64, String)> {
    const HOUR: i64 = 3600;
    const DAY: i64 = 86_400;
    let span = (t1 - t0).max(1);
    let max = max.max(2) as i64;
    let at = |t: i64| Utc.timestamp_opt(t, 0).single();
    if hourly && span <= 6 * DAY {
        for step in [HOUR, 3 * HOUR, 6 * HOUR, 12 * HOUR, DAY] {
            if span / step <= max {
                let mut t = (t0 / step + 1) * step;
                let mut out = Vec::new();
                while t <= t1 {
                    if let Some(d) = at(t) {
                        let label = if d.hour() == 0 {
                            d.format("%-d %b").to_string()
                        } else {
                            d.format("%H:%M").to_string()
                        };
                        out.push((t, label));
                    }
                    t += step;
                }
                return out;
            }
        }
    }
    for days in [1, 2, 7, 14] {
        if span / (days * DAY) <= max {
            let mut t = (t0 / DAY + 1) * DAY;
            if days == 7 || days == 14 {
                // Align weekly ticks to Mondays (1970-01-05 was a Monday).
                let monday = 4 * DAY;
                t = monday + ((t - monday + days * DAY - 1) / (days * DAY)) * days * DAY;
            }
            let mut out = Vec::new();
            while t <= t1 {
                if let Some(d) = at(t) {
                    out.push((t, d.format("%-d %b").to_string()));
                }
                t += days * DAY;
            }
            return out;
        }
    }
    let Some(start) = at(t0) else {
        return Vec::new();
    };
    for months in [1, 2, 3, 6, 12, 24, 60] {
        if span / (months as i64 * 30 * DAY) > max {
            continue;
        }
        let mut year = start.year();
        let mut month = start.month() as i32;
        // First month boundary after t0 that sits on the step grid.
        loop {
            month += 1;
            if month > 12 {
                month = 1;
                year += 1;
            }
            let index = year * 12 + month - 1;
            if months >= 12 {
                if month == 1 && year % (months / 12) == 0 {
                    break;
                }
            } else if index % months == 0 {
                break;
            }
        }
        let mut out = Vec::new();
        while let Some(d) = Utc.with_ymd_and_hms(year, month as u32, 1, 0, 0, 0).single() {
            let t = d.timestamp();
            if t > t1 {
                break;
            }
            let label = if months >= 12 || month == 1 {
                d.format("%Y").to_string()
            } else {
                d.format("%b").to_string()
            };
            out.push((t, label));
            month += months;
            while month > 12 {
                month -= 12;
                year += 1;
            }
        }
        return out;
    }
    Vec::new()
}

pub fn date_label(t: i64, hourly: bool) -> String {
    Utc.timestamp_opt(t, 0)
        .single()
        .map(|d| {
            if hourly {
                d.format("%a %-d %b %H:%M UTC").to_string()
            } else {
                d.format("%a %-d %b %Y").to_string()
            }
        })
        .unwrap_or_default()
}

/// Compact axis number: 120k, 1.5M, 0.25, −604.
pub fn axis_number(v: f64) -> String {
    let abs = v.abs();
    let text = if abs >= 1e9 {
        trim(format!("{:.2}", v / 1e9)) + "B"
    } else if abs >= 1e6 {
        trim(format!("{:.2}", v / 1e6)) + "M"
    } else if abs >= 1e4 {
        trim(format!("{:.1}", v / 1e3)) + "k"
    } else if abs >= 100.0 || abs == 0.0 {
        dollar_dashboard::model::fmt_grouped(v, 0)
    } else if abs >= 1.0 {
        trim(format!("{v:.2}"))
    } else {
        trim(format!("{v:.4}"))
    };
    dollar_dashboard::model::with_minus(text)
}

pub fn axis_usd(v: f64) -> String {
    let text = axis_number(v);
    match text.strip_prefix('−') {
        Some(rest) => format!("−${rest}"),
        None => format!("${text}"),
    }
}

fn trim(text: String) -> String {
    if text.contains('.') {
        text.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nice_ticks_land_on_round_numbers() {
        assert_eq!(nice_ticks(53_335.0, 128_718.0, 4), vec![60_000.0, 80_000.0, 100_000.0, 120_000.0]);
        assert_eq!(nice_ticks(-604.0, 1118.0, 4), vec![-500.0, 0.0, 500.0, 1000.0]);
        assert!(nice_ticks(1.0, 1.0, 4).is_empty());
    }

    #[test]
    fn log_ticks_cover_a_decade_span() {
        assert_eq!(log_ticks(0.4, 27.0), vec![0.5, 1.0, 2.0, 5.0, 10.0, 20.0]);
        let wide = log_ticks(0.05, 130_000.0);
        assert!(wide.len() <= 8 && wide.contains(&1000.0));
    }

    #[test]
    fn month_ticks_label_years_at_january() {
        let t0 = Utc.with_ymd_and_hms(2025, 10, 17, 0, 0, 0).unwrap().timestamp();
        let t1 = Utc.with_ymd_and_hms(2026, 10, 17, 0, 0, 0).unwrap().timestamp();
        let ticks = time_ticks(t0, t1, 8, false);
        let labels: Vec<&str> = ticks.iter().map(|(_, l)| l.as_str()).collect();
        assert_eq!(labels, ["Nov", "2026", "Mar", "May", "Jul", "Sep"]);
        let days = time_ticks(t1 - 20 * 86_400, t1, 8, false);
        assert!(days.iter().all(|(t, _)| t % 86_400 == 0));
    }

    #[test]
    fn axis_numbers_are_compact() {
        assert_eq!(axis_usd(120_000.0), "$120k");
        assert_eq!(axis_number(-500.0), "−500");
        assert_eq!(axis_number(0.25), "0.25");
        assert_eq!(axis_number(5.5), "5.5");
        assert_eq!(axis_number(2_500_000.0), "2.5M");
    }
}

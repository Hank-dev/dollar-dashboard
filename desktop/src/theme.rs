//! Palette, type scale and egui style, taken from the running Omarchy setup.
//!
//! Colours come from `~/.config/omarchy/current/theme/colors.toml`, the font from
//! `omarchy-font-current`, corner radius from Hyprland's `decoration:rounding`.
//! `watch` re-reads them when the theme or font changes, so `omarchy-theme-set`
//! restyles a running window. Outside Omarchy the built-in dark palette is used.
//!
//! The upper-case accessors keep the old constant names so call sites read the same.
#![allow(non_snake_case)]

use dollar_dashboard::model::{Delta, Status};
use eframe::egui::{
    self, style::ScrollStyle, Color32, CornerRadius, FontFamily, FontId, Margin, Shadow, Stroke,
    TextStyle, Vec2,
};
use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Palette {
    light: bool,
    bg: Color32,
    surface: Color32,
    raised: Color32,
    hover: Color32,
    input: Color32,
    border: Color32,
    border_strong: Color32,
    text: Color32,
    muted: Color32,
    faint: Color32,
    accent: Color32,
    selection: Color32,
    green: Color32,
    red: Color32,
    amber: Color32,
    blue: Color32,
    cyan: Color32,
    purple: Color32,
    orange: Color32,
    /// Hyprland window rounding; 0 means square corners everywhere.
    rounding: u8,
}

impl Palette {
    /// The original Market Monitor palette, used when no Omarchy theme is found.
    const BUILTIN: Palette = Palette {
        light: false,
        bg: Color32::from_rgb(11, 14, 20),
        surface: Color32::from_rgb(16, 20, 28),
        raised: Color32::from_rgb(21, 26, 36),
        hover: Color32::from_rgb(27, 33, 45),
        input: Color32::from_rgb(9, 12, 17),
        border: Color32::from_rgb(31, 38, 51),
        border_strong: Color32::from_rgb(45, 54, 71),
        text: Color32::from_rgb(228, 231, 238),
        muted: Color32::from_rgb(152, 162, 180),
        faint: Color32::from_rgb(103, 114, 134),
        accent: Color32::from_rgb(108, 160, 245),
        selection: Color32::from_rgb(38, 56, 92),
        green: Color32::from_rgb(64, 201, 140),
        red: Color32::from_rgb(235, 100, 92),
        amber: Color32::from_rgb(230, 182, 88),
        blue: Color32::from_rgb(108, 160, 245),
        cyan: Color32::from_rgb(96, 205, 210),
        purple: Color32::from_rgb(176, 140, 245),
        orange: Color32::from_rgb(240, 138, 70),
        rounding: 6,
    };

    /// Build a palette from an Omarchy `colors.toml` (flat `key = "#rrggbb"` lines).
    pub fn from_colors_toml(src: &str, rounding: u8) -> Option<Palette> {
        let mut keys = std::collections::HashMap::new();
        for line in src.lines() {
            let line = line.trim();
            if line.starts_with('#') {
                continue;
            }
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            keys.insert(k.trim().to_string(), v.trim().trim_matches('"').to_string());
        }
        let color = |k: &str| keys.get(k).and_then(|v| parse_hex(v));
        let bg = color("background")?;
        let fg = color("foreground")?;
        let lighter = color("lighter_background").unwrap_or(mix(bg, fg, 0.12));
        let dark = color("dark_background").unwrap_or(mix(bg, Color32::BLACK, 0.25));
        let red = color("red").unwrap_or(Palette::BUILTIN.red);
        let yellow = color("yellow").unwrap_or(Palette::BUILTIN.amber);
        let accent = color("accent").or(color("blue")).unwrap_or(Palette::BUILTIN.accent);
        Some(Palette {
            light: keys.get("mode").is_some_and(|m| m == "light"),
            bg,
            surface: mix(bg, lighter, 0.30),
            raised: mix(bg, lighter, 0.60),
            hover: lighter,
            input: dark,
            border: mix(bg, fg, 0.13),
            border_strong: mix(bg, fg, 0.24),
            text: fg,
            muted: mix(fg, bg, 0.24),
            faint: mix(fg, bg, 0.44),
            accent,
            selection: color("selection").unwrap_or(mix(bg, accent, 0.3)),
            green: color("green").unwrap_or(Palette::BUILTIN.green),
            red,
            amber: yellow,
            blue: color("blue").unwrap_or(Palette::BUILTIN.blue),
            cyan: color("cyan").unwrap_or(Palette::BUILTIN.cyan),
            purple: color("magenta").unwrap_or(Palette::BUILTIN.purple),
            orange: color("orange").unwrap_or(mix(red, yellow, 0.5)),
            rounding,
        })
    }
}

fn parse_hex(v: &str) -> Option<Color32> {
    let h = v.strip_prefix('#')?;
    if h.len() < 6 {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(h.get(i..i + 2)?, 16).ok();
    Some(Color32::from_rgb(byte(0)?, byte(2)?, byte(4)?))
}

/// Linear blend in sRGB space: `t = 0` gives `a`, `t = 1` gives `b`.
fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()))
}

thread_local! {
    static PALETTE: Cell<Palette> = const { Cell::new(Palette::BUILTIN) };
    static WATCH: RefCell<Watch> = RefCell::new(Watch::default());
}

fn p() -> Palette {
    PALETTE.with(Cell::get)
}

pub fn BG() -> Color32 { p().bg }
pub fn SURFACE() -> Color32 { p().surface }
pub fn RAISED() -> Color32 { p().raised }
pub fn HOVER() -> Color32 { p().hover }
pub fn INPUT() -> Color32 { p().input }
pub fn BORDER() -> Color32 { p().border }
pub fn BORDER_STRONG() -> Color32 { p().border_strong }
pub fn TEXT() -> Color32 { p().text }
pub fn MUTED() -> Color32 { p().muted }
pub fn FAINT() -> Color32 { p().faint }
/// The theme's accent: focus, selection and the active desk marker.
pub fn ACCENT() -> Color32 { p().accent }
pub fn GREEN() -> Color32 { p().green }
pub fn RED() -> Color32 { p().red }
pub fn AMBER() -> Color32 { p().amber }
pub fn BLUE() -> Color32 { p().blue }
pub fn CYAN() -> Color32 { p().cyan }
pub fn PURPLE() -> Color32 { p().purple }
pub fn ORANGE() -> Color32 { p().orange }

/// Panel corner radius: follows Hyprland, square when windows are square.
pub fn RADIUS() -> u8 {
    p().rounding.min(8)
}

/// Small-element radius. Keeps the design's relative sizes but drops to 0 with square windows.
pub fn corner(r: u8) -> CornerRadius {
    CornerRadius::same(if p().rounding == 0 { 0 } else { r })
}

/// Gap between panels and between a panel and the page edge.
pub const GAP: f32 = 12.0;

/// Former proportional sizes, shrunk because the monospace system font runs wider.
const TEXT_SCALE: f32 = 0.93;

pub fn sans(size: f32) -> FontId {
    FontId::new(size * TEXT_SCALE, FontFamily::Proportional)
}

pub fn medium(size: f32) -> FontId {
    FontId::new(size * TEXT_SCALE, FontFamily::Name("medium".into()))
}

pub fn mono(size: f32) -> FontId {
    FontId::new(size, FontFamily::Monospace)
}

pub fn status_color(status: Status) -> Color32 {
    match status {
        Status::Calm => GREEN(),
        Status::Neutral => MUTED(),
        Status::Elevated => AMBER(),
        Status::Stressed => RED(),
    }
}

/// Green for gains, red for losses, muted when flat or unknown.
pub fn move_color(value: Option<f64>) -> Color32 {
    match value {
        Some(v) if v > 0.0 => GREEN(),
        Some(v) if v < 0.0 => RED(),
        _ => MUTED(),
    }
}

pub fn delta_color(delta: Option<Delta>) -> Color32 {
    match delta {
        Some(d) if d.is_flat() => MUTED(),
        Some(d) => move_color(Some(d.value())),
        None => FAINT(),
    }
}

// ---- Omarchy discovery -------------------------------------------------------

fn omarchy_dir() -> Option<PathBuf> {
    Some(PathBuf::from(std::env::var_os("HOME")?).join(".config/omarchy/current"))
}

fn command(cmd: &str, args: &[&str]) -> Option<String> {
    let out = std::process::Command::new(cmd).args(args).output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
}

fn hypr_rounding() -> Option<u8> {
    std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE")?;
    let json = command("hyprctl", &["getoption", "decoration:rounding", "-j"])?;
    let v: serde_json::Value = serde_json::from_str(&json).ok()?;
    Some(v.get("int")?.as_u64()?.min(255) as u8)
}

fn load_palette() -> Palette {
    let rounding = hypr_rounding().unwrap_or(Palette::BUILTIN.rounding);
    omarchy_dir()
        .and_then(|d| std::fs::read_to_string(d.join("theme/colors.toml")).ok())
        .and_then(|src| Palette::from_colors_toml(&src, rounding))
        .unwrap_or(Palette { rounding, ..Palette::BUILTIN })
}

/// Regular and medium/bold files for the Omarchy system font, via fontconfig.
fn system_font_files() -> (Option<String>, Option<String>) {
    let family = command("omarchy-font-current", &[]).unwrap_or_else(|| "JetBrainsMono Nerd Font".into());
    let file = |style: &str| {
        command("fc-match", &["-f", "%{file}", &format!("{family}:style={style}")])
            .filter(|f| f.ends_with(".ttf") || f.ends_with(".otf"))
    };
    let regular = file("Regular");
    let medium = file("Medium")
        .filter(|m| Some(m) != regular.as_ref())
        .or_else(|| file("SemiBold"))
        .or_else(|| file("Bold"));
    (regular, medium)
}

type Stamp = (Option<SystemTime>, Option<SystemTime>, Option<String>);

#[derive(Default)]
struct Watch {
    checked: Option<Instant>,
    stamp: Option<Stamp>,
}

/// What a theme or font switch touches: `theme.name` and `colors.toml` change on
/// `omarchy-theme-set`; the alacritty font line changes on `omarchy-font-set`.
fn stamp() -> Stamp {
    let dir = omarchy_dir();
    let mtime = |rel: &str| {
        dir.as_ref()
            .and_then(|d| std::fs::metadata(d.join(rel)).ok())
            .and_then(|m| m.modified().ok())
    };
    let font = std::env::var_os("HOME").and_then(|h| {
        std::fs::read_to_string(PathBuf::from(h).join(".config/alacritty/alacritty.toml"))
            .ok()
            .and_then(|s| s.lines().find(|l| l.trim_start().starts_with("family")).map(str::to_string))
    });
    (mtime("theme.name"), mtime("theme/colors.toml"), font)
}

/// Re-apply the style when the Omarchy theme or font changed. Cheap: stats twice a second.
pub fn watch(ctx: &egui::Context) {
    let changed = WATCH.with_borrow_mut(|w| {
        if w.checked.is_some_and(|t| t.elapsed() < Duration::from_millis(500)) {
            return false;
        }
        w.checked = Some(Instant::now());
        let now = stamp();
        let changed = w.stamp.as_ref().is_some_and(|s| *s != now);
        w.stamp = Some(now);
        changed
    });
    if changed {
        apply_style(ctx);
        ctx.request_repaint();
    }
}

pub fn apply_style(ctx: &egui::Context) {
    let palette = load_palette();
    PALETTE.with(|c| c.set(palette));
    WATCH.with_borrow_mut(|w| {
        if w.stamp.is_none() {
            w.stamp = Some(stamp());
        }
    });

    let mut fonts = egui::FontDefinitions::default();
    let defaults_sans = fonts.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    let defaults_mono = fonts.families.get(&FontFamily::Monospace).cloned().unwrap_or_default();
    let mut load = |name: &str, path: Option<&str>| {
        let bytes = std::fs::read(path?).ok()?;
        fonts.font_data.insert(name.into(), std::sync::Arc::new(egui::FontData::from_owned(bytes)));
        Some(name.to_string())
    };
    // Omarchy draws its whole shell (bar, launcher, notifications) in one monospace
    // family, so every text style uses it. Noto stays as a glyph fallback.
    let (regular, bold) = system_font_files();
    let system = load("system", regular.as_deref())
        .or_else(|| load("system", Some("/usr/share/fonts/TTF/JetBrainsMonoNerdFont-Regular.ttf")));
    let system_bold = load("system-bold", bold.as_deref());
    let noto = load("noto-sans", Some("/usr/share/fonts/noto/NotoSans-Regular.ttf"));
    let chain = |first: &[&Option<String>], rest: &[String]| {
        first.iter().filter_map(|f| (*f).clone()).chain(rest.iter().cloned()).collect::<Vec<_>>()
    };
    fonts.families.insert(FontFamily::Proportional, chain(&[&system, &noto], &defaults_sans));
    fonts.families.insert(FontFamily::Monospace, chain(&[&system, &noto], &defaults_mono));
    fonts.families.insert(
        FontFamily::Name("medium".into()),
        chain(&[&system_bold, &system, &noto], &defaults_sans),
    );
    ctx.set_fonts(fonts);

    let radius = RADIUS();
    let mut visuals = if palette.light { egui::Visuals::light() } else { egui::Visuals::dark() };
    visuals.panel_fill = BG();
    visuals.window_fill = SURFACE();
    // Floating panels look like Omarchy's launcher: flat, outlined in the accent.
    visuals.window_stroke = Stroke::new(1.0, ACCENT());
    visuals.window_corner_radius = CornerRadius::same(radius);
    visuals.window_shadow = Shadow::NONE;
    visuals.popup_shadow = Shadow::NONE;
    visuals.menu_corner_radius = CornerRadius::same(radius);
    visuals.extreme_bg_color = INPUT();
    visuals.faint_bg_color = SURFACE();
    visuals.code_bg_color = RAISED();
    visuals.override_text_color = None;
    visuals.hyperlink_color = ACCENT();
    visuals.selection.bg_fill = palette.selection;
    visuals.selection.stroke = Stroke::new(1.0, ACCENT());
    visuals.text_cursor.stroke = Stroke::new(2.0, ACCENT());
    for widget in [
        &mut visuals.widgets.noninteractive,
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        widget.corner_radius = corner(radius.min(5));
        widget.expansion = 0.0;
    }
    visuals.widgets.noninteractive.bg_fill = SURFACE();
    visuals.widgets.noninteractive.weak_bg_fill = SURFACE();
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER());
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT());
    visuals.widgets.inactive.bg_fill = RAISED();
    visuals.widgets.inactive.weak_bg_fill = RAISED();
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER());
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, MUTED());
    visuals.widgets.hovered.bg_fill = HOVER();
    visuals.widgets.hovered.weak_bg_fill = HOVER();
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, BORDER_STRONG());
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, TEXT());
    visuals.widgets.active.bg_fill = HOVER();
    visuals.widgets.active.weak_bg_fill = HOVER();
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT());
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, TEXT());
    visuals.widgets.open = visuals.widgets.active;
    let theme = if palette.light { egui::Theme::Light } else { egui::Theme::Dark };
    ctx.set_theme(theme);
    ctx.set_visuals_of(theme, visuals);

    ctx.style_mut_of(theme, |style| {
        style.text_styles = [
            (TextStyle::Small, sans(11.0)),
            (TextStyle::Body, sans(13.0)),
            (TextStyle::Button, sans(12.5)),
            (TextStyle::Heading, medium(17.0)),
            (TextStyle::Monospace, mono(12.0)),
        ]
        .into();
        style.spacing.item_spacing = Vec2::new(8.0, 6.0);
        style.spacing.button_padding = Vec2::new(9.0, 4.0);
        style.spacing.interact_size = Vec2::new(36.0, 24.0);
        style.spacing.window_margin = Margin::same(16);
        style.spacing.menu_margin = Margin::same(6);
        style.spacing.scroll = ScrollStyle::thin();
        style.interaction.tooltip_delay = 0.25;
        style.interaction.show_tooltips_only_when_still = false;
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_omarchy_colors_toml() {
        let src = "# comment\nmode = \"light\"\naccent = \"#c5003c\"\nbackground = \"#0f0508\"\nforeground = \"#ffffff\"\nred = \"#ff0000\"\n";
        let p = Palette::from_colors_toml(src, 0).unwrap();
        assert!(p.light);
        assert_eq!(p.accent, Color32::from_rgb(0xc5, 0x00, 0x3c));
        assert_eq!(p.red, Color32::from_rgb(255, 0, 0));
        assert_eq!(p.text, Color32::WHITE);
        assert_eq!(p.rounding, 0);
        assert!(Palette::from_colors_toml("accent = \"#fff\"", 0).is_none());
    }
}

//! X conversations are a separate, dated interpretation layer.
use crate::app::{Desk, Desktop};
use crate::theme::{medium, sans, AMBER, FAINT, GAP, GREEN, MUTED, RED, TEXT};
use crate::widgets;
use dollar_dashboard::model::{fmt_age, now_unix};
use dollar_dashboard::pulse::{Sector, Tone};
use eframe::egui::{self, Color32, RichText, Ui};

fn tone_color(tone: Tone) -> Color32 {
    match tone {
        Tone::Bullish => GREEN(),
        Tone::Bearish => RED(),
        Tone::Mixed => AMBER(),
        Tone::Neutral => MUTED(),
    }
}

fn freshness(app: &Desktop) -> String {
    app.pulse
        .as_ref()
        .map(|p| {
            format!(
                "{} · updated {}",
                if p.stale(now_unix()) {
                    "STALE"
                } else {
                    "Cached report"
                },
                fmt_age(p.requested_unix, now_unix())
            )
        })
        .unwrap_or_else(|| "No X report yet".into())
}

pub(crate) fn overview(app: &mut Desktop, ui: &mut Ui) {
    let meta = freshness(app);
    widgets::panel(ui, "X vibe check", &meta, |ui| {
        if let Some(pulse) = &app.pulse {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(pulse.report.tone.label())
                        .font(medium(13.0))
                        .color(tone_color(pulse.report.tone)),
                );
                ui.label(
                    RichText::new(format!(
                        "{} confidence · {} linked posts · 48h search window",
                        pulse.report.confidence.label(),
                        pulse.source_count()
                    ))
                    .font(sans(11.0))
                    .color(FAINT()),
                );
            });
            ui.label(
                RichText::new(&pulse.report.summary)
                    .font(sans(12.0))
                    .color(TEXT()),
            );
            let topics = pulse
                .report
                .topics
                .iter()
                .take(3)
                .map(|t| t.headline.as_str())
                .collect::<Vec<_>>()
                .join(" · ");
            ui.label(RichText::new(topics).font(sans(11.0)).color(MUTED()));
        } else {
            ui.label(RichText::new("What people are talking about: Bitcoin, macro, AI, nuclear and Zcash. Import a sourced brief from Grok chat.").font(sans(12.0)).color(MUTED()));
        }
        if app.pulse_busy {
            ui.label(RichText::new("Grok is searching X…").color(FAINT()));
        }
        if !app.pulse_error.is_empty() {
            ui.label(
                RichText::new(&app.pulse_error)
                    .font(sans(11.0))
                    .color(AMBER()),
            );
        }
        if ui.small_button("Open X Pulse →").clicked() {
            app.navigate(Desk::Pulse);
        }
    });
}

pub(crate) fn show(app: &mut Desktop, ui: &mut Ui) {
    ui.label(
        RichText::new("What X is talking about")
            .font(medium(21.0))
            .color(TEXT()),
    );
    ui.label(
        RichText::new(
            "A qualitative read of searched posts · Grok interpretation · 48-hour window",
        )
        .font(sans(12.0))
        .color(MUTED()),
    );
    ui.add_space(8.0);
    ui.horizontal_wrapped(|ui| {
        if ui.button("Copy Grok prompt").on_hover_text("Paste into Grok chat using your subscription, then copy its JSON brief back here.").clicked() {
            let now = now_unix();
            ui.ctx().copy_text(dollar_dashboard::pulse::research_prompt(now));
            app.pulse_prompt_at = Some(now);
            app.notice = "X research prompt copied. Paste it into Grok chat, then import its JSON brief in X Pulse.".into();
        }
        ui.label(RichText::new(freshness(app)).font(sans(11.0)).color(FAINT()));
    });
    egui::CollapsingHeader::new("Import brief from Grok chat")
        .default_open(app.pulse.is_none())
        .show(ui, |ui| {
            ui.label("1. Copy the prompt above and paste it into Grok chat with X search enabled.");
            ui.label("2. Copy its complete JSON answer into this box, then import.");
            ui.add(egui::TextEdit::multiline(&mut app.pulse_import).desired_rows(4).desired_width(f32::INFINITY).char_limit(200_000).hint_text("Paste Grok's JSON brief here…"));
            if ui.add_enabled(!app.pulse_busy && !app.pulse_import.trim().is_empty(), egui::Button::new("Import brief")).clicked() { app.import_pulse(); }
            widgets::footnote(ui, "Uses your Grok chat subscription. Links and post dates are checked; the app cannot independently verify that a pasted brief actually searched X.");
        });
    egui::CollapsingHeader::new("Automatic search · optional xAI API")
        .show(ui, |ui| {
            let available = dollar_dashboard::pulse::api_available();
            widgets::footnote(ui, "Requires XAI_API_KEY in the app's environment. API searches are separately billed; they do not use the Grok chat subscription. No API request runs until you enable it.");
            ui.horizontal_wrapped(|ui| {
                if ui.add_enabled(available && !app.pulse_busy, egui::Button::new(if app.pulse_busy { "Searching X…" } else { "Refresh via xAI API" })).clicked() { app.refresh_pulse(); }
                ui.add_enabled(available, egui::Checkbox::new(&mut app.prefs.x_pulse_hourly, "Refresh hourly while open"));
            });
        });
    if app.pulse_busy {
        ui.label(RichText::new("Searching multiple viewpoints. This can take up to three minutes; prices continue updating.").font(sans(11.5)).color(FAINT()));
    }
    if !app.pulse_error.is_empty() {
        ui.label(RichText::new(&app.pulse_error).color(AMBER()));
    }
    ui.add_space(GAP);
    let Some(pulse) = &app.pulse else {
        widgets::panel(ui, "Your first X vibe check", "", |ui| {
            ui.label("Copy the prompt, run it in Grok chat, and import the answer above.");
            widgets::footnote(ui, "The overview and this desk will show the latest successful report offline. No sentiment is filled in before a sourced brief is received.");
        });
        return;
    };
    let report = &pulse.report;
    widgets::panel(
        ui,
        "The vibe",
        &format!("{} distinct linked posts", pulse.source_count()),
        |ui| {
            widgets::footnote(ui, pulse.origin.label());
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(report.tone.label())
                        .font(medium(16.0))
                        .color(tone_color(report.tone)),
                );
                ui.label(
                    RichText::new(format!(
                        "{} confidence in the sampled narrative",
                        report.confidence.label()
                    ))
                    .font(sans(11.5))
                    .color(MUTED()),
                );
            });
            ui.label(
                RichText::new(&report.summary)
                    .font(sans(13.0))
                    .color(TEXT()),
            );
            widgets::footnote(ui, "Searches cover the preceding 48 hours. Linked-post counts describe the evidence shown, not all posts searched or X-wide opinion. Sentiment is a model interpretation, not a measured population statistic.");
        },
    );
    ui.add_space(GAP);
    ui.horizontal_wrapped(|ui| {
        ui.selectable_value(&mut app.pulse_sector, None, "All topics");
        for sector in Sector::ALL {
            if report.topics.iter().any(|t| t.sector == sector) {
                ui.selectable_value(&mut app.pulse_sector, Some(sector), sector.label());
            }
        }
    });
    ui.add_space(6.0);
    for topic in report
        .topics
        .iter()
        .filter(|t| app.pulse_sector.is_none_or(|s| s == t.sector))
    {
        widgets::panel(ui, topic.sector.label(), "Searched conversation", |ui| {
            ui.label(
                RichText::new(&topic.headline)
                    .font(medium(14.0))
                    .color(TEXT()),
            );
            ui.horizontal_wrapped(|ui| {
                widgets::caps(ui, topic.tone.label(), tone_color(topic.tone));
                ui.label(
                    RichText::new(format!("{} confidence", topic.confidence.label()))
                        .font(sans(11.0))
                        .color(FAINT()),
                );
            });
            ui.label(RichText::new(&topic.summary).font(sans(12.5)).color(TEXT()));
            ui.add_space(5.0);
            for (label, text, color) in [
                ("Bull case", &topic.bullish_case, GREEN()),
                ("Bear case", &topic.bearish_case, RED()),
                ("Watch next", &topic.watch_next, MUTED()),
            ] {
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new(format!("{label}: "))
                            .font(sans(11.5))
                            .color(color),
                    );
                    ui.label(RichText::new(text).font(sans(11.5)).color(MUTED()));
                });
            }
            ui.add_space(5.0);
            ui.horizontal_wrapped(|ui| {
                for source in &topic.sources {
                    let date = source
                        .published_unix()
                        .and_then(|t| chrono::DateTime::from_timestamp(t, 0))
                        .map(|t| t.format("%d %b %H:%M UTC").to_string())
                        .unwrap_or_default();
                    ui.hyperlink_to(format!("@{} · {date}", source.author()), &source.url);
                }
            });
        });
        ui.add_space(GAP);
    }
    widgets::panel(ui, "Coverage & uncertainty", "", |ui| {
        ui.label(
            RichText::new(&report.limitations)
                .font(sans(11.5))
                .color(MUTED()),
        );
        let missing = Sector::ALL
            .into_iter()
            .filter(|s| *s != Sector::Other && !report.topics.iter().any(|t| t.sector == *s))
            .map(Sector::label)
            .collect::<Vec<_>>();
        if !missing.is_empty() {
            widgets::footnote(
                ui,
                &format!("No sourced topic returned for: {}.", missing.join(", ")),
            );
        }
    });
}

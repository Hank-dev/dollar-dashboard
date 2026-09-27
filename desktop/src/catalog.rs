use crate::model::Status;
use serde::Deserialize;

pub const AI_TICKERS: &[&str] = &["NVDA", "GOOGL", "MSFT", "AMZN", "AVGO", "META", "SPCX"];
pub const NUCLEAR_TICKERS: &[&str] = &["CEG", "VST", "CCJ", "BWXT", "OKLO", "UEC", "SMR", "LEU"];
pub const URANIUM_ETFS: &[&str] = &["URA", "URNM", "NLR"];

const AI_ROSTER: &[(&str, &str, &str, Option<&str>, &str)] = &[
    ("nvidia", "public", "NVIDIA", Some("NVDA"), "AI compute"),
    (
        "alphabet",
        "public",
        "Alphabet",
        Some("GOOGL"),
        "Models, search, cloud",
    ),
    (
        "microsoft",
        "public",
        "Microsoft",
        Some("MSFT"),
        "Enterprise agents",
    ),
    (
        "amazon",
        "public",
        "Amazon",
        Some("AMZN"),
        "Cloud and retail agents",
    ),
    (
        "broadcom",
        "public",
        "Broadcom",
        Some("AVGO"),
        "AI networking and ASICs",
    ),
    (
        "meta",
        "public",
        "Meta",
        Some("META"),
        "Consumer AI and open models",
    ),
    (
        "openai",
        "private",
        "OpenAI",
        None,
        "Frontier lab and agent platform",
    ),
    ("anthropic", "private", "Anthropic", None, "Frontier lab"),
    (
        "xai",
        "public",
        "SpaceX / xAI",
        Some("SPCX"),
        "Frontier lab and compute",
    ),
    (
        "databricks",
        "private",
        "Databricks",
        None,
        "Data and AI platform",
    ),
    (
        "cursor",
        "private",
        "Cursor / Anysphere",
        None,
        "Coding agent",
    ),
    (
        "perplexity",
        "private",
        "Perplexity",
        None,
        "Answer engine and agents",
    ),
];

const AI_METRICS: &[(&str, &str, &str)] = &[
    ("capex-race", "Capital", "Hyperscaler AI capex"),
    ("private-valuation", "Capital", "Private lab valuations"),
    ("consumer-scale", "Adoption", "ChatGPT reach"),
    ("coding-agent", "Adoption", "Coding assistants"),
    ("open-pressure", "Technology", "Open-model pressure"),
    ("agent-reliability", "Risk", "Agent reliability"),
];

const AI_SIGNALS: &[(&str, &str)] = &[
    ("frontier-models", "Models"),
    ("agents", "Agents"),
    ("ai-ides", "Tools"),
    ("compute-stack", "Infra"),
];

#[derive(Clone, Debug)]
pub struct PlayerCard {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub ticker: Option<String>,
    pub category: String,
    pub scale: String,
    pub role: String,
    pub exposure: String,
    pub status: Status,
    pub source: String,
    pub as_of: String,
}

#[derive(Clone, Debug)]
pub struct NoteCard {
    pub id: String,
    pub group: String,
    pub label: String,
    pub value: String,
    pub status: Status,
    pub context: String,
    pub detail: String,
    pub watch: String,
    pub source: String,
    pub as_of: String,
}

#[derive(Clone, Debug)]
pub struct Catalog {
    pub ai_verdict: String,
    pub ai_as_of: String,
    pub ai_players: Vec<PlayerCard>,
    pub ai_metrics: Vec<NoteCard>,
    pub ai_signals: Vec<NoteCard>,
    pub nuclear_verdict: String,
    pub nuclear_as_of: String,
    pub nuclear_players: Vec<PlayerCard>,
    pub nuclear_metrics: Vec<NoteCard>,
    pub nuclear_signals: Vec<NoteCard>,
}

pub fn load() -> Catalog {
    let ai = parse_ai(include_str!("../assets/ai_snapshot.json"));
    let nuclear = parse_nuclear(include_str!("../assets/nuclear.json"));
    Catalog {
        ai_verdict: ai.0,
        ai_as_of: ai.1,
        ai_players: ai.2,
        ai_metrics: ai.3,
        ai_signals: ai.4,
        nuclear_verdict: nuclear.0,
        nuclear_as_of: nuclear.1,
        nuclear_players: nuclear.2,
        nuclear_metrics: nuclear.3,
        nuclear_signals: nuclear.4,
    }
}

fn parse_ai(
    text: &str,
) -> (
    String,
    String,
    Vec<PlayerCard>,
    Vec<NoteCard>,
    Vec<NoteCard>,
) {
    let file: AiFile = serde_json::from_str(text).unwrap_or_else(|err| {
        panic!("embedded AI snapshot is not readable: {err}");
    });
    let players = AI_ROSTER
        .iter()
        .filter_map(|(id, kind, name, ticker, category)| {
            let raw = file.players.get(*id)?;
            Some(PlayerCard {
                id: (*id).into(),
                kind: (*kind).into(),
                name: (*name).into(),
                ticker: ticker.map(str::to_string),
                category: (*category).into(),
                scale: raw
                    .market_cap
                    .clone()
                    .or_else(|| raw.valuation_estimate.clone())
                    .unwrap_or_else(|| "—".into()),
                role: raw.adoption_signal.clone(),
                exposure: raw.ai_exposure.clone(),
                status: Status::from_snapshot(&raw.status),
                source: raw.source.name.clone(),
                as_of: raw.source.as_of.clone(),
            })
        })
        .collect();
    let metrics = AI_METRICS
        .iter()
        .filter_map(|(id, group, label)| {
            let raw = file.market_metrics.get(*id)?;
            Some(NoteCard {
                id: (*id).into(),
                group: (*group).into(),
                label: (*label).into(),
                value: raw.value.clone(),
                status: Status::from_snapshot(&raw.status),
                context: raw.context.clone(),
                detail: raw.detail.clone(),
                watch: String::new(),
                source: raw.source.name.clone(),
                as_of: raw.source.as_of.clone(),
            })
        })
        .collect();
    let signals = AI_SIGNALS
        .iter()
        .filter_map(|(id, track)| {
            let raw = file.tech_signals.get(*id)?;
            Some(NoteCard {
                id: (*id).into(),
                group: (*track).into(),
                label: raw.label.clone(),
                value: String::new(),
                status: Status::from_snapshot(&raw.status),
                context: String::new(),
                detail: raw.summary.clone(),
                watch: raw.watch_next.clone(),
                source: raw.source.name.clone(),
                as_of: raw.source.as_of.clone(),
            })
        })
        .collect();
    (
        file.verdict.text,
        file.verdict.as_of,
        players,
        metrics,
        signals,
    )
}

fn parse_nuclear(
    text: &str,
) -> (
    String,
    String,
    Vec<PlayerCard>,
    Vec<NoteCard>,
    Vec<NoteCard>,
) {
    let file: NuclearFile = serde_json::from_str(text).unwrap_or_else(|err| {
        panic!("embedded nuclear snapshot is not readable: {err}");
    });
    let players = file
        .players
        .into_iter()
        .map(|raw| PlayerCard {
            id: raw.id,
            kind: raw.kind,
            name: raw.name,
            ticker: raw.ticker,
            category: raw.category,
            scale: raw.scale,
            role: raw.role,
            exposure: raw.exposure,
            status: Status::from_snapshot(&raw.status),
            source: raw.source,
            as_of: raw.as_of,
        })
        .collect();
    let metrics = file
        .metrics
        .into_iter()
        .map(|raw| NoteCard {
            id: raw.id,
            group: raw.group,
            label: raw.label,
            value: raw.value,
            status: Status::from_snapshot(&raw.status),
            context: raw.context,
            detail: raw.detail,
            watch: String::new(),
            source: raw.source,
            as_of: raw.as_of,
        })
        .collect();
    let signals = file
        .signals
        .into_iter()
        .map(|raw| NoteCard {
            id: raw.id,
            group: raw.track,
            label: raw.label,
            value: String::new(),
            status: Status::from_snapshot(&raw.status),
            context: String::new(),
            detail: raw.summary,
            watch: raw.watch,
            source: raw.source,
            as_of: raw.as_of,
        })
        .collect();
    (file.verdict, file.as_of, players, metrics, signals)
}

#[derive(Deserialize)]
struct AiFile {
    verdict: AiVerdict,
    players: std::collections::BTreeMap<String, AiPlayerRaw>,
    #[serde(rename = "marketMetrics")]
    market_metrics: std::collections::BTreeMap<String, AiMetricRaw>,
    #[serde(rename = "techSignals")]
    tech_signals: std::collections::BTreeMap<String, AiSignalRaw>,
}

#[derive(Deserialize)]
struct AiVerdict {
    text: String,
    #[serde(rename = "asOf")]
    as_of: String,
}

#[derive(Deserialize)]
struct AiPlayerRaw {
    #[serde(rename = "marketCap")]
    market_cap: Option<String>,
    #[serde(rename = "valuationEstimate")]
    valuation_estimate: Option<String>,
    #[serde(rename = "adoptionSignal")]
    adoption_signal: String,
    #[serde(rename = "aiExposure")]
    ai_exposure: String,
    status: String,
    source: AiSource,
}

#[derive(Deserialize)]
struct AiMetricRaw {
    value: String,
    context: String,
    detail: String,
    status: String,
    source: AiSource,
}

#[derive(Deserialize)]
struct AiSignalRaw {
    label: String,
    summary: String,
    #[serde(rename = "watchNext")]
    watch_next: String,
    status: String,
    source: AiSource,
}

#[derive(Deserialize)]
struct AiSource {
    name: String,
    #[serde(rename = "asOf")]
    as_of: String,
}

#[derive(Deserialize)]
struct NuclearFile {
    as_of: String,
    verdict: String,
    players: Vec<NuclearPlayer>,
    metrics: Vec<NuclearMetric>,
    signals: Vec<NuclearSignal>,
}

#[derive(Deserialize)]
struct NuclearPlayer {
    id: String,
    kind: String,
    name: String,
    ticker: Option<String>,
    category: String,
    scale: String,
    role: String,
    exposure: String,
    status: String,
    source: String,
    as_of: String,
}

#[derive(Deserialize)]
struct NuclearMetric {
    id: String,
    group: String,
    label: String,
    value: String,
    status: String,
    context: String,
    detail: String,
    source: String,
    as_of: String,
}

#[derive(Deserialize)]
struct NuclearSignal {
    id: String,
    track: String,
    label: String,
    status: String,
    summary: String,
    watch: String,
    source: String,
    as_of: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_catalogs_cover_every_desk_row() {
        let catalog = load();
        assert_eq!(catalog.ai_players.len(), AI_ROSTER.len());
        assert_eq!(catalog.ai_metrics.len(), AI_METRICS.len());
        assert_eq!(catalog.ai_signals.len(), AI_SIGNALS.len());
        assert!(catalog.ai_verdict.contains("capital cycle"));
        assert_eq!(catalog.nuclear_players.len(), 12);
        assert_eq!(catalog.nuclear_metrics.len(), 6);
        assert_eq!(catalog.nuclear_signals.len(), 6);
        assert!(catalog.nuclear_verdict.contains("400 GWe"));
    }
}

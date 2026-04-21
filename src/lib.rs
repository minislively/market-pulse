use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug)]
struct Asset {
    symbol: &'static str,
    label: &'static str,
    unit: &'static str,
    value: Option<f64>,
    change: Option<f64>,
    note: Option<String>,
}

#[derive(Clone, Debug)]
struct Pulse {
    timestamp: String,
    session: String,
    basis: Vec<String>,
    mood: String,
    assets: Vec<Asset>,
    drivers: Vec<String>,
    tensions: Vec<String>,
    question: String,
    concept: String,
    notes: Vec<String>,
}

#[derive(Clone, Debug)]
struct Regime {
    timestamp: String,
    basis: Vec<String>,
    label: String,
    assets: Vec<Asset>,
    drivers: Vec<String>,
    tensions: Vec<String>,
    checks: Vec<String>,
    question: String,
    notes: Vec<String>,
}

#[derive(Clone, Debug)]
struct Inquiry {
    timestamp: String,
    question: String,
    linked: Option<String>,
    thesis_type: String,
    breakdown: Vec<String>,
    explanations: Vec<String>,
    evidence: Vec<String>,
    counter: Vec<String>,
    next_question: String,
    concepts: Vec<String>,
}

#[derive(Clone, Debug)]
struct ResearchQuery {
    question: String,
    linked: Option<String>,
}

#[derive(Clone, Debug)]
struct ResearchSource {
    title: String,
    publisher: String,
    url: String,
    published_at: Option<String>,
    relevance: String,
    evidence: String,
}

#[derive(Clone, Debug)]
struct ResearchBundle {
    provider: String,
    sources: Vec<ResearchSource>,
    notes: Vec<String>,
}

trait ResearchProvider {
    fn name(&self) -> &'static str;
    fn research(&self, query: &ResearchQuery) -> Result<ResearchBundle, String>;
}

struct NoopResearchProvider;

impl ResearchProvider for NoopResearchProvider {
    fn name(&self) -> &'static str {
        "noop"
    }

    fn research(&self, query: &ResearchQuery) -> Result<ResearchBundle, String> {
        let linked_note = query
            .linked
            .as_ref()
            .map(|ts| format!("linked to latest pulse {ts}"))
            .unwrap_or_else(|| "no prior pulse linked".into());
        Ok(ResearchBundle {
            provider: self.name().into(),
            sources: Vec::new(),
            notes: vec![
                "No built-in live RSS/API provider is configured; research mode is source-optional."
                    .into(),
                format!("{linked_note}; question scaffold is inference-only until sources are supplied."),
            ],
        })
    }
}

struct SearchCommandProvider {
    template: String,
}

impl ResearchProvider for SearchCommandProvider {
    fn name(&self) -> &'static str {
        "search-cmd"
    }

    fn research(&self, query: &ResearchQuery) -> Result<ResearchBundle, String> {
        let args = search_command_args(&self.template, &query.question)?;
        let output = run_command_with_timeout(&args, Duration::from_secs(5))?;
        if !output.status.success() {
            return Err(format!(
                "search command exited with status {}",
                output
                    .status
                    .code()
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "unknown".into())
            ));
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        let (sources, invalid_rows) = parse_search_jsonl(&stdout, 20);
        let mut notes = vec![
            "MARKET_PULSE_SEARCH_CMD supplied structured source metadata.".into(),
            "External command output is treated as source material; market-pulse still generates the reasoning scaffold.".into(),
        ];
        if invalid_rows > 0 {
            notes.push(format!(
                "{invalid_rows} invalid JSONL source row(s) skipped."
            ));
        }
        if sources.is_empty() {
            notes.push("Search command returned no valid JSONL source rows; falling back to inference scaffolding.".into());
        }
        Ok(ResearchBundle {
            provider: self.name().into(),
            sources,
            notes,
        })
    }
}

#[derive(Clone, Debug)]
struct Feedback {
    timestamp: String,
    thought: String,
    linked: Option<String>,
    claim: String,
    thesis_type: String,
    good: Vec<String>,
    check: Vec<String>,
    counter: Vec<String>,
    next: Vec<String>,
    concepts: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum CommandKind {
    Now,
    Regime,
    Think,
    Review,
    Help,
    Inquiry { text: String, no_save: bool },
    Research { text: String, no_save: bool },
}

const PULSE_QUOTE_BASIS: &[&str] = &[
    "time: local machine timestamp and session label",
    "change: latest Yahoo regularMarketPrice vs chartPreviousClose, usually the prior regular-session close",
    "window: Yahoo chart range=5d interval=1d; this is a session/daily pulse, not a weekly return",
];

const REGIME_QUOTE_BASIS: &[&str] = &[
    "time: local machine timestamp; regime is broader than today's pulse",
    "change: latest Yahoo regularMarketPrice vs first available close in the chart window",
    "window: Yahoo chart range=3mo interval=1wk; this is a 1-3 month regime read, not a trading signal",
];

const SYMBOLS: &[(&str, &str, &str)] = &[
    ("^GSPC", "S&P 500", ""),
    ("^IXIC", "Nasdaq", ""),
    ("^KS11", "KOSPI", ""),
    ("KRW=X", "USD/KRW", "KRW"),
    ("DX-Y.NYB", "DXY", ""),
    ("^TNX", "US 10Y", "%"),
    ("CL=F", "WTI", "USD"),
    ("GC=F", "Gold", "USD"),
    ("BTC-USD", "BTC", "USD"),
];

pub fn main_entry() {
    if let Err(err) = run(env::args().skip(1).collect()) {
        eprintln!("mp: {err}");
        std::process::exit(1);
    }
}

fn run(args: Vec<String>) -> Result<(), String> {
    match parse_command(&args)? {
        CommandKind::Now => now(&args),
        CommandKind::Regime => regime(&args),
        CommandKind::Think => think(&args),
        CommandKind::Review => review(&args),
        CommandKind::Help => {
            print_help();
            Ok(())
        }
        CommandKind::Inquiry { text, no_save } => inquiry(&text, no_save),
        CommandKind::Research { text, no_save } => research_inquiry(&text, no_save),
    }
}

fn parse_command(args: &[String]) -> Result<CommandKind, String> {
    match args.first().map(String::as_str) {
        None => Ok(CommandKind::Now),
        Some("now") => Ok(CommandKind::Now),
        Some("regime") => Ok(CommandKind::Regime),
        Some("think") => Ok(CommandKind::Think),
        Some("review") => Ok(CommandKind::Review),
        Some("help") | Some("--help") | Some("-h") => Ok(CommandKind::Help),
        Some("ask") => inquiry_command(&args[1..], "`mp ask` needs a question"),
        Some("research") => research_command(&args[1..], "`mp research` needs a question"),
        Some(first) if first.starts_with('-') => Err(format!("unknown option '{first}'")),
        Some(_) => inquiry_command(args, "`mp` needs a market question"),
    }
}

fn inquiry_command(args: &[String], empty_error: &str) -> Result<CommandKind, String> {
    let (text, no_save, research) = collect_question_args(args);
    if text.is_empty() {
        return Err(empty_error.into());
    }
    if research {
        Ok(CommandKind::Research { text, no_save })
    } else {
        Ok(CommandKind::Inquiry { text, no_save })
    }
}

fn research_command(args: &[String], empty_error: &str) -> Result<CommandKind, String> {
    let (text, no_save, _) = collect_question_args(args);
    if text.is_empty() {
        return Err(empty_error.into());
    }
    Ok(CommandKind::Research { text, no_save })
}

fn collect_question_args(args: &[String]) -> (String, bool, bool) {
    let no_save = args.iter().any(|a| a == "--no-save");
    let research = args.iter().any(|a| a == "--research");
    let text = args
        .iter()
        .filter(|a| !matches!(a.as_str(), "--no-save" | "--research"))
        .cloned()
        .collect::<Vec<_>>()
        .join(" ");
    let text = text.trim().to_string();
    (text, no_save, research)
}

fn print_help() {
    println!(
        "Usage:\n  mp \"your market question\" [--no-save]\n  mp \"your market question\" --research [--no-save]\n  mp ask <your market question> [--no-save]\n  mp research <your market question> [--no-save]\n  mp now [--compact] [--no-save]\n  mp regime [--no-save]\n  mp think <your market interpretation> [--no-save]\n  mp review [--limit N] [--date YYYY-MM-DD|--ago N]"
    );
}

fn now(args: &[String]) -> Result<(), String> {
    let compact = args.iter().any(|a| a == "--compact");
    let no_save = args.iter().any(|a| a == "--no-save");
    let pulse = build_pulse();
    if !no_save {
        append_event(&pulse_json(&pulse))?;
    }
    println!("{}", render_pulse(&pulse, compact));
    Ok(())
}

fn regime(args: &[String]) -> Result<(), String> {
    let no_save = args.iter().any(|a| a == "--no-save");
    let regime = build_regime();
    if !no_save {
        append_event(&regime_json(&regime))?;
    }
    println!("{}", render_regime(&regime));
    Ok(())
}

fn think(args: &[String]) -> Result<(), String> {
    let no_save = args.iter().any(|a| a == "--no-save");
    let text = args
        .iter()
        .skip(1)
        .filter(|a| a.as_str() != "--no-save")
        .cloned()
        .collect::<Vec<_>>()
        .join(" ");
    if text.trim().is_empty() {
        return Err("`mp think` needs text".into());
    }
    let linked = latest_pulse_timestamp();
    let feedback = make_feedback(text.trim(), linked.clone());
    if !no_save {
        append_event(&thought_json(text.trim(), linked.as_deref()))?;
        append_event(&feedback_json(&feedback))?;
    }
    println!("{}", render_feedback(&feedback));
    Ok(())
}

fn inquiry(question: &str, no_save: bool) -> Result<(), String> {
    let linked = latest_pulse_timestamp();
    let inquiry = make_inquiry(question.trim(), linked);
    if !no_save {
        append_event(&inquiry_json(&inquiry))?;
    }
    println!("{}", render_inquiry(&inquiry));
    Ok(())
}

fn research_inquiry(question: &str, no_save: bool) -> Result<(), String> {
    let linked = latest_pulse_timestamp();
    let query = ResearchQuery {
        question: question.trim().to_string(),
        linked: linked.clone(),
    };
    let bundle = research_bundle(&query);
    let inquiry = make_inquiry(&query.question, linked);
    if !no_save {
        append_event(&research_inquiry_json(&inquiry, &bundle))?;
    }
    println!("{}", render_research_inquiry(&inquiry, &bundle));
    Ok(())
}

fn research_bundle(query: &ResearchQuery) -> ResearchBundle {
    match env::var("MARKET_PULSE_SEARCH_CMD") {
        Ok(template) if !template.trim().is_empty() => {
            let provider = SearchCommandProvider { template };
            research_bundle_from_provider(&provider, query)
        }
        _ => research_bundle_from_provider(&NoopResearchProvider, query),
    }
}

fn research_bundle_from_provider(
    provider: &dyn ResearchProvider,
    query: &ResearchQuery,
) -> ResearchBundle {
    provider
        .research(query)
        .unwrap_or_else(|err| ResearchBundle {
            provider: provider.name().into(),
            sources: Vec::new(),
            notes: vec![format!("research provider failed gracefully: {err}")],
        })
}

fn search_command_args(template: &str, query: &str) -> Result<Vec<String>, String> {
    let mut saw_query = false;
    let args = split_template_args(template)?
        .into_iter()
        .map(|part| {
            if part.contains("{query}") {
                saw_query = true;
                part.replace("{query}", query)
            } else {
                part
            }
        })
        .collect::<Vec<_>>();
    if args.is_empty() {
        return Err("MARKET_PULSE_SEARCH_CMD is empty".into());
    }
    if !saw_query {
        return Err("MARKET_PULSE_SEARCH_CMD must include {query}".into());
    }
    Ok(args)
}

fn split_template_args(template: &str) -> Result<Vec<String>, String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut escaped = false;
    let mut has_current = false;

    for ch in template.chars() {
        if escaped {
            current.push(match ch {
                'n' if quote == Some('"') => '\n',
                'r' if quote == Some('"') => '\r',
                't' if quote == Some('"') => '\t',
                other => other,
            });
            escaped = false;
            has_current = true;
            continue;
        }

        if ch == '\\' && quote != Some('\'') {
            escaped = true;
            has_current = true;
            continue;
        }

        match quote {
            Some(q) if ch == q => {
                quote = None;
                has_current = true;
            }
            Some(_) => {
                current.push(ch);
                has_current = true;
            }
            None if (ch == '\'' || ch == '"') && current.is_empty() => {
                quote = Some(ch);
                has_current = true;
            }
            None if ch.is_whitespace() => {
                if has_current {
                    args.push(std::mem::take(&mut current));
                    has_current = false;
                }
            }
            None => {
                current.push(ch);
                has_current = true;
            }
        }
    }

    if escaped {
        return Err("MARKET_PULSE_SEARCH_CMD has a dangling escape".into());
    }
    if quote.is_some() {
        return Err("MARKET_PULSE_SEARCH_CMD has an unterminated quote".into());
    }
    if has_current {
        args.push(current);
    }
    Ok(args)
}

fn run_command_with_timeout(
    args: &[String],
    timeout: Duration,
) -> Result<std::process::Output, String> {
    let mut child = Command::new(&args[0])
        .args(&args[1..])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| e.to_string())?;
    let started = Instant::now();
    loop {
        if child.try_wait().map_err(|e| e.to_string())?.is_some() {
            return child.wait_with_output().map_err(|e| e.to_string());
        }
        if started.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!(
                "search command timed out after {}s",
                timeout.as_secs()
            ));
        }
        sleep(Duration::from_millis(25));
    }
}

fn parse_search_jsonl(text: &str, limit: usize) -> (Vec<ResearchSource>, usize) {
    let mut sources = Vec::new();
    let mut invalid = 0;
    for line in text.lines().filter(|l| !l.trim().is_empty()).take(limit) {
        match research_source_from_json_line(line) {
            Some(source) => sources.push(source),
            None => invalid += 1,
        }
    }
    (sources, invalid)
}

fn research_source_from_json_line(line: &str) -> Option<ResearchSource> {
    let title = json_field(line, "title").unwrap_or_else(|| "Untitled source".into());
    let publisher = json_field(line, "publisher").unwrap_or_else(|| "unknown publisher".into());
    let url = json_field(line, "url").unwrap_or_default();
    let evidence = json_field(line, "evidence")?;
    let relevance = json_field(line, "relevance").unwrap_or_else(|| "source evidence".into());
    let published_at = json_field(line, "published_at");
    Some(ResearchSource {
        title,
        publisher,
        url,
        published_at,
        relevance,
        evidence,
    })
}

fn review(args: &[String]) -> Result<(), String> {
    let mut limit = 80usize;
    let mut date: Option<String> = None;
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--limit" {
            if let Some(raw) = args.get(i + 1) {
                limit = raw
                    .parse()
                    .map_err(|_| "--limit must be a number".to_string())?;
            }
            i += 1;
        } else if args[i] == "--date" {
            let Some(raw) = args.get(i + 1) else {
                return Err("--date needs YYYY-MM-DD".into());
            };
            validate_review_date(raw)?;
            if date.is_some() {
                return Err("use only one review date selector".into());
            }
            date = Some(raw.clone());
            i += 1;
        } else if args[i] == "--ago" || args[i] == "--days-ago" {
            let Some(raw) = args.get(i + 1) else {
                return Err(format!("{} needs a number of days", args[i]));
            };
            if date.is_some() {
                return Err("use only one review date selector".into());
            }
            let days = parse_review_days_ago(raw)?;
            date = Some(date_for_days_ago(days)?);
            i += 1;
        }
        i += 1;
    }
    let rendered = if let Some(date) = date {
        render_review_for_date(limit, &date)
    } else {
        render_review(limit)
    };
    println!("{rendered}");
    Ok(())
}

fn parse_review_days_ago(raw: &str) -> Result<u32, String> {
    let days = raw
        .parse::<u32>()
        .map_err(|_| "--ago must be a non-negative whole number".to_string())?;
    if days <= 3660 {
        Ok(days)
    } else {
        Err("--ago must be 3660 days or less".into())
    }
}

fn date_for_days_ago(days: u32) -> Result<String, String> {
    if days == 0 {
        return command_date(&["+%Y-%m-%d"])
            .ok_or_else(|| "--ago needs the local `date` command".into());
    }
    let bsd_offset = format!("-v-{days}d");
    if let Some(date) = command_date(&[&bsd_offset, "+%Y-%m-%d"]) {
        return Ok(date);
    }
    let gnu_relative = format!("{days} days ago");
    command_date(&["-d", &gnu_relative, "+%Y-%m-%d"])
        .ok_or_else(|| "--ago needs BSD `date -v` or GNU `date -d` support".into())
}

fn command_date(args: &[&str]) -> Option<String> {
    let output = Command::new("date").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let date = String::from_utf8(output.stdout).ok()?.trim().to_string();
    validate_review_date(&date).ok()?;
    Some(date)
}

fn validate_review_date(date: &str) -> Result<(), String> {
    let bytes = date.as_bytes();
    let shape_valid = bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(i, b)| i == 4 || i == 7 || b.is_ascii_digit());
    if !shape_valid {
        return Err("--date must use YYYY-MM-DD".into());
    }
    let month = date[5..7].parse::<u32>().unwrap_or(0);
    let day = date[8..10].parse::<u32>().unwrap_or(0);
    if (1..=12).contains(&month) && (1..=31).contains(&day) {
        Ok(())
    } else {
        Err("--date must be a valid YYYY-MM-DD date".into())
    }
}

fn build_pulse() -> Pulse {
    let mut assets = Vec::new();
    let mut failures = 0;
    for (symbol, label, unit) in SYMBOLS {
        match fetch_asset(symbol, label, unit) {
            Ok(asset) => assets.push(asset),
            Err(_) => {
                failures += 1;
                assets.push(Asset {
                    symbol,
                    label,
                    unit,
                    value: None,
                    change: None,
                    note: Some("live data unavailable".into()),
                });
            }
        }
    }
    let mut notes = vec!["market quotes from Yahoo Finance chart endpoint via curl".to_string()];
    if failures > 0 {
        notes.push(format!(
            "{failures} quote(s) unavailable; kept the learning loop alive"
        ));
    }
    let avg_equity = avg_change(&assets, &["^GSPC", "^IXIC", "^KS11"]).unwrap_or(0.0);
    let mood = infer_mood(
        avg_equity,
        change_for(&assets, "DX-Y.NYB"),
        change_for(&assets, "^TNX"),
        change_for(&assets, "BTC-USD"),
    );
    let drivers = infer_drivers(&assets);
    let tensions = infer_tensions(&assets);
    let question = infer_question(&tensions, &mood);
    let concept = infer_concept(&tensions, &drivers);
    Pulse {
        timestamp: timestamp(),
        session: session(),
        basis: PULSE_QUOTE_BASIS.iter().map(|s| (*s).to_string()).collect(),
        mood,
        assets,
        drivers,
        tensions,
        question,
        concept,
        notes,
    }
}

fn build_regime() -> Regime {
    let mut assets = Vec::new();
    let mut failures = 0;
    for (symbol, label, unit) in SYMBOLS {
        match fetch_asset_window(symbol, label, unit, "3mo", "1wk", WindowChange::FirstClose) {
            Ok(asset) => assets.push(asset),
            Err(_) => {
                failures += 1;
                assets.push(Asset {
                    symbol,
                    label,
                    unit,
                    value: None,
                    change: None,
                    note: Some("regime data unavailable".into()),
                });
            }
        }
    }
    let mut notes = vec![
        "market regime uses Yahoo Finance chart endpoint via curl".to_string(),
        "interpret as learning scaffold, not investment advice or a trading signal".to_string(),
    ];
    if failures > 0 {
        notes.push(format!(
            "{failures} quote(s) unavailable; regime read is partial"
        ));
    }
    let avg_equity = avg_change(&assets, &["^GSPC", "^IXIC", "^KS11"]).unwrap_or(0.0);
    let label = infer_regime_label(
        avg_equity,
        change_for(&assets, "DX-Y.NYB"),
        change_for(&assets, "^TNX"),
        change_for(&assets, "CL=F"),
        change_for(&assets, "BTC-USD"),
    );
    let drivers = infer_regime_drivers(&assets);
    let tensions = infer_regime_tensions(&assets, &label);
    let checks = infer_regime_checks(&assets, &label);
    let question = infer_regime_question(&label, &tensions);
    Regime {
        timestamp: timestamp(),
        basis: REGIME_QUOTE_BASIS
            .iter()
            .map(|s| (*s).to_string())
            .collect(),
        label,
        assets,
        drivers,
        tensions,
        checks,
        question,
        notes,
    }
}

#[derive(Clone, Copy, Debug)]
enum WindowChange {
    PreviousClose,
    FirstClose,
}

fn fetch_asset(
    symbol: &'static str,
    label: &'static str,
    unit: &'static str,
) -> Result<Asset, String> {
    fetch_asset_window(symbol, label, unit, "5d", "1d", WindowChange::PreviousClose)
}

fn fetch_asset_window(
    symbol: &'static str,
    label: &'static str,
    unit: &'static str,
    range: &str,
    interval: &str,
    change_from: WindowChange,
) -> Result<Asset, String> {
    let url = format!(
        "https://query1.finance.yahoo.com/v8/finance/chart/{}?range={range}&interval={interval}",
        encode_symbol(symbol),
    );
    let output = Command::new("curl")
        .args([
            "-fsSL",
            "--max-time",
            "4",
            "-A",
            "Mozilla/5.0 market-pulse/0.1",
            &url,
        ])
        .stderr(Stdio::null())
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err("curl failed".into());
    }
    let body = String::from_utf8_lossy(&output.stdout);
    let closes = close_values(&body);
    let value = number_after(&body, "\"regularMarketPrice\":").or_else(|| closes.last().copied());
    let previous = match change_from {
        WindowChange::PreviousClose => number_after(&body, "\"chartPreviousClose\":")
            .or_else(|| closes.get(closes.len().saturating_sub(2)).copied()),
        WindowChange::FirstClose => closes.first().copied(),
    };
    let change = match (value, previous) {
        (Some(v), Some(p)) if p != 0.0 => Some(((v - p) / p) * 100.0),
        _ => None,
    };
    Ok(Asset {
        symbol,
        label,
        unit,
        value,
        change,
        note: None,
    })
}

fn encode_symbol(symbol: &str) -> String {
    symbol
        .replace('^', "%5E")
        .replace('=', "%3D")
        .replace('/', "%2F")
}

fn number_after(text: &str, needle: &str) -> Option<f64> {
    let start = text.find(needle)? + needle.len();
    let rest = &text[start..];
    let end = rest
        .find(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-'))
        .unwrap_or(rest.len());
    rest[..end].parse().ok()
}

fn close_values(text: &str) -> Vec<f64> {
    let Some(start_key) = text.find("\"close\":[") else {
        return vec![];
    };
    let start = start_key + "\"close\":[".len();
    let Some(end) = text[start..].find(']') else {
        return vec![];
    };
    text[start..start + end]
        .split(',')
        .filter_map(|v| v.parse().ok())
        .collect()
}

fn avg_change(assets: &[Asset], symbols: &[&str]) -> Option<f64> {
    let vals = symbols
        .iter()
        .filter_map(|s| change_for(assets, s))
        .collect::<Vec<_>>();
    if vals.is_empty() {
        None
    } else {
        Some(vals.iter().sum::<f64>() / vals.len() as f64)
    }
}

fn change_for(assets: &[Asset], symbol: &str) -> Option<f64> {
    assets
        .iter()
        .find(|a| a.symbol == symbol)
        .and_then(|a| a.change)
}

fn infer_mood(avg_equity: f64, usd: Option<f64>, rates: Option<f64>, btc: Option<f64>) -> String {
    let mut pressure = 0;
    if avg_equity < -0.35 {
        pressure -= 1;
    } else if avg_equity > 0.35 {
        pressure += 1;
    }
    if usd.is_some_and(|v| v > 0.25) {
        pressure -= 1;
    }
    if rates.is_some_and(|v| v > 0.5) {
        pressure -= 1;
    }
    if btc.is_some_and(|v| v > 1.0) {
        pressure += 1;
    }
    match pressure {
        2.. => "risk-on / growth-friendly",
        ..=-2 => "risk-off / macro pressure",
        _ => "mixed / needs confirmation",
    }
    .into()
}

fn infer_drivers(assets: &[Asset]) -> Vec<String> {
    let mut drivers = Vec::new();
    for a in assets {
        let Some(c) = a.change else {
            continue;
        };
        match a.symbol {
            "^TNX" if c.abs() > 0.5 => drivers.push(format!(
                "US 10Y yield is {}, so rate pressure matters",
                if c > 0.0 { "rising" } else { "falling" }
            )),
            "KRW=X" | "DX-Y.NYB" if c.abs() > 0.25 => drivers.push(format!(
                "Dollar/FX is {}, watch cross-market pressure",
                if c > 0.0 { "stronger" } else { "softer" }
            )),
            "^GSPC" | "^IXIC" | "^KS11" if c.abs() > 0.6 => drivers.push(format!(
                "{} is {}, check whether this is broad or sector-led",
                a.label,
                if c > 0.0 { "higher" } else { "lower" }
            )),
            "CL=F" if c.abs() > 1.0 => drivers.push(format!(
                "Oil is {}, inflation and margin narratives may matter",
                if c > 0.0 { "higher" } else { "lower" }
            )),
            _ => {}
        }
    }
    if drivers.is_empty() {
        drivers.push("No single asset is dominating; compare cross-asset confirmation".into());
        drivers.push(
            "Use the next note to separate market-wide signal from sector-specific noise".into(),
        );
    }
    drivers.truncate(4);
    drivers
}

fn infer_tensions(assets: &[Asset]) -> Vec<String> {
    let mut t = Vec::new();
    if change_for(assets, "^TNX").is_some() && change_for(assets, "^IXIC").is_some() {
        t.push("rates pressure vs growth/tech resilience".into());
    }
    if (change_for(assets, "DX-Y.NYB").is_some() || change_for(assets, "KRW=X").is_some())
        && change_for(assets, "^KS11").is_some()
    {
        t.push("USD strength vs Korea/EM risk appetite".into());
    }
    if change_for(assets, "CL=F").is_some() {
        t.push("oil/inflation pressure vs earnings optimism".into());
    }
    if t.is_empty() {
        t.push("macro signal vs sector-specific leadership".into());
    }
    t.truncate(3);
    t
}

fn infer_question(tensions: &[String], mood: &str) -> String {
    let text = tensions.join(" ");
    if text.contains("rates") {
        "Is the market trading rate pressure or earnings/growth hope?"
    } else if text.contains("USD") {
        "Is FX pressure driving risk appetite, or is it just background noise?"
    } else if mood.contains("risk-off") {
        "Which asset confirms the risk-off signal, and which asset disagrees?"
    } else {
        "What is the strongest cross-asset confirmation, and what is the main contradiction?"
    }
    .into()
}

fn infer_concept(tensions: &[String], drivers: &[String]) -> String {
    let text = format!("{} {}", tensions.join(" "), drivers.join(" ")).to_lowercase();
    if text.contains("rates") || text.contains("yield") {
        "rates vs growth"
    } else if text.contains("dollar") || text.contains("fx") || text.contains("usd") {
        "dollar liquidity"
    } else if text.contains("oil") || text.contains("inflation") {
        "inflation impulse"
    } else {
        "risk-on / risk-off"
    }
    .into()
}

fn infer_regime_label(
    avg_equity: f64,
    usd: Option<f64>,
    rates: Option<f64>,
    oil: Option<f64>,
    btc: Option<f64>,
) -> String {
    let macro_pressure = usd.is_some_and(|v| v > 1.0)
        || rates.is_some_and(|v| v > 3.0)
        || oil.is_some_and(|v| v > 8.0);
    let high_beta = btc.is_some_and(|v| v > 10.0);
    if avg_equity > 5.0 && !macro_pressure {
        "risk-on / growth-led regime"
    } else if avg_equity > 2.0 && macro_pressure {
        "equity resilience under macro pressure"
    } else if avg_equity < -3.0 && macro_pressure {
        "macro-pressure / de-risking regime"
    } else if avg_equity < -3.0 {
        "risk-off / earnings-growth doubt"
    } else if high_beta && avg_equity >= 0.0 {
        "liquidity-sensitive high-beta regime"
    } else {
        "mixed / transition regime"
    }
    .into()
}

fn infer_regime_drivers(assets: &[Asset]) -> Vec<String> {
    let mut drivers = Vec::new();
    if change_for(assets, "^IXIC").is_some_and(|v| v > 5.0) {
        drivers.push("Nasdaq strength suggests growth/AI leadership is part of the regime".into());
    }
    if avg_change(assets, &["^GSPC", "^IXIC", "^KS11"]).is_some_and(|v| v > 4.0) {
        drivers.push("Equity indexes are broadly higher over the regime window".into());
    }
    if change_for(assets, "DX-Y.NYB").is_some_and(|v| v > 1.0)
        || change_for(assets, "KRW=X").is_some_and(|v| v > 1.0)
    {
        drivers.push("Dollar/FX strength is a macro pressure channel to keep checking".into());
    }
    if change_for(assets, "^TNX").is_some_and(|v| v > 3.0) {
        drivers.push(
            "US 10Y yields are higher over the window, so discount-rate pressure matters".into(),
        );
    }
    if change_for(assets, "CL=F").is_some_and(|v| v > 8.0) {
        drivers.push("Oil is higher enough to keep inflation and margin narratives alive".into());
    }
    if change_for(assets, "GC=F").is_some_and(|v| v > 5.0) {
        drivers.push(
            "Gold strength points to hedge demand, liquidity concern, or real-rate debate".into(),
        );
    }
    if change_for(assets, "BTC-USD").is_some_and(|v| v > 10.0) {
        drivers.push("BTC strength suggests high-beta liquidity appetite is active".into());
    }
    if drivers.is_empty() {
        drivers
            .push("No single 1-3 month driver dominates; treat the regime as transitionary".into());
        drivers
            .push("Compare equities, yields, dollar, and oil before trusting one headline".into());
    }
    drivers.truncate(5);
    drivers
}

fn infer_regime_tensions(assets: &[Asset], label: &str) -> Vec<String> {
    let mut tensions = Vec::new();
    let avg_equity = avg_change(assets, &["^GSPC", "^IXIC", "^KS11"]).unwrap_or(0.0);
    if avg_equity > 2.0
        && (change_for(assets, "^TNX").is_some_and(|v| v > 3.0)
            || change_for(assets, "DX-Y.NYB").is_some_and(|v| v > 1.0))
    {
        tensions.push("equity strength vs tighter financial-condition signals".into());
    }
    if change_for(assets, "^IXIC").unwrap_or(0.0) - change_for(assets, "^KS11").unwrap_or(0.0) > 5.0
    {
        tensions.push("US growth leadership vs Korea/EM follow-through".into());
    }
    if change_for(assets, "CL=F").is_some_and(|v| v > 8.0) && avg_equity > 0.0 {
        tensions.push("risk appetite vs oil/inflation impulse".into());
    }
    if change_for(assets, "GC=F").is_some_and(|v| v > 5.0) && avg_equity > 0.0 {
        tensions.push("risk-on equities vs defensive/hedge demand in gold".into());
    }
    if label.contains("transition") {
        tensions.push("short-term pulse may disagree with the 1-3 month backdrop".into());
    }
    if tensions.is_empty() {
        tensions.push("headline trend vs cross-asset confirmation".into());
    }
    tensions.truncate(4);
    tensions
}

fn infer_regime_checks(assets: &[Asset], label: &str) -> Vec<String> {
    let mut checks = Vec::new();
    if label.contains("resilience") || label.contains("pressure") {
        checks.push(
            "Check whether yields and dollar are rising for the same reason or different reasons"
                .into(),
        );
    }
    if change_for(assets, "^IXIC").is_some() && change_for(assets, "^GSPC").is_some() {
        checks.push(
            "Compare Nasdaq vs S&P 500 to separate growth leadership from broad risk appetite"
                .into(),
        );
    }
    if change_for(assets, "^KS11").is_some() {
        checks.push(
            "Check whether Korea/KOSPI confirms the US story or lags because of FX/EM pressure"
                .into(),
        );
    }
    if change_for(assets, "CL=F").is_some() {
        checks.push(
            "Watch oil: if it keeps rising, inflation narratives can change the regime label"
                .into(),
        );
    }
    checks.push("Ask what evidence would force you to rename this regime next week".into());
    checks.truncate(5);
    checks
}

fn infer_regime_question(label: &str, tensions: &[String]) -> String {
    let text = format!("{label} {}", tensions.join(" ")).to_lowercase();
    if text.contains("macro pressure") || text.contains("financial-condition") {
        "Is equity strength absorbing macro pressure, or has the market not priced it yet?"
    } else if text.contains("growth") || text.contains("nasdaq") {
        "Is this regime broad risk-on, or mostly growth/AI leadership?"
    } else if text.contains("transition") {
        "What single cross-asset signal would prove the regime is changing?"
    } else {
        "Which asset best confirms the 1-3 month regime, and which asset disagrees?"
    }
    .into()
}

fn detect_tags(text: &str) -> Vec<&'static str> {
    let lower = text.to_lowercase();
    let mut tags = Vec::new();
    let defs: &[(&str, &[&str])] = &[
        ("rates", &["금리", "yield", "rate", "yields"]),
        (
            "semis",
            &[
                "반도체",
                "semiconductor",
                "semis",
                "ai",
                "엔비디아",
                "nvidia",
            ],
        ),
        ("fx", &["달러", "환율", "원화", "usd", "dollar", "krw"]),
        ("oil", &["유가", "oil", "wti", "원유"]),
        ("korea", &["한국", "코스피", "kospi", "korea"]),
        ("crypto", &["코인", "비트", "btc", "crypto"]),
        (
            "event",
            &[
                "ipo",
                "상장",
                "공모",
                "listing",
                "earnings",
                "실적",
                "이벤트",
            ],
        ),
        (
            "positioning",
            &[
                "포지션",
                "수급",
                "positioning",
                "숏커버",
                "short",
                "리밸런싱",
                "옵션",
                "만기",
            ],
        ),
    ];
    for (tag, needles) in defs {
        if needles.iter().any(|n| lower.contains(&n.to_lowercase())) {
            tags.push(*tag);
        }
    }
    tags.sort_unstable();
    tags.dedup();
    tags
}

fn detect_thesis_type(tags: &[&str]) -> String {
    if tags.contains(&"rates") && tags.contains(&"semis") {
        "rates/growth tension thesis"
    } else if tags.contains(&"event") {
        "event-driven / supply-calendar thesis"
    } else if tags.contains(&"positioning") {
        "positioning / flow thesis"
    } else if tags.contains(&"rates") {
        "rates / policy-expectation thesis"
    } else if tags.contains(&"fx") {
        "dollar-liquidity transmission thesis"
    } else if tags.contains(&"oil") {
        "oil / inflation impulse thesis"
    } else if tags.contains(&"semis") {
        "sector leadership thesis"
    } else if tags.contains(&"korea") {
        "Korea/EM risk-transmission thesis"
    } else if tags.contains(&"crypto") {
        "high-beta risk appetite thesis"
    } else {
        "broad market narrative thesis"
    }
    .into()
}

fn make_inquiry(question: &str, linked: Option<String>) -> Inquiry {
    let tags = detect_tags(question);
    let thesis_type = detect_thesis_type(&tags);
    Inquiry {
        timestamp: timestamp(),
        question: question.to_string(),
        linked,
        breakdown: question_breakdown(question, &tags, &thesis_type),
        explanations: possible_explanations(&tags),
        evidence: evidence_checks(&tags),
        counter: counters(&tags),
        next_question: next_better_question(&tags),
        concepts: concepts(&tags),
        thesis_type,
    }
}

fn question_breakdown(question: &str, tags: &[&str], thesis_type: &str) -> Vec<String> {
    let mut v = vec![
        format!(
            "Core question: “{}”",
            question.split_whitespace().collect::<Vec<_>>().join(" ")
        ),
        format!("Main lens: {thesis_type}."),
    ];
    if tags.is_empty() {
        v.push("No strong topic tag was detected, so start by separating price action, timing, and causality.".into());
    } else {
        v.push(format!("Detected market grammar: {}.", tags.join(", ")));
    }
    v.push(
        "Do not decide from one headline; ask what would confirm and what would falsify the story."
            .into(),
    );
    v
}

fn possible_explanations(tags: &[&str]) -> Vec<String> {
    let mut v = Vec::new();
    if tags.contains(&"rates") {
        v.push("Rate-cut or policy-easing expectations could be changing discount-rate pressure on growth assets.".into());
    }
    if tags.contains(&"event") {
        v.push("A large IPO/listing or earnings event can shift attention, supply, and positioning, but it rarely explains the whole market alone.".into());
    }
    if tags.contains(&"positioning") {
        v.push("Positioning, short-covering, option hedging, or rebalancing may create a move that looks like a macro signal.".into());
    }
    if tags.contains(&"semis") {
        v.push("Semiconductor or AI leadership may be a sector-specific growth story rather than broad risk appetite.".into());
    }
    if tags.contains(&"fx") {
        v.push(
            "Dollar strength or KRW weakness can transmit liquidity pressure into Korea/EM assets."
                .into(),
        );
    }
    if tags.contains(&"oil") {
        v.push(
            "Oil can matter through inflation expectations, margins, and sector rotation.".into(),
        );
    }
    if tags.contains(&"crypto") {
        v.push("Crypto strength can be a high-beta liquidity signal, but it may also be its own positioning cycle.".into());
    }
    v.push("Earnings revisions, liquidity, or positioning may be dominating even when the headline story sounds macro-driven.".into());
    v.push("The move may simply be noise unless multiple assets confirm the same story in the right sequence.".into());
    v.truncate(4);
    v
}

fn evidence_checks(tags: &[&str]) -> Vec<String> {
    let mut v = checks(tags);
    if tags.contains(&"event") {
        v.push("Check whether the IPO/listing/event timing actually came before the market move, not after the narrative formed.".into());
        v.push("Check whether the effect is broad index-level pressure or concentrated around related names and liquidity pockets.".into());
    }
    if tags.contains(&"positioning") {
        v.push("Look for signs of flow/positioning pressure: gap moves, squeeze-like reversals, option expiry, or narrow leadership.".into());
    }
    v.push("State the one observation that would make this explanation wrong.".into());
    v
}

fn make_feedback(text: &str, linked: Option<String>) -> Feedback {
    let tags = detect_tags(text);
    let thesis_type = detect_thesis_type(&tags);
    let clean = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let claim = if tags.is_empty() {
        format!("You are making a market interpretation that needs evidence: “{clean}”")
    } else {
        format!(
            "You are linking {} to a market interpretation: “{clean}”",
            tags.join(", ")
        )
    };
    Feedback {
        timestamp: timestamp(),
        thought: text.to_string(),
        linked,
        claim,
        thesis_type,
        good: good(&tags),
        check: evidence_checks(&tags),
        counter: counters(&tags),
        next: next_questions(&tags),
        concepts: concepts(&tags),
    }
}

fn good(tags: &[&str]) -> Vec<String> {
    let mut v =
        vec!["You wrote an explicit interpretation instead of only consuming market noise.".into()];
    if tags.contains(&"rates") && tags.contains(&"semis") {
        v.push("You separated macro pressure from sector/growth resilience, which is a useful market lens.".into());
    }
    if tags.contains(&"fx") && tags.contains(&"korea") {
        v.push("You connected FX pressure with Korea/EM risk, which is an important cross-market habit.".into());
    }
    if tags.len() >= 2 {
        v.push("You are already comparing more than one driver instead of forcing a single-cause story.".into());
    }
    if tags.contains(&"event") {
        v.push("You noticed an event-driven explanation instead of accepting the first macro narrative.".into());
    }
    v
}

fn checks(tags: &[&str]) -> Vec<String> {
    let mut v = vec!["Name the observable data that would confirm or reject this view.".into()];
    if tags.contains(&"semis") {
        v.push("Check whether semiconductor strength is broad or concentrated in a few mega-cap names.".into());
    }
    if tags.contains(&"rates") {
        v.push(
            "Check whether yields moved before or after the equity reaction; timing matters."
                .into(),
        );
    }
    if tags.contains(&"fx") {
        v.push(
            "Check whether USD strength is broad DXY strength or mostly a KRW/local move.".into(),
        );
    }
    if tags.contains(&"korea") {
        v.push(
            "Separate KOSPI index direction from sector leadership and foreign flow if available."
                .into(),
        );
    }
    if tags.contains(&"positioning") {
        v.push("Check whether the move looks like flow/positioning pressure rather than a durable fundamental change.".into());
    }
    if v.len() == 1 {
        v.push(
            "Avoid broad claims until at least two assets or events point in the same direction."
                .into(),
        );
    }
    v
}

fn counters(tags: &[&str]) -> Vec<String> {
    let mut v = Vec::new();
    if tags.contains(&"semis") {
        v.push("Semis strength may be positioning, earnings expectations, or mega-cap concentration rather than broad growth optimism.".into());
    }
    if tags.contains(&"rates") {
        v.push("Rate pressure may be background noise if earnings revisions or liquidity are dominating the session.".into());
    }
    if tags.contains(&"fx") {
        v.push("FX pressure can matter less when local sector leadership or global risk appetite is strong enough.".into());
    }
    if tags.contains(&"event") {
        v.push("An IPO/listing or earnings calendar can explain local supply/attention, but not necessarily the whole index or macro regime.".into());
    }
    if tags.contains(&"positioning") {
        v.push("A positioning-driven move can fade quickly once the flow is absorbed, even if the headline narrative sounds persuasive.".into());
    }
    if v.is_empty() {
        v.push("The same price action may come from positioning, liquidity, news timing, or sector rotation; keep at least one alternative open.".into());
    }
    v
}

fn next_questions(tags: &[&str]) -> Vec<String> {
    let mut v = Vec::new();
    if tags.contains(&"rates") && tags.contains(&"semis") {
        v.push("If yields rise further, do semis still outperform the broad market?".into());
    } else if tags.contains(&"rates") {
        v.push("If this is really easing expectations, should growth assets, the dollar, and yields confirm together?".into());
    }
    if tags.contains(&"fx") && tags.contains(&"korea") {
        v.push("Does KRW weakness coincide with foreign selling, or are exporters offsetting the pressure?".into());
    }
    if tags.contains(&"oil") {
        v.push("Is oil moving enough to change inflation expectations, or is it only a sector input today?".into());
    }
    if tags.contains(&"event") {
        v.push("What market segment should move first if the IPO/event explanation is actually driving the session?".into());
    }
    if tags.contains(&"positioning") {
        v.push("What would distinguish positioning from a real change in growth, rates, or earnings expectations?".into());
    }
    if v.is_empty() {
        v.push(
            "What would you need to see by the close to say this interpretation was wrong?".into(),
        );
    }
    v
}

fn next_better_question(tags: &[&str]) -> String {
    next_questions(tags).into_iter().next().unwrap_or_else(|| {
        "What evidence would make this market interpretation wrong by the close?".into()
    })
}

fn concepts(tags: &[&str]) -> Vec<String> {
    let mut v = Vec::new();
    for tag in tags {
        v.push(
            match *tag {
                "rates" => "rates vs growth",
                "semis" => "sector leadership",
                "fx" => "dollar liquidity",
                "oil" => "inflation impulse",
                "korea" => "EM/Korea risk transmission",
                "crypto" => "high-beta risk appetite",
                "event" => "event-driven supply/attention",
                "positioning" => "positioning and flow",
                _ => continue,
            }
            .to_string(),
        );
    }
    if v.is_empty() {
        v.push("risk-on / risk-off".into());
    }
    v
}

fn render_pulse(p: &Pulse, compact: bool) -> String {
    let seeds = question_seeds_for(p);
    if compact {
        return format!(
            "[mp] {} · {} · Puzzle: {}",
            p.mood,
            p.drivers
                .iter()
                .take(2)
                .cloned()
                .collect::<Vec<_>>()
                .join("; "),
            seeds.first().unwrap_or(&p.question)
        );
    }
    let mut out = format!(
        "Market Pulse · {} · {}\n\nMood\n  {}\n\nBasis\n",
        p.timestamp, p.session, p.mood
    );
    for b in &p.basis {
        out.push_str(&format!("  - {b}\n"));
    }
    out.push_str("\nAssets\n");
    for a in &p.assets {
        let value = a
            .value
            .map(|v| {
                format!(
                    "{v:.2}{}",
                    if a.unit.is_empty() {
                        "".to_string()
                    } else {
                        format!(" {}", a.unit)
                    }
                )
            })
            .unwrap_or_else(|| "n/a".into());
        let change = a
            .change
            .map(|c| format!("{:+.2}%", c))
            .unwrap_or_else(|| "n/a".into());
        let note = a
            .note
            .as_ref()
            .map(|n| format!(" · {n}"))
            .unwrap_or_default();
        out.push_str(&format!(
            "  - {}: {} ({}){}\n",
            a.label, value, change, note
        ));
    }
    out.push_str("\nDrivers\n");
    for (i, d) in p.drivers.iter().enumerate() {
        out.push_str(&format!("  {}. {}\n", i + 1, d));
    }
    out.push_str("\nTensions\n");
    for t in &p.tensions {
        out.push_str(&format!("  - {t}\n"));
    }
    out.push_str(&format!(
        "\nQuestion\n  {}\n\nConcept\n  {}\n",
        p.question, p.concept
    ));
    out.push_str("\nMarket puzzle / question seeds\n");
    for seed in seeds {
        out.push_str(&format!("  - {seed}\n"));
    }
    if !p.notes.is_empty() {
        out.push_str("\nSource notes\n");
        for n in &p.notes {
            out.push_str(&format!("  - {n}\n"));
        }
    }
    out
}

fn render_regime(r: &Regime) -> String {
    let mut out = format!(
        "Market Regime · {}\n\nRegime\n  {}\n\nBasis\n",
        r.timestamp, r.label
    );
    for b in &r.basis {
        out.push_str(&format!("  - {b}\n"));
    }
    out.push_str("\n1-3M Asset Map\n");
    for a in &r.assets {
        let value = a
            .value
            .map(|v| {
                format!(
                    "{v:.2}{}",
                    if a.unit.is_empty() {
                        "".to_string()
                    } else {
                        format!(" {}", a.unit)
                    }
                )
            })
            .unwrap_or_else(|| "n/a".into());
        let change = a
            .change
            .map(|c| format!("{:+.2}%", c))
            .unwrap_or_else(|| "n/a".into());
        let note = a
            .note
            .as_ref()
            .map(|n| format!(" · {n}"))
            .unwrap_or_default();
        out.push_str(&format!(
            "  - {}: {} ({}){}\n",
            a.label, value, change, note
        ));
    }
    out.push_str("\nRegime drivers\n");
    for (i, d) in r.drivers.iter().enumerate() {
        out.push_str(&format!("  {}. {}\n", i + 1, d));
    }
    out.push_str("\nRegime tensions\n");
    for t in &r.tensions {
        out.push_str(&format!("  - {t}\n"));
    }
    out.push_str("\nChecks for next session/week\n");
    for c in &r.checks {
        out.push_str(&format!("  - {c}\n"));
    }
    out.push_str(&format!(
        "\nNext better regime question\n  {}\n\nBoundary\n  Market literacy only; not investment advice, buy/sell guidance, price targets, stop-loss, or portfolio instructions.\n",
        r.question
    ));
    if !r.notes.is_empty() {
        out.push_str("\nSource notes\n");
        for n in &r.notes {
            out.push_str(&format!("  - {n}\n"));
        }
    }
    out
}

fn question_seeds_for(p: &Pulse) -> Vec<String> {
    let text = format!(
        "{} {} {}",
        p.mood,
        p.tensions.join(" "),
        p.drivers.join(" ")
    )
    .to_lowercase();
    let mut seeds = vec![p.question.clone()];
    if text.contains("rates") || text.contains("yield") {
        seeds.push(
            "If yields move again, does growth leadership strengthen, fade, or ignore it?".into(),
        );
    }
    if text.contains("usd") || text.contains("dollar") || text.contains("fx") {
        seeds.push(
            "Is dollar/FX pressure leading risk appetite, or only confirming a local move?".into(),
        );
    }
    if text.contains("oil") || text.contains("inflation") {
        seeds.push(
            "Is oil large enough to change inflation expectations, or just sector noise?".into(),
        );
    }
    if text.contains("mixed") {
        seeds.push("Which asset is disagreeing with the headline story, and why?".into());
    }
    let mut unique = Vec::new();
    for seed in seeds {
        if !unique.contains(&seed) {
            unique.push(seed);
        }
    }
    unique.truncate(3);
    unique
}

fn render_inquiry(i: &Inquiry) -> String {
    let mut out = format!("Market Inquiry · {}\n\nQuestion breakdown\n", i.timestamp);
    for x in &i.breakdown {
        out.push_str(&format!("  - {x}\n"));
    }
    out.push_str("\nPossible explanations\n");
    for (idx, x) in i.explanations.iter().enumerate() {
        out.push_str(&format!("  {}. {}\n", idx + 1, x));
    }
    out.push_str("\nEvidence to check\n");
    for x in &i.evidence {
        out.push_str(&format!("  - {x}\n"));
    }
    out.push_str("\nCounter-view\n");
    for x in &i.counter {
        out.push_str(&format!("  - {x}\n"));
    }
    out.push_str(&format!(
        "\nNext better question\n  {}\n\nConcepts\n  {}\n\nBoundary\n  Market literacy only; not investment advice, buy/sell guidance, price targets, stop-loss, or portfolio instructions.",
        i.next_question,
        i.concepts.join(", ")
    ));
    out
}

fn render_research_inquiry(i: &Inquiry, bundle: &ResearchBundle) -> String {
    let mut out = format!(
        "Research-backed Inquiry · {} · provider: {}\n\nQuestion breakdown\n",
        i.timestamp, bundle.provider
    );
    for x in &i.breakdown {
        out.push_str(&format!("  - {x}\n"));
    }
    out.push_str("\nSources checked\n");
    if bundle.sources.is_empty() {
        out.push_str("  - No configured research source returned metadata.\n");
        out.push_str(
            "  - Treat the analysis below as inference scaffolding, not source-backed fact.\n",
        );
    } else {
        for (idx, source) in bundle.sources.iter().enumerate() {
            let published = source
                .published_at
                .as_deref()
                .unwrap_or("published time unavailable");
            out.push_str(&format!(
                "  {}. {} — {} — {}\n",
                idx + 1,
                source.title,
                source.publisher,
                published
            ));
            out.push_str(&format!("     URL: {}\n", source.url));
            out.push_str(&format!("     Relevance: {}\n", source.relevance));
            out.push_str(&format!("     Evidence: {}\n", source.evidence));
        }
    }
    if !bundle.notes.is_empty() {
        out.push_str("\nResearch notes\n");
        for note in &bundle.notes {
            out.push_str(&format!("  - {note}\n"));
        }
    }
    out.push_str("\nWhat the sources suggest\n");
    if bundle.sources.is_empty() {
        out.push_str("  - No source-backed claim yet; use the inquiry lens below to decide what evidence to fetch next.\n");
    } else {
        for source in &bundle.sources {
            out.push_str(&format!("  - Source-backed: {}\n", source.evidence));
        }
    }
    out.push_str("\nEvidence for your thesis\n");
    if bundle.sources.is_empty() {
        out.push_str(
            "  - Not source-backed yet: first connect the question to observable market data.\n",
        );
    } else {
        for source in bundle.sources.iter().take(3) {
            out.push_str(&format!("  - {}: {}\n", source.publisher, source.evidence));
        }
    }
    out.push_str("\nEvidence against / counter-view\n");
    for x in &i.counter {
        out.push_str(&format!("  - {x}\n"));
    }
    out.push_str("\nData to check next\n");
    for x in &i.evidence {
        out.push_str(&format!("  - {x}\n"));
    }
    out.push_str(&format!(
        "\nNext better question\n  {}\n\nConcepts\n  {}\n\nBoundary\n  Market literacy only; not investment advice, buy/sell guidance, price targets, stop-loss, or portfolio instructions.",
        i.next_question,
        i.concepts.join(", ")
    ));
    out
}

fn render_feedback(f: &Feedback) -> String {
    let mut out = format!(
        "Feedback · {}\n\nClaim\n  {}\n\nThesis type\n  {}\n\nGood\n",
        f.timestamp, f.claim, f.thesis_type
    );
    for x in &f.good {
        out.push_str(&format!("  - {x}\n"));
    }
    out.push_str("\nCheck\n");
    for x in &f.check {
        out.push_str(&format!("  - {x}\n"));
    }
    out.push_str("\nCounter-view\n");
    for x in &f.counter {
        out.push_str(&format!("  - {x}\n"));
    }
    out.push_str("\nNext questions\n");
    for x in &f.next {
        out.push_str(&format!("  - {x}\n"));
    }
    out.push_str(&format!("\nConcepts\n  {}", f.concepts.join(", ")));
    out
}

fn journal_path() -> PathBuf {
    if let Ok(home) = env::var("MARKET_PULSE_HOME") {
        return PathBuf::from(home).join("journal.jsonl");
    }
    let home = env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".local/share/market-pulse/journal.jsonl")
}

fn append_event(line: &str) -> Result<(), String> {
    let path = journal_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    writeln!(file, "{line}").map_err(|e| e.to_string())
}

fn read_events(limit: usize) -> Vec<String> {
    let Ok(text) = fs::read_to_string(journal_path()) else {
        return Vec::new();
    };
    limit_events(read_event_lines(&text), limit)
}

fn read_event_lines(text: &str) -> Vec<String> {
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(str::to_string)
        .collect()
}

fn limit_events(mut lines: Vec<String>, limit: usize) -> Vec<String> {
    if lines.len() > limit {
        lines = lines.split_off(lines.len() - limit);
    }
    lines
}

fn read_events_for_date(limit: usize, date: &str) -> Vec<String> {
    let Ok(text) = fs::read_to_string(journal_path()) else {
        return Vec::new();
    };
    filter_events_by_date(read_event_lines(&text), date, limit)
}

fn filter_events_by_date(events: Vec<String>, date: &str, limit: usize) -> Vec<String> {
    let lines = events
        .into_iter()
        .filter(|l| event_matches_date(l, date))
        .collect::<Vec<_>>();
    limit_events(lines, limit)
}

fn event_matches_date(line: &str, date: &str) -> bool {
    json_field(line, "timestamp").is_some_and(|ts| ts.starts_with(date))
}

fn latest_pulse_timestamp() -> Option<String> {
    read_events(usize::MAX)
        .into_iter()
        .rev()
        .find(|l| l.contains("\"type\":\"pulse\""))
        .and_then(|l| json_field(&l, "timestamp"))
}

fn json_field(line: &str, key: &str) -> Option<String> {
    let key_needle = format!("\"{key}\"");
    let key_start = line.find(&key_needle)? + key_needle.len();
    let after_key = &line[key_start..];
    let colon = after_key.find(':')?;
    let after_colon = after_key[colon + 1..].trim_start();
    let value_start = after_colon.strip_prefix('"')?;
    parse_json_string(value_start)
}

fn parse_json_string(text_after_opening_quote: &str) -> Option<String> {
    let mut out = String::new();
    let mut chars = text_after_opening_quote.chars();
    while let Some(ch) = chars.next() {
        match ch {
            '"' => return Some(out),
            '\\' => {
                let escaped = chars.next()?;
                match escaped {
                    '"' => out.push('"'),
                    '\\' => out.push('\\'),
                    '/' => out.push('/'),
                    'b' => out.push('\u{0008}'),
                    'f' => out.push('\u{000c}'),
                    'n' => out.push('\n'),
                    'r' => out.push('\r'),
                    't' => out.push('\t'),
                    'u' => {
                        let mut code = String::new();
                        for _ in 0..4 {
                            code.push(chars.next()?);
                        }
                        let value = u32::from_str_radix(&code, 16).ok()?;
                        out.push(char::from_u32(value)?);
                    }
                    _ => return None,
                }
            }
            _ => out.push(ch),
        }
    }
    None
}

fn render_review(limit: usize) -> String {
    let events = read_events(limit);
    render_review_from_events(&events, &journal_path().display().to_string())
}

fn render_review_for_date(limit: usize, date: &str) -> String {
    let events = read_events_for_date(limit, date);
    render_review_for_date_from_events(&events, &journal_path().display().to_string(), date)
}

fn render_review_for_date_from_events(events: &[String], journal: &str, date: &str) -> String {
    if events.is_empty() {
        return format!(
            "No market-pulse journal entries found for {date}.\n\nJournal: {journal}\nTry `mp review --limit N` to inspect recent entries, or record one with `mp now`, `mp regime`, `mp ask`, or `mp think`."
        );
    }
    let mut out = format!("Review date filter: {date}\n\n");
    out.push_str(&render_review_from_events(events, journal));
    out
}

fn render_review_from_events(events: &[String], journal: &str) -> String {
    if events.is_empty() {
        return "No market-pulse journal entries yet. Start with `mp \"your market question\"`, then `mp think \"...\"`.".into();
    }
    let pulses = events
        .iter()
        .filter(|l| l.contains("\"type\":\"pulse\""))
        .count();
    let thoughts = events
        .iter()
        .filter(|l| l.contains("\"type\":\"thought\""))
        .count();
    let regimes = events
        .iter()
        .filter(|l| l.contains("\"type\":\"regime\""))
        .count();
    let inquiries = events
        .iter()
        .filter(|l| l.contains("\"type\":\"inquiry\""))
        .count();
    let research_inquiries = events
        .iter()
        .filter(|l| l.contains("\"type\":\"research_inquiry\""))
        .count();
    let feedback = events
        .iter()
        .filter(|l| l.contains("\"type\":\"feedback\""))
        .count();
    let mut counts: Vec<(&str, usize)> = vec![
        ("rates", 0),
        ("semis", 0),
        ("fx", 0),
        ("oil", 0),
        ("korea", 0),
        ("crypto", 0),
        ("event", 0),
        ("positioning", 0),
    ];
    let mut thesis_types = Vec::new();
    for line in events.iter().filter(|l| {
        l.contains("\"type\":\"thought\"")
            || l.contains("\"type\":\"inquiry\"")
            || l.contains("\"type\":\"research_inquiry\"")
    }) {
        let text = json_field(line, "text")
            .or_else(|| json_field(line, "question"))
            .unwrap_or_default();
        let tags = detect_tags(&text);
        if !tags.is_empty() {
            thesis_types.push(detect_thesis_type(&tags));
        }
        for tag in tags {
            if let Some((_, count)) = counts.iter_mut().find(|(name, _)| *name == tag) {
                *count += 1;
            }
        }
    }
    counts.sort_by(|a, b| b.1.cmp(&a.1));
    thesis_types.sort();
    thesis_types.dedup();
    let mut out = format!("Market Pulse Review\n\nJournal: {journal}\nEntries scanned: {} · pulses {} · regimes {} · inquiries {} · research {} · thoughts {} · feedback {}\n\nRepeated themes\n", events.len(), pulses, regimes, inquiries, research_inquiries, thoughts, feedback);
    let mut wrote = false;
    for (tag, count) in counts.into_iter().filter(|(_, c)| *c > 0).take(6) {
        wrote = true;
        out.push_str(&format!("  - {tag}: {count}\n"));
    }
    if !wrote {
        out.push_str("  - Not enough tagged thoughts yet\n");
    }
    out.push_str("\nQuestion / thesis habits\n");
    if thesis_types.is_empty() {
        out.push_str("  - Not enough inquiry/thesis history yet; ask one rough question with `mp \"...\"`.\n");
    } else {
        for t in thesis_types.iter().take(5) {
            out.push_str(&format!("  - You have been using a {t} lens.\n"));
        }
    }
    out.push_str("\nSuggested drill\n  For the next 3 notes, explicitly separate:\n  1. market-wide signal\n  2. sector-specific signal\n  3. event/positioning alternative\n  4. what would falsify the view");
    out
}

fn pulse_json(p: &Pulse) -> String {
    format!(
        "{{\"type\":\"pulse\",\"timestamp\":\"{}\",\"session\":\"{}\",\"basis\":\"{}\",\"mood\":\"{}\",\"question\":\"{}\",\"concept\":\"{}\"}}",
        esc(&p.timestamp),
        esc(&p.session),
        esc(&p.basis.join(" | ")),
        esc(&p.mood),
        esc(&p.question),
        esc(&p.concept)
    )
}

fn regime_json(r: &Regime) -> String {
    format!(
        "{{\"type\":\"regime\",\"timestamp\":\"{}\",\"basis\":\"{}\",\"label\":\"{}\",\"question\":\"{}\"}}",
        esc(&r.timestamp),
        esc(&r.basis.join(" | ")),
        esc(&r.label),
        esc(&r.question)
    )
}

fn thought_json(text: &str, linked: Option<&str>) -> String {
    format!("{{\"type\":\"thought\",\"timestamp\":\"{}\",\"text\":\"{}\",\"linked_pulse_timestamp\":{}}}", timestamp(), esc(text), opt_json(linked))
}

fn inquiry_json(i: &Inquiry) -> String {
    format!(
        "{{\"type\":\"inquiry\",\"timestamp\":\"{}\",\"question\":\"{}\",\"linked_pulse_timestamp\":{},\"thesis_type\":\"{}\",\"concepts\":\"{}\"}}",
        esc(&i.timestamp),
        esc(&i.question),
        opt_json(i.linked.as_deref()),
        esc(&i.thesis_type),
        esc(&i.concepts.join(", "))
    )
}

fn research_inquiry_json(i: &Inquiry, bundle: &ResearchBundle) -> String {
    let source_titles = bundle
        .sources
        .iter()
        .map(|s| s.title.as_str())
        .collect::<Vec<_>>()
        .join(" | ");
    format!(
        "{{\"type\":\"research_inquiry\",\"timestamp\":\"{}\",\"question\":\"{}\",\"linked_pulse_timestamp\":{},\"provider\":\"{}\",\"source_count\":{},\"source_titles\":\"{}\",\"thesis_type\":\"{}\",\"concepts\":\"{}\"}}",
        esc(&i.timestamp),
        esc(&i.question),
        opt_json(i.linked.as_deref()),
        esc(&bundle.provider),
        bundle.sources.len(),
        esc(&source_titles),
        esc(&i.thesis_type),
        esc(&i.concepts.join(", "))
    )
}

fn feedback_json(f: &Feedback) -> String {
    format!("{{\"type\":\"feedback\",\"timestamp\":\"{}\",\"thought\":\"{}\",\"linked_pulse_timestamp\":{},\"claim\":\"{}\",\"concepts\":\"{}\"}}", esc(&f.timestamp), esc(&f.thought), opt_json(f.linked.as_deref()), esc(&f.claim), esc(&f.concepts.join(", ")))
}

fn opt_json(value: Option<&str>) -> String {
    value
        .map(|v| format!("\"{}\"", esc(v)))
        .unwrap_or_else(|| "null".into())
}

fn esc(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

fn timestamp() -> String {
    let output = Command::new("date").arg("+%Y-%m-%dT%H:%M:%S%z").output();
    if let Ok(out) = output {
        if let Ok(s) = String::from_utf8(out.stdout) {
            return s.trim().to_string();
        }
    }
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("unix:{secs}")
}

fn session() -> String {
    let hour = Command::new("date")
        .arg("+%H")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.trim().parse::<u32>().ok())
        .unwrap_or(12);
    match hour {
        6..=8 => "Korea morning / US close handoff",
        9..=11 => "Korea open",
        12..=14 => "Asia midday",
        15..=17 => "Korea close",
        18..=21 => "US pre-open",
        _ => "US session / global watch",
    }
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_bare_question_and_ask_to_inquiry() {
        let bare = vec!["금리가".into(), "내려간".into(), "이유?".into()];
        assert_eq!(
            parse_command(&bare).unwrap(),
            CommandKind::Inquiry {
                text: "금리가 내려간 이유?".into(),
                no_save: false,
            }
        );

        let ask = vec![
            "ask".into(),
            "대형".into(),
            "IPO가".into(),
            "영향?".into(),
            "--no-save".into(),
        ];
        assert_eq!(
            parse_command(&ask).unwrap(),
            CommandKind::Inquiry {
                text: "대형 IPO가 영향?".into(),
                no_save: true,
            }
        );
        assert!(parse_command(&["ask".into()]).is_err());
        assert!(parse_command(&["--bad".into()]).is_err());
    }

    #[test]
    fn routes_regime_to_regime_command() {
        assert_eq!(
            parse_command(&["regime".into(), "--no-save".into()]).unwrap(),
            CommandKind::Regime
        );
    }

    #[test]
    fn routes_research_subcommand_and_flag() {
        let research = vec![
            "research".into(),
            "금리".into(),
            "하락이".into(),
            "성장주에".into(),
            "좋음?".into(),
            "--no-save".into(),
        ];
        assert_eq!(
            parse_command(&research).unwrap(),
            CommandKind::Research {
                text: "금리 하락이 성장주에 좋음?".into(),
                no_save: true,
            }
        );

        let flagged = vec![
            "대형".into(),
            "IPO가".into(),
            "영향?".into(),
            "--research".into(),
        ];
        assert_eq!(
            parse_command(&flagged).unwrap(),
            CommandKind::Research {
                text: "대형 IPO가 영향?".into(),
                no_save: false,
            }
        );
        assert!(parse_command(&["research".into()]).is_err());
    }

    #[test]
    fn detects_korean_tags() {
        let tags =
            detect_tags("금리가 부담인데 반도체가 버티고 달러도 강하다. 대형 IPO 상장도 있다");
        assert!(tags.contains(&"rates"));
        assert!(tags.contains(&"semis"));
        assert!(tags.contains(&"fx"));
        assert!(tags.contains(&"event"));
    }

    #[test]
    fn inquiry_renders_required_sections_and_boundary() {
        let inquiry = make_inquiry("금리가 내려갔다는데 이게 완화 기대 때문임?", None);
        let out = render_inquiry(&inquiry);
        for section in [
            "Question breakdown",
            "Possible explanations",
            "Evidence to check",
            "Counter-view",
            "Next better question",
            "Boundary",
            "not investment advice",
        ] {
            assert!(out.contains(section), "missing section: {section}");
        }
        assert!(inquiry.thesis_type.contains("rates"));
    }

    #[test]
    fn research_output_renders_no_provider_fallback_and_boundary() {
        let inquiry = make_inquiry("금리 하락이 성장주에 좋은 신호임?", None);
        let query = ResearchQuery {
            question: inquiry.question.clone(),
            linked: None,
        };
        let bundle = research_bundle_from_provider(&NoopResearchProvider, &query);
        let out = render_research_inquiry(&inquiry, &bundle);
        for section in [
            "Research-backed Inquiry",
            "Sources checked",
            "No configured research source",
            "inference scaffolding",
            "What the sources suggest",
            "Evidence against / counter-view",
            "Data to check next",
            "Boundary",
            "not investment advice",
        ] {
            assert!(out.contains(section), "missing section: {section}");
        }
        assert_eq!(bundle.sources.len(), 0);
    }

    #[test]
    fn research_output_renders_sources_with_metadata() {
        let inquiry = make_inquiry("대형 IPO 때문에 성장주가 강한 걸까?", None);
        let bundle = fixture_research_bundle();
        let out = render_research_inquiry(&inquiry, &bundle);
        assert!(out.contains("Fixture IPO calendar"));
        assert!(out.contains("market-pulse fixture"));
        assert!(out.contains("fixture://ipo-calendar"));
        assert!(out.contains("Relevance: event timing"));
        assert!(out.contains("Source-backed:"));
    }

    #[test]
    fn search_command_template_uses_query_placeholder_without_shell() {
        let args = search_command_args("fixture-search --json {query}", "금리 하락 신호")
            .expect("template should parse");
        assert_eq!(args[0], "fixture-search");
        assert_eq!(args[1], "--json");
        assert_eq!(args[2], "금리 하락 신호");
        assert!(search_command_args("fixture-search --json", "질문").is_err());
    }

    #[test]
    fn search_command_template_supports_quoted_args_without_shell() {
        let args = search_command_args(
            "fixture-search --label \"market source\" --json '{\"evidence\":\"{query}\",\"publisher\":\"unit\"}'",
            "달러 강세",
        )
        .expect("quoted template should parse");
        assert_eq!(
            args,
            vec![
                "fixture-search",
                "--label",
                "market source",
                "--json",
                "{\"evidence\":\"달러 강세\",\"publisher\":\"unit\"}"
            ]
        );
        assert!(search_command_args("fixture-search \"unterminated {query}", "질문").is_err());
    }

    #[test]
    fn search_jsonl_parses_sources_and_caps_rows() {
        let mut lines = Vec::new();
        for idx in 0..25 {
            lines.push(format!(
                "{{\"title\":\"Source {idx}\",\"publisher\":\"fixture\",\"url\":\"fixture://{idx}\",\"evidence\":\"Evidence {idx}\",\"relevance\":\"test\"}}"
            ));
        }
        lines.push("not json".into());
        let (sources, invalid) = parse_search_jsonl(&lines.join("\n"), 20);
        assert_eq!(sources.len(), 20);
        assert_eq!(invalid, 0);
        assert_eq!(sources[0].title, "Source 0");
        assert_eq!(sources[19].url, "fixture://19");
    }

    #[test]
    fn invalid_search_rows_do_not_crash() {
        let (sources, invalid) = parse_search_jsonl(
            "{\"title\":\"Missing evidence\"}\n{\"title\":\"Good\",\"publisher\":\"fixture\",\"url\":\"fixture://ok\",\"evidence\":\"Useful evidence\",\"relevance\":\"test\"}",
            20,
        );
        assert_eq!(sources.len(), 1);
        assert_eq!(invalid, 1);
        assert_eq!(sources[0].evidence, "Useful evidence");
    }

    #[test]
    fn search_jsonl_decodes_escaped_strings() {
        let (sources, invalid) = parse_search_jsonl(
            "{\"title\":\"Quoted \\\"Source\\\"\",\"publisher\":\"fixture\",\"url\":\"fixture://escaped\",\"evidence\":\"line one\\nline two \\\\ backed\",\"relevance\":\"unicode \\u2713\"}",
            20,
        );
        assert_eq!(invalid, 0);
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].title, "Quoted \"Source\"");
        assert_eq!(sources[0].evidence, "line one\nline two \\ backed");
        assert_eq!(sources[0].relevance, "unicode ✓");
    }

    #[test]
    fn search_command_provider_reads_jsonl_sources() {
        let provider = SearchCommandProvider {
            template:
                "/bin/echo {\"title\":\"Fixture\",\"publisher\":\"test\",\"url\":\"fixture://source\",\"evidence\":\"{query}\",\"relevance\":\"unit\"}"
                    .into(),
        };
        let query = ResearchQuery {
            question: "금리 하락 신호".into(),
            linked: None,
        };
        let bundle = research_bundle_from_provider(&provider, &query);
        assert_eq!(bundle.provider, "search-cmd");
        assert_eq!(bundle.sources.len(), 1);
        assert_eq!(bundle.sources[0].evidence, "금리 하락 신호");
        assert!(bundle
            .notes
            .iter()
            .any(|n| n.contains("MARKET_PULSE_SEARCH_CMD")));
    }

    #[test]
    fn search_command_failure_degrades_gracefully() {
        let provider = SearchCommandProvider {
            template: "/definitely/missing-market-pulse-search {query}".into(),
        };
        let query = ResearchQuery {
            question: "달러 강세".into(),
            linked: None,
        };
        let bundle = research_bundle_from_provider(&provider, &query);
        assert_eq!(bundle.provider, "search-cmd");
        assert!(bundle.sources.is_empty());
        assert!(bundle.notes.iter().any(|n| n.contains("failed gracefully")));
    }

    #[test]
    fn research_history_records_metadata() {
        let inquiry = make_inquiry("금리와 반도체가 같이 움직이나?", Some("pulse-ts".into()));
        let bundle = fixture_research_bundle();
        let json = research_inquiry_json(&inquiry, &bundle);
        assert!(json.contains("\"type\":\"research_inquiry\""));
        assert!(json.contains("\"provider\":\"fixture\""));
        assert!(json.contains("\"source_count\":1"));
        assert!(json.contains("Fixture IPO calendar"));
        assert!(json.contains("\"linked_pulse_timestamp\":\"pulse-ts\""));
    }

    #[test]
    fn provider_error_degrades_gracefully() {
        struct ErrorProvider;
        impl ResearchProvider for ErrorProvider {
            fn name(&self) -> &'static str {
                "error-fixture"
            }

            fn research(&self, _query: &ResearchQuery) -> Result<ResearchBundle, String> {
                Err("network disabled in phase 1".into())
            }
        }

        let query = ResearchQuery {
            question: "달러 강세가 코스피에 부담임?".into(),
            linked: None,
        };
        let bundle = research_bundle_from_provider(&ErrorProvider, &query);
        assert_eq!(bundle.provider, "error-fixture");
        assert!(bundle.sources.is_empty());
        assert!(bundle.notes.iter().any(|n| n.contains("failed gracefully")));
    }

    #[test]
    fn feedback_has_counter_view() {
        let f = make_feedback("금리가 부담인데도 반도체가 버티는 것 같다", None);
        assert!(f.claim.contains("rates"));
        assert!(f.thesis_type.contains("rates/growth"));
        assert!(f.counter.iter().any(|x| x.contains("Semis strength")));
        assert!(f.check.iter().any(|x| x.to_lowercase().contains("yields")));
    }

    #[test]
    fn event_thesis_exposes_timing_evidence_gap() {
        let f = make_feedback(
            "대형 IPO 상장 때문에 금리 완화 기대처럼 보이는 것 아닐까",
            None,
        );
        assert!(f.thesis_type.contains("event-driven"));
        assert!(f.check.iter().any(|x| x.to_lowercase().contains("timing")));
    }

    #[test]
    fn review_summarizes_inquiries_and_habits() {
        let events = vec![
            "{\"type\":\"inquiry\",\"timestamp\":\"t\",\"question\":\"금리와 IPO 상장이 성장주에 영향?\",\"thesis_type\":\"event-driven / supply-calendar thesis\",\"concepts\":\"rates vs growth\"}".into(),
            "{\"type\":\"research_inquiry\",\"timestamp\":\"t\",\"question\":\"달러가 코스피에 부담?\",\"provider\":\"noop\",\"source_count\":0,\"thesis_type\":\"dollar-liquidity transmission thesis\",\"concepts\":\"dollar liquidity\"}".into(),
            "{\"type\":\"thought\",\"timestamp\":\"t\",\"text\":\"달러가 강한데 코스피가 버틴다\",\"linked_pulse_timestamp\":null}".into(),
        ];
        let out = render_review_from_events(&events, "/tmp/journal.jsonl");
        assert!(out.contains("inquiries 1"));
        assert!(out.contains("research 1"));
        assert!(out.contains("Question / thesis habits"));
        assert!(out.contains("event"));
        assert!(out.contains("rates"));
    }

    #[test]
    fn review_date_filter_keeps_only_matching_timestamp_date() {
        let events = vec![
            "{\"type\":\"inquiry\",\"timestamp\":\"2026-04-20T09:00:00+0900\",\"question\":\"달러가 코스피에 부담?\",\"thesis_type\":\"dollar-liquidity transmission thesis\",\"concepts\":\"dollar liquidity\"}".into(),
            "{\"type\":\"thought\",\"timestamp\":\"2026-04-21T10:00:00+0900\",\"text\":\"금리가 내려가는데 성장주가 버틴다\",\"linked_pulse_timestamp\":null}".into(),
            "{\"type\":\"research_inquiry\",\"timestamp\":\"2026-04-21T11:00:00+0900\",\"question\":\"대형 IPO가 시장에 영향?\",\"provider\":\"noop\",\"source_count\":0,\"thesis_type\":\"event-driven / supply-calendar thesis\",\"concepts\":\"event-driven supply/attention\"}".into(),
        ];
        let filtered = filter_events_by_date(events, "2026-04-21", 80);
        assert_eq!(filtered.len(), 2);
        let out = render_review_for_date_from_events(&filtered, "/tmp/journal.jsonl", "2026-04-21");
        assert!(out.contains("Review date filter: 2026-04-21"));
        assert!(out.contains("Entries scanned: 2"));
        assert!(out.contains("rates"));
        assert!(out.contains("event"));
        assert!(!out.contains("dollar-liquidity"));
    }

    #[test]
    fn review_date_filter_applies_limit_after_date_match() {
        let events = vec![
            "{\"type\":\"thought\",\"timestamp\":\"2026-04-21T09:00:00+0900\",\"text\":\"금리\",\"linked_pulse_timestamp\":null}".into(),
            "{\"type\":\"thought\",\"timestamp\":\"2026-04-21T10:00:00+0900\",\"text\":\"달러\",\"linked_pulse_timestamp\":null}".into(),
            "{\"type\":\"thought\",\"timestamp\":\"2026-04-21T11:00:00+0900\",\"text\":\"유가\",\"linked_pulse_timestamp\":null}".into(),
        ];
        let filtered = filter_events_by_date(events, "2026-04-21", 2);
        assert_eq!(filtered.len(), 2);
        assert!(filtered[0].contains("10:00:00"));
        assert!(filtered[1].contains("11:00:00"));
    }

    #[test]
    fn review_date_filter_has_empty_date_message() {
        let out = render_review_for_date_from_events(&[], "/tmp/journal.jsonl", "2026-04-19");
        assert!(out.contains("No market-pulse journal entries found for 2026-04-19"));
        assert!(out.contains("mp review --limit N"));
    }

    #[test]
    fn review_date_validation_rejects_non_iso_date() {
        assert!(validate_review_date("2026-04-21").is_ok());
        assert!(validate_review_date("20260421").is_err());
        assert!(validate_review_date("2026-99-99").is_err());
        assert!(validate_review_date("yesterday").is_err());
        assert!(review(&["review".into(), "--date".into(), "bad".into()]).is_err());
    }

    #[test]
    fn review_days_ago_validation_accepts_small_whole_numbers() {
        assert_eq!(parse_review_days_ago("0").unwrap(), 0);
        assert_eq!(parse_review_days_ago("5").unwrap(), 5);
        assert!(parse_review_days_ago("-1").is_err());
        assert!(parse_review_days_ago("1.5").is_err());
        assert!(parse_review_days_ago("4000").is_err());
    }

    #[test]
    fn review_rejects_multiple_date_selectors() {
        assert!(review(&[
            "review".into(),
            "--date".into(),
            "2026-04-21".into(),
            "--ago".into(),
            "1".into(),
        ])
        .is_err());
    }

    #[test]
    fn compose_rates_tension() {
        let p = compose_test_pulse(vec![
            Asset {
                symbol: "^IXIC",
                label: "Nasdaq",
                unit: "",
                value: Some(100.0),
                change: Some(-0.8),
                note: None,
            },
            Asset {
                symbol: "^TNX",
                label: "US 10Y",
                unit: "%",
                value: Some(4.8),
                change: Some(1.1),
                note: None,
            },
            Asset {
                symbol: "DX-Y.NYB",
                label: "DXY",
                unit: "",
                value: Some(105.0),
                change: Some(0.3),
                note: None,
            },
        ]);
        assert!(p.tensions.iter().any(|t| t.contains("rates")));
        assert!(!p.question.is_empty());
        assert!(!question_seeds_for(&p).is_empty());
        let rendered = render_pulse(&p, false);
        assert!(rendered.contains("Market puzzle / question seeds"));
        assert!(rendered.contains("Basis"));
        assert!(rendered.contains("not a weekly return"));
    }

    #[test]
    fn regime_renders_timeframe_basis_and_boundaries() {
        let regime = compose_test_regime(vec![
            Asset {
                symbol: "^IXIC",
                label: "Nasdaq",
                unit: "",
                value: Some(100.0),
                change: Some(8.0),
                note: None,
            },
            Asset {
                symbol: "^GSPC",
                label: "S&P 500",
                unit: "",
                value: Some(100.0),
                change: Some(4.0),
                note: None,
            },
            Asset {
                symbol: "^TNX",
                label: "US 10Y",
                unit: "%",
                value: Some(4.8),
                change: Some(4.0),
                note: None,
            },
            Asset {
                symbol: "DX-Y.NYB",
                label: "DXY",
                unit: "",
                value: Some(105.0),
                change: Some(1.2),
                note: None,
            },
        ]);
        assert!(regime.label.contains("resilience") || regime.label.contains("risk-on"));
        let rendered = render_regime(&regime);
        assert!(rendered.contains("Market Regime"));
        assert!(rendered.contains("1-3 month regime read"));
        assert!(rendered.contains("Next better regime question"));
        assert!(rendered.contains("not investment advice"));
    }

    fn compose_test_pulse(assets: Vec<Asset>) -> Pulse {
        let avg_equity = avg_change(&assets, &["^GSPC", "^IXIC", "^KS11"]).unwrap_or(0.0);
        let mood = infer_mood(
            avg_equity,
            change_for(&assets, "DX-Y.NYB"),
            change_for(&assets, "^TNX"),
            change_for(&assets, "BTC-USD"),
        );
        let drivers = infer_drivers(&assets);
        let tensions = infer_tensions(&assets);
        Pulse {
            timestamp: timestamp(),
            session: session(),
            basis: PULSE_QUOTE_BASIS.iter().map(|s| (*s).to_string()).collect(),
            question: infer_question(&tensions, &mood),
            concept: infer_concept(&tensions, &drivers),
            mood,
            assets,
            drivers,
            tensions,
            notes: vec![],
        }
    }

    fn compose_test_regime(assets: Vec<Asset>) -> Regime {
        let avg_equity = avg_change(&assets, &["^GSPC", "^IXIC", "^KS11"]).unwrap_or(0.0);
        let label = infer_regime_label(
            avg_equity,
            change_for(&assets, "DX-Y.NYB"),
            change_for(&assets, "^TNX"),
            change_for(&assets, "CL=F"),
            change_for(&assets, "BTC-USD"),
        );
        let drivers = infer_regime_drivers(&assets);
        let tensions = infer_regime_tensions(&assets, &label);
        Regime {
            timestamp: timestamp(),
            basis: REGIME_QUOTE_BASIS
                .iter()
                .map(|s| (*s).to_string())
                .collect(),
            question: infer_regime_question(&label, &tensions),
            checks: infer_regime_checks(&assets, &label),
            label,
            assets,
            drivers,
            tensions,
            notes: vec![],
        }
    }

    fn fixture_research_bundle() -> ResearchBundle {
        ResearchBundle {
            provider: "fixture".into(),
            sources: vec![ResearchSource {
                title: "Fixture IPO calendar".into(),
                publisher: "market-pulse fixture".into(),
                url: "fixture://ipo-calendar".into(),
                published_at: Some("2026-04-20T00:00:00Z".into()),
                relevance: "event timing".into(),
                evidence: "The fixture says event timing must precede the market move.".into(),
            }],
            notes: vec!["deterministic fixture source for tests".into()],
        }
    }
}

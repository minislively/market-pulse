use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

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
    mood: String,
    assets: Vec<Asset>,
    drivers: Vec<String>,
    tensions: Vec<String>,
    question: String,
    concept: String,
    notes: Vec<String>,
}

#[derive(Clone, Debug)]
struct Feedback {
    timestamp: String,
    thought: String,
    linked: Option<String>,
    claim: String,
    good: Vec<String>,
    check: Vec<String>,
    counter: Vec<String>,
    next: Vec<String>,
    concepts: Vec<String>,
}

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
    match args.first().map(String::as_str) {
        None | Some("now") => now(&args),
        Some("think") => think(&args),
        Some("review") => review(&args),
        Some("help") | Some("--help") | Some("-h") => {
            println!("Usage:\n  mp now [--compact] [--no-save]\n  mp think <your market interpretation> [--no-save]\n  mp review [--limit N]");
            Ok(())
        }
        Some(cmd) => Err(format!("unknown command '{cmd}'")),
    }
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

fn review(args: &[String]) -> Result<(), String> {
    let mut limit = 80usize;
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--limit" {
            if let Some(raw) = args.get(i + 1) {
                limit = raw
                    .parse()
                    .map_err(|_| "--limit must be a number".to_string())?;
            }
            i += 1;
        }
        i += 1;
    }
    println!("{}", render_review(limit));
    Ok(())
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
        mood,
        assets,
        drivers,
        tensions,
        question,
        concept,
        notes,
    }
}

fn fetch_asset(
    symbol: &'static str,
    label: &'static str,
    unit: &'static str,
) -> Result<Asset, String> {
    let url = format!(
        "https://query1.finance.yahoo.com/v8/finance/chart/{}?range=5d&interval=1d",
        encode_symbol(symbol)
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
    let value = number_after(&body, "\"regularMarketPrice\":")
        .or_else(|| close_values(&body).last().copied());
    let previous = number_after(&body, "\"chartPreviousClose\":").or_else(|| {
        let vals = close_values(&body);
        vals.get(vals.len().saturating_sub(2)).copied()
    });
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
    ];
    for (tag, needles) in defs {
        if needles.iter().any(|n| lower.contains(&n.to_lowercase())) {
            tags.push(*tag);
        }
    }
    tags.sort_unstable();
    tags
}

fn make_feedback(text: &str, linked: Option<String>) -> Feedback {
    let tags = detect_tags(text);
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
        good: good(&tags),
        check: checks(&tags),
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
    if v.is_empty() {
        v.push("The same price action may come from positioning, liquidity, news timing, or sector rotation; keep at least one alternative open.".into());
    }
    v
}

fn next_questions(tags: &[&str]) -> Vec<String> {
    let mut v = Vec::new();
    if tags.contains(&"rates") && tags.contains(&"semis") {
        v.push("If yields rise further, do semis still outperform the broad market?".into());
    }
    if tags.contains(&"fx") && tags.contains(&"korea") {
        v.push("Does KRW weakness coincide with foreign selling, or are exporters offsetting the pressure?".into());
    }
    if tags.contains(&"oil") {
        v.push("Is oil moving enough to change inflation expectations, or is it only a sector input today?".into());
    }
    if v.is_empty() {
        v.push(
            "What would you need to see by the close to say this interpretation was wrong?".into(),
        );
    }
    v
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
    if compact {
        return format!(
            "[mp] {} · {} · Q: {}",
            p.mood,
            p.drivers
                .iter()
                .take(2)
                .cloned()
                .collect::<Vec<_>>()
                .join("; "),
            p.question
        );
    }
    let mut out = format!(
        "Market Pulse · {} · {}\n\nMood\n  {}\n\nAssets\n",
        p.timestamp, p.session, p.mood
    );
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
    if !p.notes.is_empty() {
        out.push_str("\nSource notes\n");
        for n in &p.notes {
            out.push_str(&format!("  - {n}\n"));
        }
    }
    out
}

fn render_feedback(f: &Feedback) -> String {
    let mut out = format!(
        "Feedback · {}\n\nClaim\n  {}\n\nGood\n",
        f.timestamp, f.claim
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
    let mut lines = text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    if lines.len() > limit {
        lines = lines.split_off(lines.len() - limit);
    }
    lines
}

fn latest_pulse_timestamp() -> Option<String> {
    read_events(usize::MAX)
        .into_iter()
        .rev()
        .find(|l| l.contains("\"type\":\"pulse\""))
        .and_then(|l| json_field(&l, "timestamp"))
}

fn json_field(line: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\":\"");
    let start = line.find(&needle)? + needle.len();
    let rest = &line[start..];
    let end = rest.find('"')?;
    Some(rest[..end].replace("\\\"", "\"").replace("\\n", "\n"))
}

fn render_review(limit: usize) -> String {
    let events = read_events(limit);
    if events.is_empty() {
        return "No market-pulse journal entries yet. Start with `mp now`, then `mp think \"...\"`.".into();
    }
    let pulses = events
        .iter()
        .filter(|l| l.contains("\"type\":\"pulse\""))
        .count();
    let thoughts = events
        .iter()
        .filter(|l| l.contains("\"type\":\"thought\""))
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
    ];
    for line in events.iter().filter(|l| l.contains("\"type\":\"thought\"")) {
        let text = json_field(line, "text").unwrap_or_default();
        for tag in detect_tags(&text) {
            if let Some((_, count)) = counts.iter_mut().find(|(name, _)| *name == tag) {
                *count += 1;
            }
        }
    }
    counts.sort_by(|a, b| b.1.cmp(&a.1));
    let mut out = format!("Market Pulse Review\n\nJournal: {}\nEntries scanned: {} · pulses {} · thoughts {} · feedback {}\n\nRepeated themes\n", journal_path().display(), events.len(), pulses, thoughts, feedback);
    let mut wrote = false;
    for (tag, count) in counts.into_iter().filter(|(_, c)| *c > 0).take(6) {
        wrote = true;
        out.push_str(&format!("  - {tag}: {count}\n"));
    }
    if !wrote {
        out.push_str("  - Not enough tagged thoughts yet\n");
    }
    out.push_str("\nSuggested drill\n  For the next 3 notes, explicitly separate:\n  1. market-wide signal\n  2. sector-specific signal\n  3. alternative explanation");
    out
}

fn pulse_json(p: &Pulse) -> String {
    format!("{{\"type\":\"pulse\",\"timestamp\":\"{}\",\"session\":\"{}\",\"mood\":\"{}\",\"question\":\"{}\",\"concept\":\"{}\"}}", esc(&p.timestamp), esc(&p.session), esc(&p.mood), esc(&p.question), esc(&p.concept))
}

fn thought_json(text: &str, linked: Option<&str>) -> String {
    format!("{{\"type\":\"thought\",\"timestamp\":\"{}\",\"text\":\"{}\",\"linked_pulse_timestamp\":{}}}", timestamp(), esc(text), opt_json(linked))
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
    fn detects_korean_tags() {
        let tags = detect_tags("금리가 부담인데 반도체가 버티고 달러도 강하다");
        assert!(tags.contains(&"rates"));
        assert!(tags.contains(&"semis"));
        assert!(tags.contains(&"fx"));
    }

    #[test]
    fn feedback_has_counter_view() {
        let f = make_feedback("금리가 부담인데도 반도체가 버티는 것 같다", None);
        assert!(f.claim.contains("rates"));
        assert!(f.counter.iter().any(|x| x.contains("Semis strength")));
        assert!(f.check.iter().any(|x| x.to_lowercase().contains("yields")));
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
            question: infer_question(&tensions, &mood),
            concept: infer_concept(&tensions, &drivers),
            mood,
            assets,
            drivers,
            tensions,
            notes: vec![],
        }
    }
}

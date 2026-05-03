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
struct DailyDecisionChecklist {
    scenario: &'static str,
    confirm: &'static str,
    falsify: &'static str,
    watch: &'static str,
    discipline: &'static str,
    journal: &'static str,
}

#[derive(Clone, Debug)]
struct FomoCheckpoint {
    timestamp: String,
    linked_pulse: Option<String>,
    linked_radar: Option<String>,
    scenario: Option<String>,
    confirm: Option<String>,
    falsify: Option<String>,
    watch: Option<String>,
    prompt: String,
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
struct Weekly {
    timestamp: String,
    basis: Vec<String>,
    label: String,
    assets: Vec<Asset>,
    drivers: Vec<String>,
    tensions: Vec<String>,
    questions: Vec<String>,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EarningsBucket {
    Recent,
    Upcoming,
}

#[derive(Clone, Debug, Default)]
struct EarningsFields {
    ticker: Option<String>,
    company: Option<String>,
    report_date: Option<String>,
    timing: Option<String>,
    eps_actual: Option<String>,
    eps_estimate: Option<String>,
    revenue_actual: Option<String>,
    revenue_estimate: Option<String>,
    surprise: Option<String>,
    guidance: Option<String>,
    price_reaction: Option<String>,
}

#[derive(Clone, Debug)]
struct EarningsHint {
    source: ResearchSource,
    fields: EarningsFields,
    freshness: &'static str,
}

#[derive(Clone, Debug)]
struct EarningsBundle {
    timestamp: String,
    provider: String,
    recent: Vec<EarningsHint>,
    upcoming: Vec<EarningsHint>,
    notes: Vec<String>,
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
    Watch,
    Fomo,
    Week,
    Calendar,
    Regime,
    Think,
    Review,
    Find,
    Earnings,
    Help,
    Inquiry { text: String, no_save: bool },
    Research { text: String, no_save: bool },
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ParsedCommand {
    kind: CommandKind,
    args: Vec<String>,
}

const PULSE_QUOTE_BASIS: &[&str] = &[
    "time: local machine timestamp and session label",
    "change: latest Yahoo daily close value vs prior daily close; regularMarketPrice is fallback only",
    "window: Yahoo chart range=5d interval=1d; close-to-close pulse, not high/low gap, exact 24h, or weekly return",
];

const REGIME_QUOTE_BASIS: &[&str] = &[
    "time: local machine timestamp; regime is broader than today's pulse",
    "change: latest Yahoo weekly close value vs first available weekly close; regularMarketPrice is fallback only",
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

const PULSE_ONLY_SYMBOLS: &[(&str, &str, &str)] = &[("^SOX", "Semis", "")];

const FOMO_JOURNAL_PROMPT: &str =
    "Run `mp think` with one claim, one confirming signal, and one falsifier.";

pub fn main_entry() {
    if let Err(err) = run(env::args().skip(1).collect()) {
        eprintln!("mp: {err}");
        std::process::exit(1);
    }
}

fn run(args: Vec<String>) -> Result<(), String> {
    let parsed = parse_command_args(&args)?;
    match parsed.kind {
        CommandKind::Now => now(&parsed.args),
        CommandKind::Watch => watch(&parsed.args),
        CommandKind::Fomo => fomo(&parsed.args),
        CommandKind::Week => week(&parsed.args),
        CommandKind::Calendar => calendar(&parsed.args),
        CommandKind::Regime => regime(&parsed.args),
        CommandKind::Think => think(&parsed.args),
        CommandKind::Review => review(&parsed.args),
        CommandKind::Find => find(&parsed.args),
        CommandKind::Earnings => earnings(&parsed.args),
        CommandKind::Help => {
            print_help();
            Ok(())
        }
        CommandKind::Inquiry { text, no_save } => inquiry(&text, no_save),
        CommandKind::Research { text, no_save } => research_inquiry(&text, no_save),
    }
}

#[cfg(test)]
fn parse_command(args: &[String]) -> Result<CommandKind, String> {
    parse_command_args(args).map(|parsed| parsed.kind)
}

fn parse_command_args(args: &[String]) -> Result<ParsedCommand, String> {
    match args.first().map(String::as_str) {
        None => Ok(parsed(CommandKind::Now, args)),
        Some("now") => Ok(parsed(CommandKind::Now, args)),
        Some("watch") => Ok(parsed(CommandKind::Watch, args)),
        Some("fomo") => Ok(parsed(CommandKind::Fomo, args)),
        Some("week") | Some("weekly") => Ok(parsed(CommandKind::Week, args)),
        Some("calendar") | Some("cal") => Ok(parsed(CommandKind::Calendar, args)),
        Some("regime") => Ok(parsed(CommandKind::Regime, args)),
        Some("think") => Ok(parsed(CommandKind::Think, args)),
        Some("review") => Ok(parsed(CommandKind::Review, args)),
        Some("find") | Some("search") => Ok(parsed(CommandKind::Find, args)),
        Some("earnings") => Ok(parsed(earnings_command(args)?, args)),
        Some("help") | Some("--help") | Some("-h") => Ok(parsed(CommandKind::Help, args)),
        Some("ask") => Ok(parsed(
            inquiry_command(&args[1..], "`mp ask` needs a question")?,
            args,
        )),
        Some("research") => Ok(parsed(
            research_command(&args[1..], "`mp research` needs a question")?,
            args,
        )),
        Some("--research") => Ok(parsed(
            research_command(args, "`mp --research` needs a question")?,
            args,
        )),
        Some(first) if first.starts_with('-') => Err(format!("unknown option '{first}'")),
        Some(_) if args.iter().any(|arg| arg == "--research") => Ok(parsed(
            inquiry_command(args, "`mp` needs a market question")?,
            args,
        )),
        Some(_) => natural_command(args).unwrap_or_else(|| {
            inquiry_command(args, "`mp` needs a market question").map(|kind| parsed(kind, args))
        }),
    }
}

fn parsed(kind: CommandKind, args: &[String]) -> ParsedCommand {
    ParsedCommand {
        kind,
        args: args.to_vec(),
    }
}

fn parsed_with_args(kind: CommandKind, args: Vec<String>) -> ParsedCommand {
    ParsedCommand { kind, args }
}

fn natural_command(args: &[String]) -> Option<Result<ParsedCommand, String>> {
    if let Some(parsed) = natural_review_command(args) {
        return Some(Ok(parsed));
    }
    if let Some(parsed) = natural_find_command(args) {
        return Some(Ok(parsed));
    }
    if let Some(parsed) = natural_think_command(args) {
        return Some(Ok(parsed));
    }
    if has_research_intent(args) {
        return Some(
            research_command(args, "`mp` needs a market question").map(|kind| parsed(kind, args)),
        );
    }
    if is_natural_now(args) {
        return Some(Ok(parsed(CommandKind::Now, args)));
    }
    if is_natural_week(args) {
        return Some(Ok(parsed(CommandKind::Week, args)));
    }
    if is_natural_regime(args) {
        return Some(Ok(parsed(CommandKind::Regime, args)));
    }
    if is_natural_calendar(args) {
        return Some(Ok(parsed(CommandKind::Calendar, args)));
    }
    None
}

fn natural_review_command(args: &[String]) -> Option<ParsedCommand> {
    let (tokens, passthrough) = natural_tokens_and_flags(args, FlagPolicy::ReviewOrFind);
    if !tokens
        .iter()
        .any(|token| token.contains("복기") || token.contains("리뷰"))
    {
        return None;
    }

    let alias = review_alias_from_tokens(&tokens)?;
    let mut normalized = vec!["review".to_string(), alias.to_string()];
    normalized.extend(passthrough);
    Some(parsed_with_args(CommandKind::Review, normalized))
}

fn natural_find_command(args: &[String]) -> Option<ParsedCommand> {
    let (tokens, passthrough) = natural_tokens_and_flags(args, FlagPolicy::ReviewOrFind);
    if !tokens.iter().any(|token| {
        token.contains("찾")
            || token.contains("검색")
            || matches!(token.as_str(), "전에" | "지난번" | "기억")
    }) {
        return None;
    }

    let query = tokens
        .into_iter()
        .filter(|token| !is_find_route_word(token))
        .collect::<Vec<_>>();
    if query.is_empty() {
        return None;
    }

    let mut normalized = vec!["find".to_string()];
    normalized.extend(query);
    normalized.extend(passthrough);
    Some(parsed_with_args(CommandKind::Find, normalized))
}

fn natural_think_command(args: &[String]) -> Option<ParsedCommand> {
    let (tokens, passthrough) = natural_tokens_and_flags(args, FlagPolicy::Think);
    if !tokens.iter().any(|token| {
        matches!(token.as_str(), "생각" | "메모" | "기록" | "판단") || token.starts_with("생각:")
    }) {
        return None;
    }

    let thought = tokens
        .into_iter()
        .flat_map(|token| think_text_parts(&token))
        .collect::<Vec<_>>();
    if thought.is_empty() {
        return None;
    }

    let mut normalized = vec!["think".to_string()];
    normalized.extend(thought);
    normalized.extend(passthrough);
    Some(parsed_with_args(CommandKind::Think, normalized))
}

fn review_alias_from_tokens(tokens: &[String]) -> Option<&'static str> {
    if tokens.iter().any(|token| token.contains("지난주")) {
        return Some("--last-week");
    }
    if tokens
        .iter()
        .any(|token| token.contains("이번주") || token.contains("주간"))
    {
        return Some("--this-week");
    }
    if tokens.iter().any(|token| token.contains("어제")) {
        return Some("--yesterday");
    }
    if tokens.iter().any(|token| token.contains("오늘")) {
        return Some("--today");
    }
    None
}

fn is_find_route_word(token: &str) -> bool {
    token.contains("찾")
        || token.contains("검색")
        || matches!(token, "전에" | "지난번" | "기억" | "내용")
}

fn think_text_parts(token: &str) -> Vec<String> {
    match token {
        "내" | "생각" | "메모" | "기록" | "판단" => Vec::new(),
        token if token.starts_with("생각:") => {
            let rest = token.trim_start_matches("생각:").trim();
            if rest.is_empty() {
                Vec::new()
            } else {
                vec![rest.to_string()]
            }
        }
        _ => vec![token.to_string()],
    }
}

fn has_research_intent(args: &[String]) -> bool {
    let (tokens, _) = natural_tokens_and_flags(args, FlagPolicy::TextOnly);
    tokens.iter().any(|token| {
        token.contains("리서치")
            || token.contains("근거")
            || token.contains("출처")
            || token.contains("왜")
            || token.contains("확인")
            || token.contains("뉴스")
            || token.contains("자료")
            || token.contains("팩트체크")
    })
}

fn is_natural_now(args: &[String]) -> bool {
    let (tokens, _) = natural_tokens_and_flags(args, FlagPolicy::TextOnly);
    let snapshot_like = tokens.iter().any(|token| {
        token.contains("시황")
            || token.contains("시장")
            || token.contains("마켓")
            || token.contains("펄스")
    });
    tokens.len() <= 4 && snapshot_like
}

fn is_natural_week(args: &[String]) -> bool {
    let (tokens, _) = natural_tokens_and_flags(args, FlagPolicy::TextOnly);
    tokens
        .iter()
        .any(|token| token.contains("이번주") || token.contains("주간") || token.contains("한주"))
}

fn is_natural_regime(args: &[String]) -> bool {
    let (tokens, _) = natural_tokens_and_flags(args, FlagPolicy::TextOnly);
    tokens.iter().any(|token| {
        token.contains("레짐")
            || token.contains("국면")
            || token.contains("1-3개월")
            || token.contains("중기")
    }) || (tokens.iter().any(|token| token.contains("흐름"))
        && tokens
            .iter()
            .any(|token| token.contains("큰") || token.contains("중기")))
}

fn is_natural_calendar(args: &[String]) -> bool {
    let (tokens, _) = natural_tokens_and_flags(args, FlagPolicy::TextOnly);
    tokens.iter().any(|token| token.contains("캘린더"))
}

#[derive(Clone, Copy)]
enum FlagPolicy {
    TextOnly,
    ReviewOrFind,
    Think,
}

fn natural_tokens_and_flags(args: &[String], policy: FlagPolicy) -> (Vec<String>, Vec<String>) {
    let mut tokens = Vec::new();
    let mut passthrough = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if matches!(
            arg.as_str(),
            "--limit" | "--date" | "--days" | "--ago" | "--days-ago"
        ) {
            if matches!(policy, FlagPolicy::ReviewOrFind) {
                passthrough.push(arg.clone());
                if let Some(value) = args.get(i + 1) {
                    passthrough.push(value.clone());
                }
            }
            i += 2;
            continue;
        }
        if matches!(
            arg.as_str(),
            "--today" | "--yesterday" | "--this-week" | "--last-week"
        ) {
            if matches!(policy, FlagPolicy::ReviewOrFind) {
                passthrough.push(arg.clone());
            }
            i += 1;
            continue;
        }
        if arg == "--no-save" {
            if matches!(policy, FlagPolicy::Think) {
                passthrough.push(arg.clone());
            }
            i += 1;
            continue;
        }
        if arg.starts_with("--") {
            i += 1;
            continue;
        }
        tokens.push(arg.clone());
        i += 1;
    }
    (tokens, passthrough)
}

fn earnings_command(args: &[String]) -> Result<CommandKind, String> {
    for arg in args.iter().skip(1) {
        match arg.as_str() {
            "--no-save" => {}
            "--save" => {
                return Err("`mp earnings --save` is deferred in v1; earnings does not write journal events yet".into());
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown earnings option '{other}'"));
            }
            other => {
                return Err(format!(
                    "unexpected earnings argument '{other}'; use `mp earnings [--no-save]`"
                ));
            }
        }
    }
    Ok(CommandKind::Earnings)
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

fn help_text() -> &'static str {
    "Usage:\n  mp \"your market question\" [--no-save]\n  mp \"your market question\" --research [--no-save]\n  mp ask <your market question> [--no-save]\n  mp research <your market question> [--no-save]\n  mp now [--compact] [--no-save]\n  mp watch [--no-save]\n  mp fomo [--no-save]\n  mp week [--no-save]\n  mp calendar\n  mp regime [--no-save]\n  mp earnings --no-save\n  mp think <your market interpretation> [--no-save]\n  mp review [--limit N] [--date YYYY-MM-DD|--days N|--today|--yesterday|--this-week|--last-week]\n  mp find <query> [--limit N] [--date YYYY-MM-DD|--days N|--today|--yesterday|--this-week|--last-week]\n\nNatural aliases:\n  mp 오늘 시황\n  mp NVDA\n  mp NVDA 리서치\n  mp 전에 금리 찾아줘 --limit 3\n  mp 이번주 복기"
}

fn print_help() {
    println!("{}", help_text());
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

fn watch(args: &[String]) -> Result<(), String> {
    let no_save = args.iter().any(|a| a == "--no-save");
    let pulse = build_pulse();
    let checklist = daily_decision_checklist(&pulse);
    if !no_save {
        append_event(&radar_json(&pulse, &checklist))?;
    }
    println!("{}", render_radar(&pulse, &checklist));
    Ok(())
}

fn fomo(args: &[String]) -> Result<(), String> {
    let no_save = args.iter().any(|a| a == "--no-save");
    let checkpoint = build_fomo_checkpoint();
    if !no_save {
        append_event(&fomo_check_json(&checkpoint))?;
    }
    println!("{}", render_fomo_checkpoint(&checkpoint));
    Ok(())
}

fn week(args: &[String]) -> Result<(), String> {
    let no_save = args.iter().any(|a| a == "--no-save");
    let weekly = build_week();
    let events = read_week_events(120);
    if !no_save {
        append_event(&week_json(&weekly, events.len()))?;
    }
    println!("{}", render_week(&weekly, &events));
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

fn calendar(_args: &[String]) -> Result<(), String> {
    println!("{}", render_calendar());
    Ok(())
}

fn earnings(args: &[String]) -> Result<(), String> {
    earnings_command(args)?;
    let bundle = build_earnings_bundle();
    println!("{}", render_earnings(&bundle));
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

const EARNINGS_FRESHNESS_DAYS: i64 = 14;
const EARNINGS_RECENT_QUERY: &str =
    "recent major US earnings results EPS revenue guidance stock reaction source";
const EARNINGS_UPCOMING_QUERY: &str =
    "upcoming major US earnings this week next week calendar radar source";

trait EarningsProvider {
    fn name(&self) -> &'static str;
    fn earnings(&self) -> Result<EarningsBundle, String>;
}

struct NoopEarningsProvider;

impl EarningsProvider for NoopEarningsProvider {
    fn name(&self) -> &'static str {
        "noop"
    }

    fn earnings(&self) -> Result<EarningsBundle, String> {
        Ok(EarningsBundle {
            timestamp: timestamp(),
            provider: self.name().into(),
            recent: Vec::new(),
            upcoming: Vec::new(),
            notes: vec![
                "No built-in earnings database or paid API provider is configured.".into(),
                "Configure MARKET_PULSE_SEARCH_CMD for source-backed earnings hints; until then this card is a reasoning scaffold only.".into(),
            ],
        })
    }
}

struct SearchCommandEarningsProvider {
    template: String,
}

impl EarningsProvider for SearchCommandEarningsProvider {
    fn name(&self) -> &'static str {
        "search-cmd"
    }

    fn earnings(&self) -> Result<EarningsBundle, String> {
        let (recent, mut notes) =
            self.fetch_bucket(EarningsBucket::Recent, EARNINGS_RECENT_QUERY)?;
        let (upcoming, upcoming_notes) =
            self.fetch_bucket(EarningsBucket::Upcoming, EARNINGS_UPCOMING_QUERY)?;
        notes.extend(upcoming_notes);
        notes.insert(
            0,
            "MARKET_PULSE_SEARCH_CMD supplied source metadata; optional structured earnings fields are rendered only when explicitly provided.".into(),
        );
        Ok(EarningsBundle {
            timestamp: timestamp(),
            provider: self.name().into(),
            recent,
            upcoming,
            notes,
        })
    }
}

impl SearchCommandEarningsProvider {
    fn fetch_bucket(
        &self,
        bucket: EarningsBucket,
        query: &str,
    ) -> Result<(Vec<EarningsHint>, Vec<String>), String> {
        let args = search_command_args(&self.template, query)?;
        let output = run_command_with_timeout(&args, Duration::from_secs(5))?;
        if !output.status.success() {
            return Err(format!(
                "earnings search command exited with status {}",
                output
                    .status
                    .code()
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "unknown".into())
            ));
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        let (rows, invalid_rows) = parse_earnings_jsonl(&stdout, 10);
        let label = match bucket {
            EarningsBucket::Recent => "recent-results",
            EarningsBucket::Upcoming => "upcoming-radar",
        };
        let today = date_for_days_ago(0).ok();
        let hints = rows
            .into_iter()
            .map(|(source, fields)| EarningsHint {
                freshness: earnings_freshness_with_today(
                    source.published_at.as_deref(),
                    today.as_deref(),
                ),
                source,
                fields,
            })
            .collect::<Vec<_>>();
        let mut notes = vec![format!(
            "{label}: query bucket fixed by command intent, not inferred from source prose."
        )];
        if invalid_rows > 0 {
            notes.push(format!(
                "{label}: {invalid_rows} invalid JSONL source row(s) skipped."
            ));
        }
        if hints.is_empty() {
            notes.push(format!("{label}: no valid source rows returned."));
        }
        Ok((hints, notes))
    }
}

fn build_earnings_bundle() -> EarningsBundle {
    match env::var("MARKET_PULSE_SEARCH_CMD") {
        Ok(template) if !template.trim().is_empty() => {
            let provider = SearchCommandEarningsProvider { template };
            earnings_bundle_from_provider(&provider)
        }
        _ => earnings_bundle_from_provider(&NoopEarningsProvider),
    }
}

fn earnings_bundle_from_provider(provider: &dyn EarningsProvider) -> EarningsBundle {
    provider.earnings().unwrap_or_else(|err| EarningsBundle {
        timestamp: timestamp(),
        provider: provider.name().into(),
        recent: Vec::new(),
        upcoming: Vec::new(),
        notes: vec![format!("earnings provider failed gracefully: {err}")],
    })
}

fn earnings_freshness_with_today(published_at: Option<&str>, today: Option<&str>) -> &'static str {
    let Some(published) = published_at.and_then(date_prefix) else {
        return "unknown";
    };
    let Some(today) = today.and_then(date_prefix) else {
        return "unknown";
    };
    let Some(age) = days_between(&published, &today) else {
        return "unknown";
    };
    if age > EARNINGS_FRESHNESS_DAYS {
        "stale"
    } else {
        "fresh"
    }
}

fn date_prefix(value: &str) -> Option<String> {
    let prefix = value.get(0..10)?;
    validate_review_date(prefix).ok()?;
    Some(prefix.to_string())
}

fn days_between(start: &str, end: &str) -> Option<i64> {
    Some(date_to_day_number(end)? - date_to_day_number(start)?)
}

fn date_to_day_number(date: &str) -> Option<i64> {
    validate_review_date(date).ok()?;
    let year = date[0..4].parse::<i64>().ok()?;
    let month = date[5..7].parse::<i64>().ok()?;
    let day = date[8..10].parse::<i64>().ok()?;
    Some(days_from_civil(year, month, day))
}

fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = year - if month <= 2 { 1 } else { 0 };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let yoe = year - era * 400;
    let mp = month + if month > 2 { -3 } else { 9 };
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe
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

fn parse_earnings_jsonl(
    text: &str,
    limit: usize,
) -> (Vec<(ResearchSource, EarningsFields)>, usize) {
    let mut rows = Vec::new();
    let mut invalid = 0;
    for line in text.lines().filter(|l| !l.trim().is_empty()).take(limit) {
        match earnings_source_from_json_line(line) {
            Some(row) => rows.push(row),
            None => invalid += 1,
        }
    }
    (rows, invalid)
}

fn earnings_source_from_json_line(line: &str) -> Option<(ResearchSource, EarningsFields)> {
    let source = research_source_from_json_line(line)?;
    let fields = EarningsFields {
        ticker: json_field(line, "ticker"),
        company: json_field(line, "company"),
        report_date: json_field(line, "report_date"),
        timing: json_field(line, "timing"),
        eps_actual: json_field(line, "eps_actual"),
        eps_estimate: json_field(line, "eps_estimate"),
        revenue_actual: json_field(line, "revenue_actual"),
        revenue_estimate: json_field(line, "revenue_estimate"),
        surprise: json_field(line, "surprise"),
        guidance: json_field(line, "guidance"),
        price_reaction: json_field(line, "price_reaction"),
    };
    Some((source, fields))
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

#[derive(Clone, Debug)]
enum ReviewFilter {
    Date(String),
    Dates { label: String, dates: Vec<String> },
}

fn review(args: &[String]) -> Result<(), String> {
    let mut limit = 80usize;
    let mut filter: Option<ReviewFilter> = None;
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
            set_review_filter(&mut filter, ReviewFilter::Date(raw.clone()))?;
            i += 1;
        } else if matches!(args[i].as_str(), "--days" | "--ago" | "--days-ago") {
            let Some(raw) = args.get(i + 1) else {
                return Err(format!("{} needs a number of days", args[i]));
            };
            let days = parse_review_days_ago(raw)?;
            set_review_filter(&mut filter, ReviewFilter::Date(date_for_days_ago(days)?))?;
            i += 1;
        } else if matches!(
            args[i].as_str(),
            "--today" | "--yesterday" | "--this-week" | "--last-week"
        ) {
            set_review_filter(&mut filter, review_filter_for_alias(&args[i])?)?;
        }
        i += 1;
    }
    let rendered = if let Some(filter) = filter {
        render_review_for_filter(limit, &filter)
    } else {
        render_review(limit)
    };
    println!("{rendered}");
    Ok(())
}

fn find(args: &[String]) -> Result<(), String> {
    let parsed = parse_find_args(args)?;
    let rendered = render_find(parsed.limit, &parsed.query, parsed.filter.as_ref());
    println!("{rendered}");
    Ok(())
}

#[derive(Clone, Debug)]
struct FindArgs {
    query: String,
    limit: usize,
    filter: Option<ReviewFilter>,
}

fn parse_find_args(args: &[String]) -> Result<FindArgs, String> {
    let mut limit = 20usize;
    let mut filter: Option<ReviewFilter> = None;
    let mut query_parts = Vec::new();
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--limit" {
            let Some(raw) = args.get(i + 1) else {
                return Err("--limit needs a number".into());
            };
            limit = raw
                .parse()
                .map_err(|_| "--limit must be a number".to_string())?;
            i += 2;
            continue;
        }
        if args[i] == "--date" {
            let Some(raw) = args.get(i + 1) else {
                return Err("--date needs YYYY-MM-DD".into());
            };
            validate_review_date(raw)?;
            set_review_filter(&mut filter, ReviewFilter::Date(raw.clone()))?;
            i += 2;
            continue;
        }
        if matches!(args[i].as_str(), "--days" | "--ago" | "--days-ago") {
            let Some(raw) = args.get(i + 1) else {
                return Err(format!("{} needs a number of days", args[i]));
            };
            let days = parse_review_days_ago(raw)?;
            set_review_filter(&mut filter, ReviewFilter::Date(date_for_days_ago(days)?))?;
            i += 2;
            continue;
        }
        if matches!(
            args[i].as_str(),
            "--today" | "--yesterday" | "--this-week" | "--last-week"
        ) {
            set_review_filter(&mut filter, review_filter_for_alias(&args[i])?)?;
            i += 1;
            continue;
        }
        if args[i].starts_with('-') {
            return Err(format!("unknown find option '{}'", args[i]));
        }
        query_parts.push(args[i].clone());
        i += 1;
    }
    let query = query_parts.join(" ").trim().to_string();
    if query.is_empty() {
        return Err("`mp find` needs a journal search query".into());
    }
    Ok(FindArgs {
        query,
        limit,
        filter,
    })
}

fn set_review_filter(filter: &mut Option<ReviewFilter>, next: ReviewFilter) -> Result<(), String> {
    if filter.is_some() {
        return Err("use only one review date selector".into());
    }
    *filter = Some(next);
    Ok(())
}

fn review_filter_for_alias(alias: &str) -> Result<ReviewFilter, String> {
    match alias {
        "--today" => Ok(ReviewFilter::Date(date_for_days_ago(0)?)),
        "--yesterday" => Ok(ReviewFilter::Date(date_for_days_ago(1)?)),
        "--this-week" => {
            let dates = current_week_date_prefixes();
            Ok(ReviewFilter::Dates {
                label: format!("this-week {}", week_window_label(&dates)),
                dates,
            })
        }
        "--last-week" => {
            let dates = last_week_date_prefixes();
            Ok(ReviewFilter::Dates {
                label: format!("last-week {}", week_window_label(&dates)),
                dates,
            })
        }
        _ => Err(format!("unknown review period alias '{alias}'")),
    }
}

fn parse_review_days_ago(raw: &str) -> Result<u32, String> {
    let days = raw
        .parse::<u32>()
        .map_err(|_| "--days must be a non-negative whole number".to_string())?;
    if days <= 3660 {
        Ok(days)
    } else {
        Err("--days must be 3660 days or less".into())
    }
}

fn date_for_days_ago(days: u32) -> Result<String, String> {
    if days == 0 {
        return command_date(&["+%Y-%m-%d"])
            .ok_or_else(|| "--days needs the local `date` command".into());
    }
    let bsd_offset = format!("-v-{days}d");
    if let Some(date) = command_date(&[&bsd_offset, "+%Y-%m-%d"]) {
        return Ok(date);
    }
    let gnu_relative = format!("{days} days ago");
    command_date(&["-d", &gnu_relative, "+%Y-%m-%d"])
        .ok_or_else(|| "--days needs BSD `date -v` or GNU `date -d` support".into())
}

fn current_week_date_prefixes() -> Vec<String> {
    let Some(weekday) = iso_weekday() else {
        return date_prefixes_for_days(7);
    };
    let mut dates = (0..weekday)
        .filter_map(|days_ago| date_for_days_ago(days_ago).ok())
        .collect::<Vec<_>>();
    dates.reverse();
    dates
}

fn last_week_date_prefixes() -> Vec<String> {
    let Some(weekday) = iso_weekday() else {
        let mut dates = (7..14)
            .filter_map(|days_ago| date_for_days_ago(days_ago).ok())
            .collect::<Vec<_>>();
        dates.reverse();
        return dates;
    };
    let mut dates = (weekday..weekday + 7)
        .filter_map(|days_ago| date_for_days_ago(days_ago).ok())
        .collect::<Vec<_>>();
    dates.reverse();
    dates
}

fn iso_weekday() -> Option<u32> {
    command_date_raw(&["+%u"])
        .and_then(|raw| raw.parse::<u32>().ok())
        .filter(|day| (1..=7).contains(day))
}

fn week_basis(dates: &[String]) -> Vec<String> {
    let window = week_window_label(dates);
    vec![
        format!("time: local machine timestamp; current-week window {window}; weekly is a learning loop, not a trading signal"),
        "market window: Yahoo chart range=1mo interval=1d; change is latest daily close vs first close matching the current local week; regularMarketPrice is fallback only, and assets without a current-week close fall back to the latest available close".into(),
        format!("journal window: current local calendar week {window}, filtered before the weekly card is saved"),
    ]
}

fn week_window_label(dates: &[String]) -> String {
    match (dates.first(), dates.last()) {
        (Some(first), Some(last)) if first == last => first.clone(),
        (Some(first), Some(last)) => format!("{first}..{last}"),
        _ => "unavailable".into(),
    }
}

fn date_for_unix_timestamp(seconds: i64) -> Option<String> {
    let raw = seconds.to_string();
    command_date(&["-r", &raw, "+%Y-%m-%d"]).or_else(|| {
        let gnu_timestamp = format!("@{raw}");
        command_date(&["-d", &gnu_timestamp, "+%Y-%m-%d"])
    })
}

fn command_date(args: &[&str]) -> Option<String> {
    let date = command_date_raw(args)?;
    validate_review_date(&date).ok()?;
    Some(date)
}

fn command_date_raw(args: &[&str]) -> Option<String> {
    let output = Command::new("date").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8(output.stdout).ok()?.trim().to_string())
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
    for &(symbol, label, unit) in SYMBOLS.iter().chain(PULSE_ONLY_SYMBOLS.iter()) {
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
    let mut notes = vec![
        "market quotes from Yahoo Finance chart endpoint via curl".to_string(),
        "mixed sessions: US indices, Korea, FX, futures, and crypto can have different clocks; compare directionally"
            .to_string(),
    ];
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

fn build_week() -> Weekly {
    let mut assets = Vec::new();
    let mut failures = 0;
    let week_dates = current_week_date_prefixes();
    for (symbol, label, unit) in SYMBOLS {
        match fetch_asset_window(
            symbol,
            label,
            unit,
            "1mo",
            "1d",
            WindowChange::FirstMatchingDate(week_dates.clone()),
        ) {
            Ok(asset) => assets.push(asset),
            Err(_) => {
                failures += 1;
                assets.push(Asset {
                    symbol,
                    label,
                    unit,
                    value: None,
                    change: None,
                    note: Some("weekly data unavailable".into()),
                });
            }
        }
    }
    let mut notes = vec![
        "weekly market window uses Yahoo Finance chart endpoint via curl".to_string(),
        "weekly journal window uses the current local calendar week".to_string(),
    ];
    if failures > 0 {
        notes.push(format!(
            "{failures} quote(s) unavailable; weekly read is partial"
        ));
    }
    let avg_equity = avg_change(&assets, &["^GSPC", "^IXIC", "^KS11"]).unwrap_or(0.0);
    let label = infer_week_label(
        avg_equity,
        change_for(&assets, "DX-Y.NYB"),
        change_for(&assets, "^TNX"),
        change_for(&assets, "CL=F"),
        change_for(&assets, "BTC-USD"),
    );
    let drivers = infer_week_drivers(&assets);
    let tensions = infer_week_tensions(&assets, &label);
    let questions = infer_week_questions(&assets, &label, &tensions);
    Weekly {
        timestamp: timestamp(),
        basis: week_basis(&week_dates),
        label,
        assets,
        drivers,
        tensions,
        questions,
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

#[derive(Clone, Debug)]
enum WindowChange {
    PriorDailyClose,
    FirstClose,
    FirstMatchingDate(Vec<String>),
}

fn fetch_asset(
    symbol: &'static str,
    label: &'static str,
    unit: &'static str,
) -> Result<Asset, String> {
    fetch_asset_window(
        symbol,
        label,
        unit,
        "5d",
        "1d",
        WindowChange::PriorDailyClose,
    )
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
    let (value, previous) = value_and_previous_for_window(&body, &closes, &change_from);
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

fn value_and_previous_for_window(
    body: &str,
    closes: &[f64],
    change_from: &WindowChange,
) -> (Option<f64>, Option<f64>) {
    let market_price = || number_after(body, "\"regularMarketPrice\":");
    let latest_close = || closes.last().copied();
    let prior_close = || {
        closes
            .len()
            .checked_sub(2)
            .and_then(|i| closes.get(i))
            .copied()
    };

    let value = match change_from {
        WindowChange::PriorDailyClose
        | WindowChange::FirstClose
        | WindowChange::FirstMatchingDate(_) => latest_close().or_else(market_price),
    };
    let previous = match change_from {
        WindowChange::PriorDailyClose => {
            prior_close().or_else(|| number_after(body, "\"chartPreviousClose\":"))
        }
        WindowChange::FirstClose => closes.first().copied(),
        WindowChange::FirstMatchingDate(dates) => {
            first_close_for_dates(body, dates).or_else(|| closes.last().copied())
        }
    };
    (value, previous)
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

fn timestamp_values(text: &str) -> Vec<i64> {
    let Some(start_key) = text.find("\"timestamp\":[") else {
        return vec![];
    };
    let start = start_key + "\"timestamp\":[".len();
    let Some(end) = text[start..].find(']') else {
        return vec![];
    };
    text[start..start + end]
        .split(',')
        .filter_map(|v| v.parse().ok())
        .collect()
}

fn first_close_for_dates(text: &str, dates: &[String]) -> Option<f64> {
    if dates.is_empty() {
        return None;
    }
    timestamp_values(text)
        .into_iter()
        .zip(close_values(text))
        .find_map(|(ts, close)| {
            date_for_unix_timestamp(ts)
                .filter(|date| dates.iter().any(|d| d == date))
                .map(|_| close)
        })
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

fn infer_week_label(
    avg_equity: f64,
    usd: Option<f64>,
    rates: Option<f64>,
    oil: Option<f64>,
    btc: Option<f64>,
) -> String {
    let macro_pressure = usd.is_some_and(|v| v > 0.7)
        || rates.is_some_and(|v| v > 1.5)
        || oil.is_some_and(|v| v > 4.0);
    let high_beta = btc.is_some_and(|v| v > 5.0);
    if avg_equity > 1.5 && !macro_pressure {
        "risk-on learning week"
    } else if avg_equity > 0.5 && macro_pressure {
        "equity resilience vs macro-pressure week"
    } else if avg_equity < -1.5 && macro_pressure {
        "de-risking / macro-pressure week"
    } else if avg_equity < -1.5 {
        "risk-off / growth-doubt week"
    } else if high_beta && avg_equity >= 0.0 {
        "high-beta liquidity watch week"
    } else {
        "mixed / transition learning week"
    }
    .into()
}

fn infer_week_drivers(assets: &[Asset]) -> Vec<String> {
    let mut drivers = Vec::new();
    if change_for(assets, "^IXIC").is_some_and(|v| v > 1.5) {
        drivers.push("Nasdaq strength is the first place to test growth/AI leadership".into());
    }
    if avg_change(assets, &["^GSPC", "^IXIC", "^KS11"]).is_some_and(|v| v > 1.2) {
        drivers.push("Equities are broadly higher over the weekly window".into());
    }
    if change_for(assets, "DX-Y.NYB").is_some_and(|v| v.abs() > 0.7)
        || change_for(assets, "KRW=X").is_some_and(|v| v.abs() > 0.7)
    {
        drivers.push("Dollar/FX moved enough to check whether liquidity pressure mattered".into());
    }
    if change_for(assets, "^TNX").is_some_and(|v| v.abs() > 1.5) {
        drivers.push("US 10Y yield moved enough to test the rates-vs-growth story".into());
    }
    if change_for(assets, "CL=F").is_some_and(|v| v.abs() > 4.0) {
        drivers.push(
            "Oil moved enough to keep inflation/margin narratives in the weekly review".into(),
        );
    }
    if change_for(assets, "BTC-USD").is_some_and(|v| v.abs() > 5.0) {
        drivers.push("BTC/high beta moved enough to check liquidity appetite".into());
    }
    if drivers.is_empty() {
        drivers.push(
            "No single weekly driver dominates; use the journal themes to choose what to study"
                .into(),
        );
        drivers
            .push("Compare equities, rates, dollar, oil, and your own repeated questions".into());
    }
    drivers.truncate(5);
    drivers
}

fn infer_week_tensions(assets: &[Asset], label: &str) -> Vec<String> {
    let mut tensions = Vec::new();
    let avg_equity = avg_change(assets, &["^GSPC", "^IXIC", "^KS11"]).unwrap_or(0.0);
    if avg_equity > 0.5
        && (change_for(assets, "^TNX").is_some_and(|v| v > 1.5)
            || change_for(assets, "DX-Y.NYB").is_some_and(|v| v > 0.7))
    {
        tensions.push("weekly equity strength vs tighter financial-condition signals".into());
    }
    if change_for(assets, "^IXIC").unwrap_or(0.0) - change_for(assets, "^GSPC").unwrap_or(0.0) > 1.0
    {
        tensions.push("Nasdaq/growth leadership vs broad-market confirmation".into());
    }
    if change_for(assets, "^KS11").unwrap_or(0.0) < avg_equity - 1.0 {
        tensions.push("Korea/EM follow-through vs US market story".into());
    }
    if change_for(assets, "CL=F").is_some_and(|v| v > 4.0) && avg_equity > 0.0 {
        tensions.push("risk appetite vs oil/inflation impulse".into());
    }
    if label.contains("transition") {
        tensions.push("this week's pulse may not match the broader regime".into());
    }
    if tensions.is_empty() {
        tensions.push("headline weekly move vs cross-asset confirmation".into());
    }
    tensions.truncate(4);
    tensions
}

fn infer_week_questions(assets: &[Asset], label: &str, tensions: &[String]) -> Vec<String> {
    let text = format!("{label} {}", tensions.join(" ")).to_lowercase();
    let mut tags = detect_tags(&text);
    if text.contains("financial-condition")
        || text.contains("rates")
        || change_for(assets, "^TNX").is_some()
    {
        push_tag(&mut tags, "rates");
    }
    if text.contains("nasdaq") || text.contains("growth") || text.contains("ai") {
        push_tag(&mut tags, "semis");
    }
    if text.contains("korea") || change_for(assets, "^KS11").is_some() {
        push_tag(&mut tags, "korea");
    }
    if text.contains("fx")
        || text.contains("dollar")
        || change_for(assets, "DX-Y.NYB").is_some()
        || change_for(assets, "KRW=X").is_some()
    {
        push_tag(&mut tags, "fx");
    }
    if text.contains("oil") || change_for(assets, "CL=F").is_some_and(|v| v.abs() > 4.0) {
        push_tag(&mut tags, "oil");
    }
    let mut questions = validation_questions(&tags);
    questions
        .push("By next Friday, what evidence would force you to rename this week’s story?".into());
    let mut unique = Vec::new();
    for question in questions {
        if !unique.contains(&question) {
            unique.push(question);
        }
    }
    unique.truncate(5);
    unique
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
        next_question: next_better_question_for(&tags, question),
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
        next: next_questions_for(&tags, text),
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

fn push_tag(tags: &mut Vec<&'static str>, tag: &'static str) {
    if !tags.contains(&tag) {
        tags.push(tag);
    }
}

fn contains_hangul(text: &str) -> bool {
    text.chars().any(|ch| {
        ('가'..='힣').contains(&ch) || ('ㄱ'..='ㅎ').contains(&ch) || ('ㅏ'..='ㅣ').contains(&ch)
    })
}

fn validation_questions_for(tags: &[&str], korean: bool) -> Vec<String> {
    if korean {
        return korean_validation_questions(tags);
    }
    english_validation_questions(tags)
}

fn korean_validation_questions(tags: &[&str]) -> Vec<String> {
    let mut v = Vec::new();
    if tags.contains(&"rates") && tags.contains(&"semis") {
        v.push("성장주 강세가 금리 부담을 흡수한다는 증거는 뭐지?".into());
        v.push("이 금리/성장주 해석을 틀리게 만들 신호는 뭐지?".into());
        v.push("실적, 유동성, 수급, 완화 기대 중 대안가설은 뭐지?".into());
    } else if tags.contains(&"rates") {
        v.push("완화 기대인지 성장 둔화 매수인지 구분할 증거는 뭐지?".into());
        v.push("금리·달러·성장주가 엇갈리면 이 해석은 어떻게 깨지지?".into());
    }
    if tags.contains(&"fx") && tags.contains(&"korea") {
        v.push("환율이 한국 리스크를 주도한다는 증거는 뭐지?".into());
        v.push("원화 약세에도 외국인 매도나 지수 약세가 없으면 어떻게 볼까?".into());
    } else if tags.contains(&"fx") {
        v.push("달러 강세가 전반적 유동성 신호라는 증거는 뭐지?".into());
    }
    if tags.contains(&"oil") {
        v.push("유가가 인플레 기대를 바꾸고 있다는 증거는 뭐지?".into());
    }
    if tags.contains(&"event") {
        v.push("상장/이벤트가 원인이면 어떤 자산이 먼저 움직여야 하지?".into());
        v.push("이벤트 설명을 틀리게 만들 반대 신호는 뭐지?".into());
    }
    if tags.contains(&"positioning") {
        v.push("수급/포지셔닝과 진짜 펀더멘털 변화를 어떻게 구분하지?".into());
    }
    if v.is_empty() {
        v.push("이 해석을 확인할 증거와 틀리게 만들 관찰은 뭐지?".into());
        v.push("같은 움직임을 설명할 다른 가설은 뭐지?".into());
    }
    dedupe_and_limit(v, 4)
}

fn english_validation_questions(tags: &[&str]) -> Vec<String> {
    let mut v = Vec::new();
    if tags.contains(&"rates") && tags.contains(&"semis") {
        v.push("What evidence shows growth leadership is absorbing rate pressure?".into());
        v.push("What would falsify the rates/growth story: higher yields with weak breadth, or lower yields without growth leadership?".into());
        v.push(
            "What alternative fits: earnings, liquidity, positioning, or genuine easing hopes?"
                .into(),
        );
    } else if tags.contains(&"rates") {
        v.push("What confirms easing hopes rather than growth-scare bond buying?".into());
        v.push(
            "What falsifies the rates story if yields, dollar, and growth stop lining up?".into(),
        );
    }
    if tags.contains(&"fx") && tags.contains(&"korea") {
        v.push("What evidence shows FX is driving Korea risk?".into());
        v.push("What falsifies FX pressure: weak KRW without foreign selling, or exporters offsetting it?".into());
    } else if tags.contains(&"fx") {
        v.push("What proves dollar strength is a market-wide liquidity signal?".into());
    }
    if tags.contains(&"oil") {
        v.push("What evidence shows oil is changing inflation expectations?".into());
    }
    if tags.contains(&"event") {
        v.push("What should move first if the IPO/listing story is driving the session?".into());
        v.push("What falsifies the event story: broad assets moving first, or unrelated sectors leading?".into());
    }
    if tags.contains(&"positioning") {
        v.push(
            "What distinguishes positioning/flow from real changes in growth, rates, or earnings?"
                .into(),
        );
    }
    if v.is_empty() {
        v.push("What evidence confirms this view, and what observation makes it wrong?".into());
        v.push("What alternative explains the same tape without a one-cause story?".into());
    }
    dedupe_and_limit(v, 4)
}

fn dedupe_and_limit(questions: Vec<String>, limit: usize) -> Vec<String> {
    let mut unique = Vec::new();
    for question in questions {
        if !unique.contains(&question) {
            unique.push(question);
        }
    }
    unique.truncate(limit);
    unique
}

fn validation_questions(tags: &[&str]) -> Vec<String> {
    validation_questions_for(tags, false)
}

fn next_questions_for(tags: &[&str], source_text: &str) -> Vec<String> {
    validation_questions_for(tags, contains_hangul(source_text))
}

fn next_better_question_for(tags: &[&str], source_text: &str) -> String {
    validation_questions_for(tags, contains_hangul(source_text))
        .into_iter()
        .next()
        .unwrap_or_else(|| {
            if contains_hangul(source_text) {
                "이 해석을 확인할 증거와 틀리게 만들 관찰은 뭐지?".into()
            } else {
                "What evidence would confirm this interpretation, and what would make it wrong?"
                    .into()
            }
        })
}

fn tags_from_summary(summary: &JournalSummary, limit: usize) -> Vec<&'static str> {
    summary
        .tag_counts
        .iter()
        .filter(|(_, count)| *count > 0)
        .take(limit)
        .map(|(tag, _)| *tag)
        .collect()
}

fn recall_question(query: &str, filter: &str, tags: &[&str]) -> String {
    if contains_hangul(query) {
        if tags.contains(&"rates") {
            return format!(
                "기간({filter}) 기준으로 예전 \"{query}\" 메모는 완화 기대/성장 둔화/금리 부담 중 뭐였고, 어떤 금리·달러 움직임이 그 해석을 깨지?"
            );
        }
        if tags.contains(&"fx") || tags.contains(&"korea") {
            return format!(
                "기간({filter}) 기준으로 \"{query}\"는 달러 압력/한국 리스크/섹터 로테이션 중 뭐였고, 어떤 증거가 그 해석을 반박하지?"
            );
        }
        if tags.contains(&"event") || tags.contains(&"positioning") {
            return format!(
                "기간({filter}) 기준으로 \"{query}\"는 이벤트·수급인지 지속 신호인지, 무엇이 시간차 노이즈였음을 보여주지?"
            );
        }
        if tags.contains(&"oil") {
            return format!(
                "기간({filter}) 기준으로 \"{query}\"는 인플레/수요/섹터 회전 중 뭐였고, 무엇이 그 경로를 깨지?"
            );
        }
        return format!(
            "기간({filter}) 기준으로 \"{query}\"를 썼던 때와 지금 무엇이 달라졌고, 어떤 증거가 그 해석을 깨지?"
        );
    }
    if tags.contains(&"rates") {
        return format!(
            "In {filter}, did old \"{query}\" notes assume easing, growth scare, or rate pressure—and which yield/dollar move would falsify that read?"
        );
    }
    if tags.contains(&"fx") || tags.contains(&"korea") {
        return format!(
            "In {filter}, did \"{query}\" mean global dollar pressure, Korea stress, or sector rotation—and what evidence disproves it?"
        );
    }
    if tags.contains(&"event") || tags.contains(&"positioning") {
        return format!(
            "In {filter}, was \"{query}\" event/flow or durable signal—and what proves the old read was timing noise?"
        );
    }
    if tags.contains(&"oil") {
        return format!(
            "In {filter}, did \"{query}\" mean inflation, demand, or sector rotation—and what falsifies that channel now?"
        );
    }
    format!(
        "In {filter}, what changed since \"{query}\", and what evidence falsifies that old interpretation now?"
    )
}

fn review_drill(summary: &JournalSummary) -> String {
    let tags = tags_from_summary(summary, 2);
    let focus = if tags.is_empty() {
        if summary.korean_entries > 0 {
            "다음 반복 테마".to_string()
        } else {
            "your next repeated theme".to_string()
        }
    } else {
        tags.join(" + ")
    };
    if summary.korean_entries > 0 {
        return format!(
            "\nSuggested drill\n  다음 3개 메모에서 {focus}를 검증 루프로 나눠보세요:\n  1. 주장: 내가 말하는 시장 스토리는?\n  2. 증거: 맞다면 다음에 무엇이 움직여야 하지?\n  3. 대안: 같은 흐름을 설명할 다른 가설은?\n  4. 반증: 무엇이 나오면 이 해석을 바꿔야 하지?"
        );
    }
    format!(
        "\nSuggested drill\n  For the next 3 notes, run a validation loop on {focus}:\n  1. claim: what story am I telling?\n  2. evidence: what should move next if I am right?\n  3. alternative: what else explains the same tape?\n  4. falsifier: what would make me rename the view?"
    )
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
    let checklist = daily_decision_checklist(p);
    out.push_str(&render_daily_decision_checklist(&checklist));
    if !p.notes.is_empty() {
        out.push_str("\nSource notes\n");
        for n in &p.notes {
            out.push_str(&format!("  - {n}\n"));
        }
    }
    out
}

fn daily_decision_checklist(p: &Pulse) -> DailyDecisionChecklist {
    let nasdaq = change_for(&p.assets, "^IXIC");
    let spx = change_for(&p.assets, "^GSPC");
    let semis = change_for(&p.assets, "^SOX");
    let kospi = change_for(&p.assets, "^KS11");
    let usd_krw = change_for(&p.assets, "KRW=X");
    let dxy = change_for(&p.assets, "DX-Y.NYB");
    let rates = change_for(&p.assets, "^TNX");
    let oil = change_for(&p.assets, "CL=F");
    let btc = change_for(&p.assets, "BTC-USD");

    let semis_available = semis.is_some();
    let semis_positive = semis.is_some_and(|v| v > 0.75);
    let semis_leading = semis.is_some_and(|v| v > 1.25)
        && nasdaq.is_some_and(|v| v > 0.5)
        && semis.unwrap_or(0.0) - spx.unwrap_or(0.0) >= 0.5;
    let semis_weak = semis.is_some_and(|v| v < -0.5)
        || (nasdaq.is_some_and(|v| v > 0.5) && semis.is_some_and(|v| v < 0.0));
    let equity_positive = [nasdaq, spx, kospi]
        .iter()
        .flatten()
        .any(|change| *change > 0.5);
    let high_beta_positive = btc.is_some_and(|v| v > 1.0) || nasdaq.is_some_and(|v| v > 0.5);
    let fx_pressure = dxy.is_some_and(|v| v > 0.25) || usd_krw.is_some_and(|v| v > 0.25);
    let macro_pressure =
        fx_pressure || rates.is_some_and(|v| v > 0.5) || oil.is_some_and(|v| v > 1.0);
    let korea_strong = kospi.is_some_and(|v| v > 0.6);

    let scenario = if semis_leading && macro_pressure {
        "Semis-led growth risk-on; macro confirmation incomplete"
    } else if semis_leading {
        "Semis-led growth risk-on; watch BTC/KOSPI confirmation"
    } else if semis_available && semis_weak {
        "Growth leadership fading; watch whether semis or BTC breaks first"
    } else if korea_strong && fx_pressure && semis_positive {
        "Korea/EM beta confirmation needed despite semis strength"
    } else if korea_strong && fx_pressure {
        "Korea/EM beta confirmation needed"
    } else if semis_available && nasdaq.is_some_and(|v| v > 0.5) && !semis_positive {
        "Nasdaq risk-on with semis confirmation still pending"
    } else if high_beta_positive && macro_pressure {
        "High-beta risk-on attempt; macro confirmation incomplete"
    } else if high_beta_positive || equity_positive {
        "Broad risk-on attempt; watch dollar/rates confirmation"
    } else if macro_pressure && nasdaq.is_some_and(|v| v > 0.0) {
        "Macro pressure fighting growth leadership"
    } else {
        "Mixed tape; wait for confirmation rather than forcing a one-cause story"
    };

    let confirm = if semis_leading && macro_pressure {
        "Semis and Nasdaq keep leading while BTC/KOSPI confirm and dollar/rates pressure stops rising."
    } else if semis_leading {
        "Semis and Nasdaq keep leading while BTC and KOSPI confirm the same risk tone."
    } else if semis_available && semis_weak {
        "Semis stabilize before Nasdaq, BTC, and KOSPI lose the same growth story."
    } else if semis_available && nasdaq.is_some_and(|v| v > 0.5) && !semis_positive {
        "Semis stop lagging while Nasdaq, BTC, and KOSPI keep confirming together."
    } else if korea_strong && fx_pressure {
        "KOSPI strength holds while USD/KRW and DXY stop adding pressure."
    } else if high_beta_positive && macro_pressure {
        "Equities and BTC keep confirming while dollar/rates pressure stops rising."
    } else if high_beta_positive || equity_positive {
        "Nasdaq, S&P 500, KOSPI, and BTC keep pointing in the same direction."
    } else {
        "At least two assets confirm the same story in the next check."
    };

    let falsify = if semis_available {
        "Semis lose leadership first, or Nasdaq weakness pulls BTC/KOSPI lower while dollar or rates pressure rises."
    } else if macro_pressure {
        "Nasdaq weakness pulls BTC/KOSPI lower while dollar or rates pressure rises."
    } else {
        "The strongest index fades first and cross-asset confirmation disappears."
    };

    DailyDecisionChecklist {
        scenario,
        confirm,
        falsify,
        watch: if semis_available {
            "Semis vs Nasdaq, BTC, KOSPI, DXY, USD/KRW, and US 10Y."
        } else {
            "Nasdaq vs S&P 500, BTC, DXY, USD/KRW, US 10Y, and KOSPI."
        },
        discipline: if semis_available {
            "Treat laggards as unproven unless they hold during Nasdaq/semis weakness."
        } else {
            "Treat laggards as unproven unless they hold when Nasdaq weakens."
        },
        journal: if semis_available {
            "Run `mp think` with one leadership claim, one confirming signal, and one falsifier."
        } else {
            "Run `mp think` with one claim, one confirming signal, and one falsifier."
        },
    }
}

fn render_daily_decision_checklist(checklist: &DailyDecisionChecklist) -> String {
    format!(
        "\nDaily Decision Checklist\n  Scenario: {}\n  Confirm: {}\n  Falsify: {}\n  Watch: {}\n  Discipline: {}\n  Journal: {}\n",
        checklist.scenario,
        checklist.confirm,
        checklist.falsify,
        checklist.watch,
        checklist.discipline,
        checklist.journal
    )
}

fn render_radar(p: &Pulse, checklist: &DailyDecisionChecklist) -> String {
    let mut out = format!(
        "Market Radar · {} · {}\n\nMood\n  {}\n\nBasis\n",
        p.timestamp, p.session, p.mood
    );
    for b in &p.basis {
        out.push_str(&format!("  - {b}\n"));
    }
    out.push_str(&format!(
        "\nRadar\n  Scenario: {}\n  Watch: {}\n  Confirm: {}\n  Falsify: {}\n",
        checklist.scenario, checklist.watch, checklist.confirm, checklist.falsify
    ));
    if !p.drivers.is_empty() {
        out.push_str("\nDrivers to explain, not chase\n");
        for (i, d) in p.drivers.iter().take(3).enumerate() {
            out.push_str(&format!("  {}. {d}\n", i + 1));
        }
    }
    if !p.tensions.is_empty() {
        out.push_str("\nTension\n");
        for tension in p.tensions.iter().take(2) {
            out.push_str(&format!("  - {tension}\n"));
        }
    }
    out.push_str(
        "\nFOMO checkpoint\n  - Am I reacting to evidence, or opportunity-cost fear?\n  - What single observation would make this story wrong today?\n  - What would I write in `mp think` if I had to defend the claim later?\n\nBoundary\n  Reasoning support only; no trading instructions.\n",
    );
    if !p.notes.is_empty() {
        out.push_str("\nSource notes\n");
        for n in &p.notes {
            out.push_str(&format!("  - {n}\n"));
        }
    }
    out
}

fn build_fomo_checkpoint() -> FomoCheckpoint {
    let radar = latest_radar_event();
    FomoCheckpoint {
        timestamp: timestamp(),
        linked_pulse: radar
            .as_ref()
            .and_then(|line| json_field(line, "linked_pulse_timestamp"))
            .or_else(latest_pulse_timestamp),
        linked_radar: radar
            .as_ref()
            .and_then(|line| json_field(line, "timestamp"))
            .or_else(latest_radar_timestamp),
        scenario: radar.as_ref().and_then(|line| json_field(line, "scenario")),
        confirm: radar.as_ref().and_then(|line| json_field(line, "confirm")),
        falsify: radar.as_ref().and_then(|line| json_field(line, "falsify")),
        watch: radar.as_ref().and_then(|line| json_field(line, "watch")),
        prompt: FOMO_JOURNAL_PROMPT.into(),
    }
}

fn render_fomo_checkpoint(checkpoint: &FomoCheckpoint) -> String {
    let latest_radar = checkpoint
        .linked_radar
        .as_deref()
        .unwrap_or("not recorded yet");
    let latest_pulse = checkpoint
        .linked_pulse
        .as_deref()
        .unwrap_or("not recorded yet");
    let scenario = checkpoint
        .scenario
        .as_deref()
        .unwrap_or("no radar scenario yet; run `mp watch` for a fresh context card");

    let mut out = format!(
        "FOMO Checkpoint · {}\n\nContext\n  Latest radar: {}\n  Latest pulse: {}\n  Scenario: {}\n\nPause\n  1. What exactly am I reacting to?\n  2. Which evidence confirms the scenario?\n  3. Which observation would falsify it today?\n  4. Is this evidence, or opportunity-cost fear?\n",
        checkpoint.timestamp, latest_radar, latest_pulse, scenario
    );
    if checkpoint.confirm.is_some() || checkpoint.falsify.is_some() || checkpoint.watch.is_some() {
        out.push_str("\nCarry-over checks\n");
        if let Some(watch) = checkpoint.watch.as_deref() {
            out.push_str(&format!("  Watch: {watch}\n"));
        }
        if let Some(confirm) = checkpoint.confirm.as_deref() {
            out.push_str(&format!("  Confirm: {confirm}\n"));
        }
        if let Some(falsify) = checkpoint.falsify.as_deref() {
            out.push_str(&format!("  Falsify: {falsify}\n"));
        }
    }
    out.push_str(&format!(
        "\nNext journal prompt\n  {}\n\nBoundary\n  Reasoning support only; no trading instructions.\n",
        checkpoint.prompt
    ));
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

fn render_week(w: &Weekly, events: &[String]) -> String {
    let summary = summarize_events(events);
    let mut out = format!(
        "Weekly Market Pulse · {}\n\nWeek story\n  {}\n\nBasis\n",
        w.timestamp, w.label
    );
    for b in &w.basis {
        out.push_str(&format!("  - {b}\n"));
    }
    out.push_str("\n1W Asset Map\n");
    for a in &w.assets {
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
    out.push_str("\nWeekly market themes\n");
    for (i, d) in w.drivers.iter().enumerate() {
        out.push_str(&format!("  {}. {}\n", i + 1, d));
    }
    out.push_str("\nWeekly tensions\n");
    for t in &w.tensions {
        out.push_str(&format!("  - {t}\n"));
    }
    out.push_str(&format!(
        "\nThis week's learning loop\n  Entries scanned: {} · pulses {} · regimes {} · inquiries {} · research {} · thoughts {} · feedback {}\n",
        events.len(),
        summary.pulses,
        summary.regimes,
        summary.inquiries,
        summary.research_inquiries,
        summary.thoughts,
        summary.feedback
    ));
    out.push_str("\nRecurring journal themes\n");
    let mut wrote_themes = false;
    for (tag, count) in summary
        .tag_counts
        .iter()
        .filter(|(_, count)| *count > 0)
        .take(5)
    {
        wrote_themes = true;
        out.push_str(&format!("  - {tag}: {count}\n"));
    }
    if !wrote_themes {
        out.push_str("  - Not enough tagged questions/thoughts this week yet\n");
    }
    out.push_str("\nQuestion / thesis habits\n");
    if summary.thesis_types.is_empty() {
        out.push_str("  - Ask or think at least once this week to build a visible pattern.\n");
    } else {
        for thesis in summary.thesis_types.iter().take(4) {
            out.push_str(&format!("  - You used a {thesis} lens.\n"));
        }
    }
    out.push_str("\nNext week check questions\n");
    for q in &w.questions {
        out.push_str(&format!("  - {q}\n"));
    }
    out.push_str("\nWeekly drill\n  Pick one repeated theme above and write one falsifiable `mp think` note before the next `mp-week`.\n\nBoundary\n  Market literacy only; not investment advice, buy/sell guidance, price targets, stop-loss, or portfolio instructions.\n");
    if !w.notes.is_empty() {
        out.push_str("\nSource notes\n");
        for n in &w.notes {
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

fn render_earnings(bundle: &EarningsBundle) -> String {
    let mut out = format!(
        "Earnings Pulse · {} · provider: {}\n\n",
        bundle.timestamp, bundle.provider
    );
    out.push_str("Recent results / source-backed hints\n");
    render_earnings_hints(&mut out, &bundle.recent);
    out.push_str("\nUpcoming radar / source-backed hints\n");
    render_earnings_hints(&mut out, &bundle.upcoming);
    out.push_str("\nEvidence checks\n");
    out.push_str("  - Are revisions, guidance tone, and post-earnings reactions pointing in the same direction?\n");
    out.push_str(
        "  - Are semis/Nasdaq reactions broad, or concentrated in a few mega-cap names?\n",
    );
    out.push_str(
        "  - Which upcoming reports could falsify the current growth/earnings narrative?\n",
    );
    out.push_str("\nCounter-view\n");
    out.push_str("  - A source headline or one earnings beat may reflect positioning, guidance nuance, or single-name concentration rather than broad risk-on.\n");
    if !bundle.notes.is_empty() {
        out.push_str("\nSource notes\n");
        for note in &bundle.notes {
            out.push_str(&format!("  - {note}\n"));
        }
    }
    out.push_str("\nBoundary\n  Reasoning support only; not investment advice, buy/sell guidance, price targets, stop-loss, or portfolio instructions. Experimental and incomplete: not an official filings or earnings database replacement.\n");
    out
}

fn render_earnings_hints(out: &mut String, hints: &[EarningsHint]) {
    if hints.is_empty() {
        out.push_str("  - No source-backed earnings hints available; configure MARKET_PULSE_SEARCH_CMD or check official company/filing sources.\n");
        return;
    }
    for (idx, hint) in hints.iter().enumerate() {
        let source = &hint.source;
        let published = source
            .published_at
            .as_deref()
            .unwrap_or("published time unavailable");
        out.push_str(&format!(
            "  {}. {} — {} — {} — freshness: {}\n",
            idx + 1,
            source.title,
            source.publisher,
            published,
            hint.freshness
        ));
        out.push_str(&format!("     URL: {}\n", empty_as_unknown(&source.url)));
        if let Some(identity) = earnings_identity(&hint.fields) {
            out.push_str(&format!("     Company: {identity}\n"));
        }
        out.push_str(&format!("     Evidence: {}\n", source.evidence));
        out.push_str(&format!("     Relevance: {}\n", source.relevance));
        out.push_str(&format!(
            "     Report: date={} timing={} price_reaction={}\n",
            opt_or_unknown(hint.fields.report_date.as_deref()),
            opt_or_unknown(hint.fields.timing.as_deref()),
            opt_or_unknown(hint.fields.price_reaction.as_deref())
        ));
        out.push_str(&format!(
            "     EPS: actual={} estimate={} surprise={}\n",
            opt_or_unknown(hint.fields.eps_actual.as_deref()),
            opt_or_unknown(hint.fields.eps_estimate.as_deref()),
            opt_or_unknown(hint.fields.surprise.as_deref())
        ));
        out.push_str(&format!(
            "     Revenue: actual={} estimate={}\n",
            opt_or_unknown(hint.fields.revenue_actual.as_deref()),
            opt_or_unknown(hint.fields.revenue_estimate.as_deref())
        ));
        out.push_str(&format!(
            "     Guidance: {}\n",
            opt_or_unknown(hint.fields.guidance.as_deref())
        ));
    }
}

fn earnings_identity(fields: &EarningsFields) -> Option<String> {
    match (fields.ticker.as_deref(), fields.company.as_deref()) {
        (Some(ticker), Some(company)) => Some(format!("{ticker} · {company}")),
        (Some(ticker), None) => Some(ticker.to_string()),
        (None, Some(company)) => Some(company.to_string()),
        (None, None) => None,
    }
}

fn opt_or_unknown(value: Option<&str>) -> &str {
    value.filter(|v| !v.trim().is_empty()).unwrap_or("unknown")
}

fn empty_as_unknown(value: &str) -> &str {
    if value.trim().is_empty() {
        "unknown"
    } else {
        value
    }
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
    let source_backed = !bundle.sources.is_empty();
    let heading = if source_backed {
        "Source-backed Research Inquiry"
    } else {
        "Research unavailable"
    };
    let mut out = format!(
        "{heading} · {} · provider: {}\n\nQuestion breakdown\n",
        i.timestamp, bundle.provider
    );
    for x in &i.breakdown {
        out.push_str(&format!("  - {x}\n"));
    }
    out.push_str("\nSources checked\n");
    if bundle.sources.is_empty() {
        out.push_str("  - Source-backed research is unavailable: no configured provider returned source metadata.\n");
        out.push_str("  - Configure MARKET_PULSE_SEARCH_CMD with a JSONL source bridge for source-backed research.\n");
        out.push_str(
            "  - Treat the analysis below as inference scaffolding only, not source-backed fact.\n",
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
        out.push_str("  - Source-backed research is unavailable in this run; use the inquiry lens below to decide what evidence to fetch next.\n");
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CalendarCoverage {
    Full,
    Partial,
    SourceLimited,
    Unavailable,
}

impl CalendarCoverage {
    fn label(self) -> &'static str {
        match self {
            CalendarCoverage::Full => "full curated static coverage",
            CalendarCoverage::Partial => "partial curated static coverage",
            CalendarCoverage::SourceLimited => "source-limited coverage",
            CalendarCoverage::Unavailable => "coverage unavailable/stale",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ExchangeGroup {
    UsEquities,
    KoreaEquities,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SourceAgreement {
    Agree,
    NyseOnly,
    NasdaqOnly,
    BothMissing,
    Disagree,
}

const US_SOURCE_AGREEMENT_MATRIX: &[SourceAgreement] = &[
    SourceAgreement::Agree,
    SourceAgreement::NyseOnly,
    SourceAgreement::NasdaqOnly,
    SourceAgreement::BothMissing,
    SourceAgreement::Disagree,
];

fn source_agreement_label(agreement: SourceAgreement) -> &'static str {
    match agreement {
        SourceAgreement::Agree => "NYSE+Nasdaq agree",
        SourceAgreement::NyseOnly => "NYSE only",
        SourceAgreement::NasdaqOnly => "Nasdaq only",
        SourceAgreement::BothMissing => "both missing",
        SourceAgreement::Disagree => "sources disagree",
    }
}

fn source_matrix_summary() -> String {
    US_SOURCE_AGREEMENT_MATRIX
        .iter()
        .map(|agreement| source_agreement_label(*agreement))
        .collect::<Vec<_>>()
        .join(" / ")
}

#[derive(Clone, Copy, Debug)]
struct ClosureRule {
    date: &'static str,
    reason: &'static str,
}

#[derive(Clone, Copy, Debug)]
struct EarlyCloseRule {
    date: &'static str,
    close_minutes: u16,
    close_label: &'static str,
    reason: &'static str,
}

#[derive(Clone, Copy, Debug)]
struct ExchangeCalendar {
    group: ExchangeGroup,
    label: &'static str,
    exchange_tz: &'static str,
    regular_hours: &'static str,
    regular_open_minutes: u16,
    regular_close_minutes: u16,
    coverage_years: &'static [i32],
    last_curated: &'static str,
    source_labels: &'static [&'static str],
    closures: &'static [ClosureRule],
    early_closes: &'static [EarlyCloseRule],
    default_coverage: CalendarCoverage,
    coverage_note: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ExchangeDateTime {
    date: String,
    time: String,
    weekday: u32,
    zone: String,
}

impl ExchangeDateTime {
    fn minutes(&self) -> Option<u16> {
        parse_hhmm_minutes(&self.time)
    }
}

#[derive(Clone, Debug)]
struct ExchangeSessionRow {
    label: &'static str,
    exchange_local: String,
    regular_hours: &'static str,
    coverage: CalendarCoverage,
    status: String,
    note: &'static str,
}

const US_COVERAGE_YEARS: &[i32] = &[2026, 2027];
const KR_COVERAGE_YEARS: &[i32] = &[2026, 2027];

const US_SOURCES: &[&str] = &[
    "Nasdaq 2026 holiday schedule",
    "NYSE Group 2026-2028 holiday/early-close release",
];
const KRX_SOURCES: &[&str] = &["KRX trading guide", "KRX Market Closing(Holiday) page"];

const US_CLOSURES: &[ClosureRule] = &[
    ClosureRule {
        date: "2026-01-01",
        reason: "New Year's Day",
    },
    ClosureRule {
        date: "2026-01-19",
        reason: "Martin Luther King Jr. Day",
    },
    ClosureRule {
        date: "2026-02-16",
        reason: "Washington's Birthday / Presidents Day",
    },
    ClosureRule {
        date: "2026-04-03",
        reason: "Good Friday",
    },
    ClosureRule {
        date: "2026-05-25",
        reason: "Memorial Day",
    },
    ClosureRule {
        date: "2026-06-19",
        reason: "Juneteenth National Independence Day",
    },
    ClosureRule {
        date: "2026-07-03",
        reason: "Independence Day observed",
    },
    ClosureRule {
        date: "2026-09-07",
        reason: "Labor Day",
    },
    ClosureRule {
        date: "2026-11-26",
        reason: "Thanksgiving Day",
    },
    ClosureRule {
        date: "2026-12-25",
        reason: "Christmas Day",
    },
    // NYSE publishes 2027 dates, but Nasdaq grouped-row proof is source-limited until
    // the Nasdaq source also provides matching 2027 coverage.
    ClosureRule {
        date: "2027-01-01",
        reason: "New Year's Day (NYSE source; grouped proof source-limited)",
    },
];

const US_EARLY_CLOSES: &[EarlyCloseRule] = &[
    EarlyCloseRule {
        date: "2026-11-27",
        close_minutes: 13 * 60,
        close_label: "13:00 ET",
        reason: "Day after Thanksgiving",
    },
    EarlyCloseRule {
        date: "2026-12-24",
        close_minutes: 13 * 60,
        close_label: "13:00 ET",
        reason: "Christmas Eve",
    },
];

const KRX_CLOSURES: &[ClosureRule] = &[
    ClosureRule {
        date: "2026-05-01",
        reason: "Labor Day",
    },
    ClosureRule {
        date: "2026-12-31",
        reason: "KRX year-end closure (structural source)",
    },
    ClosureRule {
        date: "2027-05-01",
        reason: "Labor Day",
    },
    ClosureRule {
        date: "2027-12-31",
        reason: "KRX year-end closure (structural source)",
    },
];

const NO_EARLY_CLOSES: &[EarlyCloseRule] = &[];

const US_EQUITIES_CALENDAR: ExchangeCalendar = ExchangeCalendar {
    group: ExchangeGroup::UsEquities,
    label: "US equities (NYSE/Nasdaq)",
    exchange_tz: "America/New_York",
    regular_hours: "09:30-16:00 ET",
    regular_open_minutes: 9 * 60 + 30,
    regular_close_minutes: 16 * 60,
    coverage_years: US_COVERAGE_YEARS,
    last_curated: "2026-04-23",
    source_labels: US_SOURCES,
    closures: US_CLOSURES,
    early_closes: US_EARLY_CLOSES,
    default_coverage: CalendarCoverage::Full,
    coverage_note: "Grouped NYSE/Nasdaq row is full only when both sources cover and agree; 2027 remains source-limited if Nasdaq coverage is absent.",
};

const KRX_EQUITIES_CALENDAR: ExchangeCalendar = ExchangeCalendar {
    group: ExchangeGroup::KoreaEquities,
    label: "Korea equities (KRX/KOSPI)",
    exchange_tz: "Asia/Seoul",
    regular_hours: "09:00-15:30 KST",
    regular_open_minutes: 9 * 60,
    regular_close_minutes: 15 * 60 + 30,
    coverage_years: KR_COVERAGE_YEARS,
    last_curated: "2026-04-23",
    source_labels: KRX_SOURCES,
    closures: KRX_CLOSURES,
    early_closes: NO_EARLY_CLOSES,
    default_coverage: CalendarCoverage::Partial,
    coverage_note: "KRX regular hours and structural closures are curated, but full year-specific Korean public-holiday coverage is partial.",
};

fn render_calendar() -> String {
    let today = date_for_days_ago(0).unwrap_or_else(|_| "unavailable".into());
    let yesterday = date_for_days_ago(1).unwrap_or_else(|_| "unavailable".into());
    let this_week = current_week_date_prefixes();
    let last_week = last_week_date_prefixes();
    let rows = exchange_session_rows_with_clock(&SystemExchangeClock);
    let mut out = format!(
        "Market Pulse Calendar · {}\n\nLocal date windows\n",
        timestamp()
    );
    out.push_str(&format!("  - today: {today}\n"));
    out.push_str(&format!("  - yesterday: {yesterday}\n"));
    out.push_str(&format!(
        "  - this-week: {}\n",
        week_window_label(&this_week)
    ));
    out.push_str(&format!(
        "  - last-week: {}\n",
        week_window_label(&last_week)
    ));
    out.push_str("\nExchange sessions (curated static rules)\n");
    for row in rows {
        out.push_str(&format!(
            "  - {}: {}; regular {}; {}; coverage {}\n",
            row.label,
            row.exchange_local,
            row.regular_hours,
            row.status,
            row.coverage.label()
        ));
        out.push_str(&format!("    note: {}\n", row.note));
    }
    out.push_str("\nSource / freshness\n");
    for calendar in [&US_EQUITIES_CALENDAR, &KRX_EQUITIES_CALENDAR] {
        out.push_str(&format!(
            "  - {}: years {}; last curated {}; sources: {}; {}{}\n",
            calendar.label,
            year_list(calendar.coverage_years),
            calendar.last_curated,
            calendar.source_labels.join(" + "),
            calendar.coverage_note,
            if matches!(calendar.group, ExchangeGroup::UsEquities) {
                format!(" matrix: {}.", source_matrix_summary())
            } else {
                String::new()
            }
        ));
    }
    out.push_str(
        "\nCalendar ↔ pulse bridge\n  - mp now: close-to-close daily pulse; read it against latest available exchange closes, not only the local date.\n  - mp week: local journal week plus first matching Yahoo daily close in the current local week.\n  - US/Korea session dates can differ from the Korea local timestamp; this card gives interpretation context, not official live exchange proof.\n",
    );
    out.push_str(
        "\nReview shortcuts\n  - mp review --today\n  - mp review --yesterday\n  - mp review --this-week\n  - mp review --last-week\n",
    );
    out.push_str(
        "\nHow market-pulse uses these windows\n  - mp week uses the current local calendar week for journal review.\n  - mp week prices the market window from the first Yahoo close matching the current local week when available.\n  - mp review period aliases filter journal timestamp dates; they are not fuzzy full-text search.\n",
    );
    out.push_str(
        "\nBoundary\n  Deterministic curated static exchange-calendar context for market literacy; not a live official exchange feed, live event/news calendar, or trading signal.\n",
    );
    out
}

trait ExchangeClock {
    fn now_in(&self, tz: &str) -> Option<ExchangeDateTime>;
}

struct SystemExchangeClock;

impl ExchangeClock for SystemExchangeClock {
    fn now_in(&self, tz: &str) -> Option<ExchangeDateTime> {
        exchange_datetime_from_system_date(tz)
    }
}

fn exchange_datetime_from_system_date(tz: &str) -> Option<ExchangeDateTime> {
    let output = Command::new("date")
        .env("TZ", tz)
        .arg("+%Y-%m-%d %H:%M %u %Z")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    parse_exchange_datetime(&String::from_utf8(output.stdout).ok()?)
}

fn parse_exchange_datetime(raw: &str) -> Option<ExchangeDateTime> {
    let parts = raw.split_whitespace().collect::<Vec<_>>();
    if parts.len() < 4 {
        return None;
    }
    validate_review_date(parts[0]).ok()?;
    let weekday = parts[2]
        .parse::<u32>()
        .ok()
        .filter(|day| (1..=7).contains(day))?;
    parse_hhmm_minutes(parts[1])?;
    Some(ExchangeDateTime {
        date: parts[0].to_string(),
        time: parts[1].to_string(),
        weekday,
        zone: parts[3].to_string(),
    })
}

fn parse_hhmm_minutes(value: &str) -> Option<u16> {
    let (hour, minute) = value.split_once(':')?;
    let hour = hour.parse::<u16>().ok()?;
    let minute = minute.parse::<u16>().ok()?;
    if hour <= 23 && minute <= 59 {
        Some(hour * 60 + minute)
    } else {
        None
    }
}

fn exchange_session_rows_with_clock(clock: &dyn ExchangeClock) -> [ExchangeSessionRow; 2] {
    [
        exchange_session_row(
            &US_EQUITIES_CALENDAR,
            clock.now_in(US_EQUITIES_CALENDAR.exchange_tz),
        ),
        exchange_session_row(
            &KRX_EQUITIES_CALENDAR,
            clock.now_in(KRX_EQUITIES_CALENDAR.exchange_tz),
        ),
    ]
}

fn exchange_session_row(
    calendar: &'static ExchangeCalendar,
    exchange_now: Option<ExchangeDateTime>,
) -> ExchangeSessionRow {
    let coverage = coverage_for_exchange_date(calendar, exchange_now.as_ref());
    let exchange_local = exchange_now
        .as_ref()
        .map(|dt| format!("exchange-local {} {} {}", dt.date, dt.time, dt.zone))
        .unwrap_or_else(|| format!("exchange-local time unavailable ({})", calendar.exchange_tz));
    ExchangeSessionRow {
        label: calendar.label,
        exchange_local,
        regular_hours: calendar.regular_hours,
        coverage,
        status: session_status(calendar, exchange_now.as_ref()),
        note: calendar.coverage_note,
    }
}

fn coverage_for_exchange_date(
    calendar: &ExchangeCalendar,
    exchange_now: Option<&ExchangeDateTime>,
) -> CalendarCoverage {
    let Some(dt) = exchange_now else {
        return CalendarCoverage::Unavailable;
    };
    let Some(year) = year_from_date(&dt.date) else {
        return CalendarCoverage::Unavailable;
    };
    if !calendar.coverage_years.contains(&year) {
        return CalendarCoverage::Unavailable;
    }
    if matches!(calendar.group, ExchangeGroup::UsEquities) {
        return grouped_us_coverage_from_agreement(us_source_agreement_for_year(year));
    }
    calendar.default_coverage
}

fn grouped_us_coverage_from_agreement(agreement: SourceAgreement) -> CalendarCoverage {
    match agreement {
        SourceAgreement::Agree => CalendarCoverage::Full,
        SourceAgreement::NyseOnly | SourceAgreement::NasdaqOnly | SourceAgreement::Disagree => {
            CalendarCoverage::SourceLimited
        }
        SourceAgreement::BothMissing => CalendarCoverage::Unavailable,
    }
}

fn us_source_agreement_for_year(year: i32) -> SourceAgreement {
    match year {
        2026 => SourceAgreement::Agree,
        2027 => SourceAgreement::NyseOnly,
        _ => SourceAgreement::BothMissing,
    }
}

fn session_status(calendar: &ExchangeCalendar, exchange_now: Option<&ExchangeDateTime>) -> String {
    let Some(dt) = exchange_now else {
        return "status unavailable: exchange timestamp unavailable".into();
    };
    if dt.weekday >= 6 {
        return "closed: weekend".into();
    }
    let coverage = coverage_for_exchange_date(calendar, Some(dt));
    if coverage == CalendarCoverage::Unavailable {
        return format!(
            "status unavailable: {} outside curated coverage {}",
            dt.date,
            year_list(calendar.coverage_years)
        );
    }
    if coverage == CalendarCoverage::SourceLimited {
        return format!(
            "source-limited: grouped source coverage is incomplete for {}; not full official open proof",
            dt.date
        );
    }
    if let Some(rule) = closure_for_date(calendar, &dt.date) {
        return format!("closed: holiday ({})", rule.reason);
    }
    if let Some(rule) = early_close_for_date(calendar, &dt.date) {
        return early_close_status(calendar, dt, rule);
    }
    regular_session_status(calendar, dt, coverage)
}

fn regular_session_status(
    calendar: &ExchangeCalendar,
    dt: &ExchangeDateTime,
    coverage: CalendarCoverage,
) -> String {
    let Some(minutes) = dt.minutes() else {
        return "status unavailable: exchange timestamp parse failed".into();
    };
    let qualifier = match coverage {
        CalendarCoverage::Full => "under curated static calendar rules",
        CalendarCoverage::Partial => "by partial KRX rules",
        CalendarCoverage::SourceLimited => "with source-limited coverage",
        CalendarCoverage::Unavailable => "with unavailable coverage",
    };
    if minutes < calendar.regular_open_minutes {
        return format!("before regular session {qualifier}");
    }
    if minutes >= calendar.regular_close_minutes {
        return format!("after regular session {qualifier}");
    }
    match coverage {
        CalendarCoverage::Partial => format!("regular session {qualifier}"),
        _ => format!("open {qualifier}"),
    }
}

fn early_close_status(
    calendar: &ExchangeCalendar,
    dt: &ExchangeDateTime,
    rule: &EarlyCloseRule,
) -> String {
    let Some(minutes) = dt.minutes() else {
        return "status unavailable: exchange timestamp parse failed".into();
    };
    if minutes < calendar.regular_open_minutes {
        format!(
            "before regular session; early close today: closes {} ({})",
            rule.close_label, rule.reason
        )
    } else if minutes < rule.close_minutes {
        format!(
            "open under curated static calendar rules; early close today: closes {} ({})",
            rule.close_label, rule.reason
        )
    } else {
        format!(
            "after early close: closed after {} ({})",
            rule.close_label, rule.reason
        )
    }
}

fn closure_for_date<'a>(calendar: &'a ExchangeCalendar, date: &str) -> Option<&'a ClosureRule> {
    calendar.closures.iter().find(|rule| rule.date == date)
}

fn early_close_for_date<'a>(
    calendar: &'a ExchangeCalendar,
    date: &str,
) -> Option<&'a EarlyCloseRule> {
    calendar.early_closes.iter().find(|rule| rule.date == date)
}

fn year_from_date(date: &str) -> Option<i32> {
    validate_review_date(date).ok()?;
    date[0..4].parse().ok()
}

fn year_list(years: &[i32]) -> String {
    years
        .iter()
        .map(|year| year.to_string())
        .collect::<Vec<_>>()
        .join("/")
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

fn read_week_events(limit: usize) -> Vec<String> {
    let Ok(text) = fs::read_to_string(journal_path()) else {
        return Vec::new();
    };
    let events = read_event_lines(&text);
    let dates = current_week_date_prefixes();
    if dates.is_empty() {
        return limit_events(events, limit);
    }
    filter_events_by_dates(events, &dates, limit)
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

fn read_events_for_dates(limit: usize, dates: &[String]) -> Vec<String> {
    let Ok(text) = fs::read_to_string(journal_path()) else {
        return Vec::new();
    };
    filter_events_by_dates(read_event_lines(&text), dates, limit)
}

fn read_events_for_filter(limit: usize, filter: Option<&ReviewFilter>) -> Vec<String> {
    match filter {
        Some(ReviewFilter::Date(date)) => read_events_for_date(limit, date),
        Some(ReviewFilter::Dates { dates, .. }) => read_events_for_dates(limit, dates),
        None => read_events(limit),
    }
}

fn filter_events_by_date(events: Vec<String>, date: &str, limit: usize) -> Vec<String> {
    let lines = events
        .into_iter()
        .filter(|l| event_matches_date(l, date))
        .collect::<Vec<_>>();
    limit_events(lines, limit)
}

fn filter_events_by_dates(events: Vec<String>, dates: &[String], limit: usize) -> Vec<String> {
    let lines = events
        .into_iter()
        .filter(|l| {
            json_field(l, "timestamp")
                .is_some_and(|ts| dates.iter().any(|date| ts.starts_with(date)))
        })
        .collect::<Vec<_>>();
    limit_events(lines, limit)
}

fn date_prefixes_for_days(days: u32) -> Vec<String> {
    (0..days)
        .filter_map(|days_ago| date_for_days_ago(days_ago).ok())
        .collect()
}

fn event_matches_date(line: &str, date: &str) -> bool {
    json_field(line, "timestamp").is_some_and(|ts| ts.starts_with(date))
}

fn latest_pulse_timestamp() -> Option<String> {
    latest_event_timestamp("pulse")
}

fn latest_radar_timestamp() -> Option<String> {
    latest_event_timestamp("radar")
}

fn latest_event_timestamp(event_type: &str) -> Option<String> {
    latest_event(event_type).and_then(|l| json_field(&l, "timestamp"))
}

fn latest_radar_event() -> Option<String> {
    latest_event("radar")
}

fn latest_event(event_type: &str) -> Option<String> {
    let needle = format!("\"type\":\"{event_type}\"");
    read_events(usize::MAX)
        .into_iter()
        .rev()
        .find(|l| l.contains(&needle))
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

fn render_review_for_filter(limit: usize, filter: &ReviewFilter) -> String {
    match filter {
        ReviewFilter::Date(date) => render_review_for_date(limit, date),
        ReviewFilter::Dates { label, dates } => render_review_for_dates(limit, label, dates),
    }
}

fn render_review_for_date(limit: usize, date: &str) -> String {
    let events = read_events_for_date(limit, date);
    render_review_for_date_from_events(&events, &journal_path().display().to_string(), date)
}

fn render_review_for_dates(limit: usize, label: &str, dates: &[String]) -> String {
    let events = read_events_for_dates(limit, dates);
    render_review_for_dates_from_events(&events, &journal_path().display().to_string(), label)
}

fn render_find(limit: usize, query: &str, filter: Option<&ReviewFilter>) -> String {
    let events = find_events(limit, query, filter);
    render_find_from_events(
        &events,
        &journal_path().display().to_string(),
        query,
        filter_label(filter),
    )
}

fn find_events(limit: usize, query: &str, filter: Option<&ReviewFilter>) -> Vec<String> {
    let query = query.to_lowercase();
    read_events_for_filter(usize::MAX, filter)
        .into_iter()
        .filter(|line| line.to_lowercase().contains(&query))
        .rev()
        .take(limit)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect()
}

fn filter_label(filter: Option<&ReviewFilter>) -> String {
    match filter {
        Some(ReviewFilter::Date(date)) => format!("date {date}"),
        Some(ReviewFilter::Dates { label, .. }) => label.clone(),
        None => "all journal entries".into(),
    }
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

fn render_review_for_dates_from_events(events: &[String], journal: &str, label: &str) -> String {
    if events.is_empty() {
        return format!(
            "No market-pulse journal entries found for {label}.\n\nJournal: {journal}\nTry `mp calendar` to inspect available date windows, or record one with `mp now`, `mp week`, `mp ask`, or `mp think`."
        );
    }
    let mut out = format!("Review period filter: {label}\n\n");
    out.push_str(&render_review_from_events(events, journal));
    out
}

fn render_find_from_events(
    events: &[String],
    journal: &str,
    query: &str,
    filter: String,
) -> String {
    if events.is_empty() {
        return format!(
            "No market-pulse journal entries matched \"{query}\".\n\nJournal: {journal}\nFilter: {filter}\nTry a simpler keyword, `mp calendar` for date windows, or record one with `mp ask`, `mp research`, or `mp think`."
        );
    }

    let summary = summarize_events(events);
    let mut out = format!(
        "Market Pulse Find\n\nQuery: \"{query}\"\nFilter: {filter}\nJournal: {journal}\nEntries matched: {} · pulses {} · radars {} · fomo {} · weeks {} · regimes {} · inquiries {} · research {} · thoughts {} · feedback {}\n\nMatches\n",
        events.len(),
        summary.pulses,
        summary.radars,
        summary.fomo_checks,
        summary.weeks,
        summary.regimes,
        summary.inquiries,
        summary.research_inquiries,
        summary.thoughts,
        summary.feedback
    );
    for line in events.iter().rev().take(12) {
        out.push_str(&format!("  - {}\n", event_recall_snippet(line, query)));
    }

    out.push_str("\nRecurring themes in matches\n");
    let mut wrote_theme = false;
    for (tag, count) in summary
        .tag_counts
        .iter()
        .filter(|(_, count)| *count > 0)
        .take(4)
    {
        wrote_theme = true;
        out.push_str(&format!("  - {tag}: {count}\n"));
    }
    if !wrote_theme {
        out.push_str("  - Not enough tagged matching entries yet\n");
    }
    let mut recall_tags = detect_tags(query);
    for tag in tags_from_summary(&summary, 2) {
        push_tag(&mut recall_tags, tag);
    }
    let question = recall_question(query, &filter, &recall_tags);
    out.push_str(&format!(
        "\nNext recall question\n  {question}\n\nBoundary\n  `mp find` searches your local journal only. It is recall support for market literacy, not live research or trading advice."
    ));
    out
}

fn event_recall_snippet(line: &str, query: &str) -> String {
    let timestamp = json_field(line, "timestamp").unwrap_or_else(|| "unknown-time".into());
    let event_type = json_field(line, "type").unwrap_or_else(|| "entry".into());
    let body = json_field(line, "text")
        .or_else(|| json_field(line, "question"))
        .or_else(|| json_field(line, "scenario"))
        .or_else(|| json_field(line, "prompt"))
        .or_else(|| json_field(line, "confirm"))
        .or_else(|| json_field(line, "falsify"))
        .or_else(|| json_field(line, "mood"))
        .or_else(|| json_field(line, "concept"))
        .or_else(|| json_field(line, "source_titles"))
        .unwrap_or_else(|| compact_raw_event(line));
    format!(
        "{timestamp} · {event_type} · {}",
        compact_snippet(&body, query)
    )
}

fn compact_raw_event(line: &str) -> String {
    compact_snippet(line, "")
}

fn compact_snippet(text: &str, _query: &str) -> String {
    let cleaned = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let max_chars = 140usize;
    if cleaned.chars().count() <= max_chars {
        return cleaned;
    }
    let snippet = cleaned.chars().take(max_chars).collect::<String>();
    format!("{snippet}...")
}

#[derive(Clone, Debug)]
struct JournalSummary {
    pulses: usize,
    radars: usize,
    fomo_checks: usize,
    weeks: usize,
    regimes: usize,
    inquiries: usize,
    research_inquiries: usize,
    thoughts: usize,
    feedback: usize,
    tag_counts: Vec<(&'static str, usize)>,
    thesis_types: Vec<String>,
    korean_entries: usize,
}

fn summarize_events(events: &[String]) -> JournalSummary {
    let pulses = count_events(events, "pulse");
    let radars = count_events(events, "radar");
    let fomo_checks = count_events(events, "fomo_check");
    let weeks = count_events(events, "week");
    let regimes = count_events(events, "regime");
    let inquiries = count_events(events, "inquiry");
    let research_inquiries = count_events(events, "research_inquiry");
    let thoughts = count_events(events, "thought");
    let feedback = count_events(events, "feedback");
    let mut tag_counts: Vec<(&str, usize)> = vec![
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
    let mut korean_entries = 0usize;
    for line in events.iter().filter(|l| {
        l.contains("\"type\":\"thought\"")
            || l.contains("\"type\":\"inquiry\"")
            || l.contains("\"type\":\"research_inquiry\"")
            || l.contains("\"type\":\"radar\"")
            || l.contains("\"type\":\"fomo_check\"")
    }) {
        let text = json_field(line, "text")
            .or_else(|| json_field(line, "question"))
            .or_else(|| json_field(line, "scenario"))
            .or_else(|| json_field(line, "prompt"))
            .unwrap_or_default();
        if contains_hangul(&text) {
            korean_entries += 1;
        }
        let tags = detect_tags(&text);
        if !tags.is_empty() {
            thesis_types.push(detect_thesis_type(&tags));
        }
        for tag in tags {
            if let Some((_, count)) = tag_counts.iter_mut().find(|(name, _)| *name == tag) {
                *count += 1;
            }
        }
    }
    tag_counts.sort_by(|a, b| b.1.cmp(&a.1));
    thesis_types.sort();
    thesis_types.dedup();
    JournalSummary {
        pulses,
        radars,
        fomo_checks,
        weeks,
        regimes,
        inquiries,
        research_inquiries,
        thoughts,
        feedback,
        tag_counts,
        thesis_types,
        korean_entries,
    }
}

fn count_events(events: &[String], event_type: &str) -> usize {
    let needle = format!("\"type\":\"{event_type}\"");
    events.iter().filter(|l| l.contains(&needle)).count()
}

fn render_review_from_events(events: &[String], journal: &str) -> String {
    if events.is_empty() {
        return "No market-pulse journal entries yet. Start with `mp \"your market question\"`, then `mp think \"...\"`.".into();
    }
    let summary = summarize_events(events);
    let mut out = format!("Market Pulse Review\n\nJournal: {journal}\nEntries scanned: {} · pulses {} · radars {} · fomo {} · weeks {} · regimes {} · inquiries {} · research {} · thoughts {} · feedback {}\n\nRepeated themes\n", events.len(), summary.pulses, summary.radars, summary.fomo_checks, summary.weeks, summary.regimes, summary.inquiries, summary.research_inquiries, summary.thoughts, summary.feedback);
    let mut wrote = false;
    for (tag, count) in summary.tag_counts.iter().filter(|(_, c)| *c > 0).take(6) {
        wrote = true;
        out.push_str(&format!("  - {tag}: {count}\n"));
    }
    if !wrote {
        out.push_str("  - Not enough tagged thoughts yet\n");
    }
    out.push_str("\nQuestion / thesis habits\n");
    if summary.thesis_types.is_empty() {
        out.push_str("  - Not enough inquiry/thesis history yet; ask one rough question with `mp \"...\"`.\n");
    } else {
        for t in summary.thesis_types.iter().take(5) {
            out.push_str(&format!("  - You have been using a {t} lens.\n"));
        }
    }
    out.push_str(&review_drill(&summary));
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

fn radar_json(p: &Pulse, checklist: &DailyDecisionChecklist) -> String {
    format!(
        "{{\"type\":\"radar\",\"timestamp\":\"{}\",\"session\":\"{}\",\"linked_pulse_timestamp\":\"{}\",\"mood\":\"{}\",\"scenario\":\"{}\",\"confirm\":\"{}\",\"falsify\":\"{}\",\"watch\":\"{}\",\"prompt\":\"{}\"}}",
        esc(&p.timestamp),
        esc(&p.session),
        esc(&p.timestamp),
        esc(&p.mood),
        esc(checklist.scenario),
        esc(checklist.confirm),
        esc(checklist.falsify),
        esc(checklist.watch),
        esc(FOMO_JOURNAL_PROMPT)
    )
}

fn fomo_check_json(checkpoint: &FomoCheckpoint) -> String {
    format!(
        "{{\"type\":\"fomo_check\",\"timestamp\":\"{}\",\"linked_pulse_timestamp\":{},\"linked_radar_timestamp\":{},\"scenario\":\"{}\",\"prompt\":\"{}\"}}",
        esc(&checkpoint.timestamp),
        opt_json(checkpoint.linked_pulse.as_deref()),
        opt_json(checkpoint.linked_radar.as_deref()),
        esc(checkpoint.scenario.as_deref().unwrap_or("")),
        esc(&checkpoint.prompt)
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

fn week_json(w: &Weekly, journal_entries: usize) -> String {
    format!(
        "{{\"type\":\"week\",\"timestamp\":\"{}\",\"basis\":\"{}\",\"label\":\"{}\",\"journal_entries\":{},\"question\":\"{}\"}}",
        esc(&w.timestamp),
        esc(&w.basis.join(" | ")),
        esc(&w.label),
        journal_entries,
        esc(w.questions.first().map(String::as_str).unwrap_or(""))
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

    fn args(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|part| (*part).to_string()).collect()
    }

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
    fn routes_watch_and_fomo_commands() {
        assert_eq!(
            parse_command(&["watch".into(), "--no-save".into()]).unwrap(),
            CommandKind::Watch
        );
        assert_eq!(
            parse_command(&["fomo".into(), "--no-save".into()]).unwrap(),
            CommandKind::Fomo
        );
    }

    #[test]
    fn routes_week_to_week_command() {
        assert_eq!(
            parse_command(&["week".into(), "--no-save".into()]).unwrap(),
            CommandKind::Week
        );
        assert_eq!(
            parse_command(&["weekly".into(), "--no-save".into()]).unwrap(),
            CommandKind::Week
        );
    }

    #[test]
    fn routes_calendar_to_calendar_command() {
        assert_eq!(
            parse_command(&["calendar".into()]).unwrap(),
            CommandKind::Calendar
        );
        assert_eq!(
            parse_command(&["cal".into()]).unwrap(),
            CommandKind::Calendar
        );
    }

    #[test]
    fn routes_find_to_find_command() {
        assert_eq!(
            parse_command(&["find".into(), "금리".into()]).unwrap(),
            CommandKind::Find
        );
        assert_eq!(
            parse_command(&["search".into(), "달러".into()]).unwrap(),
            CommandKind::Find
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
    fn routes_korean_market_pulse_aliases_to_now() {
        assert_eq!(
            parse_command(&args(&["오늘", "시황"])).unwrap(),
            CommandKind::Now
        );
        assert_eq!(
            parse_command(&args(&["지금", "시장"])).unwrap(),
            CommandKind::Now
        );
        assert_eq!(
            parse_command(&args(&["시장", "펄스"])).unwrap(),
            CommandKind::Now
        );
    }

    #[test]
    fn routes_korean_period_and_regime_aliases() {
        assert_eq!(
            parse_command(&args(&["이번주"])).unwrap(),
            CommandKind::Week
        );
        assert_eq!(
            parse_command(&args(&["주간", "체크"])).unwrap(),
            CommandKind::Week
        );
        assert_eq!(
            parse_command(&args(&["레짐"])).unwrap(),
            CommandKind::Regime
        );
        assert_eq!(
            parse_command(&args(&["국면", "체크"])).unwrap(),
            CommandKind::Regime
        );
        assert_eq!(
            parse_command(&args(&["캘린더"])).unwrap(),
            CommandKind::Calendar
        );
    }

    #[test]
    fn routes_korean_review_aliases_with_date_windows() {
        assert_eq!(
            parse_command_args(&args(&["오늘", "복기"])).unwrap(),
            ParsedCommand {
                kind: CommandKind::Review,
                args: args(&["review", "--today"]),
            }
        );
        assert_eq!(
            parse_command_args(&args(&["어제", "복기"])).unwrap(),
            ParsedCommand {
                kind: CommandKind::Review,
                args: args(&["review", "--yesterday"]),
            }
        );
        assert_eq!(
            parse_command_args(&args(&["이번주", "복기"])).unwrap(),
            ParsedCommand {
                kind: CommandKind::Review,
                args: args(&["review", "--this-week"]),
            }
        );
        assert_eq!(
            parse_command_args(&args(&["지난주", "리뷰", "--limit", "3"])).unwrap(),
            ParsedCommand {
                kind: CommandKind::Review,
                args: args(&["review", "--last-week", "--limit", "3"]),
            }
        );
    }

    #[test]
    fn routes_korean_recall_aliases_to_find() {
        assert_eq!(
            parse_command_args(&args(&["전에", "금리", "찾아줘"])).unwrap(),
            ParsedCommand {
                kind: CommandKind::Find,
                args: args(&["find", "금리"]),
            }
        );
        assert_eq!(
            parse_command_args(&args(&["지난번", "반도체", "검색", "--limit", "3"])).unwrap(),
            ParsedCommand {
                kind: CommandKind::Find,
                args: args(&["find", "반도체", "--limit", "3"]),
            }
        );
    }

    #[test]
    fn routes_korean_thought_aliases_to_think() {
        assert_eq!(
            parse_command_args(&args(&["내", "생각", "나스닥은", "너무", "오른듯"])).unwrap(),
            ParsedCommand {
                kind: CommandKind::Think,
                args: args(&["think", "나스닥은", "너무", "오른듯"]),
            }
        );
        assert_eq!(
            parse_command_args(&args(&["메모", "달러가", "약한데", "코스피가", "강함"])).unwrap(),
            ParsedCommand {
                kind: CommandKind::Think,
                args: args(&["think", "달러가", "약한데", "코스피가", "강함"]),
            }
        );
    }

    #[test]
    fn routes_korean_research_intent_to_research() {
        assert_eq!(
            parse_command(&args(&["NVDA", "리서치"])).unwrap(),
            CommandKind::Research {
                text: "NVDA 리서치".into(),
                no_save: false,
            }
        );
        assert_eq!(
            parse_command(&args(&["반도체", "왜", "오름?"])).unwrap(),
            CommandKind::Research {
                text: "반도체 왜 오름?".into(),
                no_save: false,
            }
        );
        assert_eq!(
            parse_command(&args(&["삼성전자", "근거", "확인", "--no-save"])).unwrap(),
            CommandKind::Research {
                text: "삼성전자 근거 확인".into(),
                no_save: true,
            }
        );
    }

    #[test]
    fn routes_leading_research_flag_without_opening_unknown_options() {
        assert_eq!(
            parse_command(&args(&["--research", "NVDA"])).unwrap(),
            CommandKind::Research {
                text: "NVDA".into(),
                no_save: false,
            }
        );
        assert_eq!(
            parse_command(&args(&["--research", "삼성전자", "--no-save"])).unwrap(),
            CommandKind::Research {
                text: "삼성전자".into(),
                no_save: true,
            }
        );
        assert!(parse_command(&args(&["--bad", "NVDA"]))
            .unwrap_err()
            .contains("unknown option '--bad'"));
    }

    #[test]
    fn routes_ticker_company_asset_only_to_safe_inquiry() {
        for asset in ["NVDA", "BTC", "비트코인", "삼성전자", "반도체"] {
            assert_eq!(
                parse_command(&args(&[asset])).unwrap(),
                CommandKind::Inquiry {
                    text: asset.into(),
                    no_save: false,
                }
            );
        }
    }

    #[test]
    fn natural_router_preserves_precedence() {
        assert!(matches!(
            parse_command(&args(&["research", "오늘", "시황"])).unwrap(),
            CommandKind::Research { .. }
        ));
        assert_eq!(
            parse_command(&args(&["ask", "NVDA", "리서치?"])).unwrap(),
            CommandKind::Inquiry {
                text: "NVDA 리서치?".into(),
                no_save: false,
            }
        );
        assert_eq!(
            parse_command(&args(&["now", "리서치"])).unwrap(),
            CommandKind::Now
        );
        assert_eq!(
            parse_command(&args(&["오늘", "시황"])).unwrap(),
            CommandKind::Now
        );
        assert!(matches!(
            parse_command(&args(&["오늘", "시황", "근거"])).unwrap(),
            CommandKind::Research { .. }
        ));
        assert!(matches!(
            parse_command(&args(&["오늘", "시황", "--research"])).unwrap(),
            CommandKind::Research { .. }
        ));
        assert!(matches!(
            parse_command(&args(&["오늘", "복기", "--research"])).unwrap(),
            CommandKind::Research { .. }
        ));
        assert!(matches!(
            parse_command(&args(&["전에", "금리", "찾아줘", "--research"])).unwrap(),
            CommandKind::Research { .. }
        ));
        assert_eq!(
            parse_command(&args(&[
                "내",
                "생각",
                "반도체가",
                "시장",
                "주도인지",
                "확인"
            ]))
            .unwrap(),
            CommandKind::Think
        );
        assert_eq!(
            parse_command(&args(&["전에", "반도체", "확인한", "내용", "찾아줘"])).unwrap(),
            CommandKind::Find
        );
        assert!(matches!(
            parse_command(&args(&["내", "생각", "반도체", "확인", "--research"])).unwrap(),
            CommandKind::Research { .. }
        ));
        assert!(matches!(
            parse_command(&args(&["--research", "NVDA"])).unwrap(),
            CommandKind::Research { .. }
        ));
        assert!(parse_command(&args(&["--bad", "NVDA"])).is_err());
        assert!(matches!(
            parse_command(&args(&["NVDA"])).unwrap(),
            CommandKind::Inquiry { .. }
        ));
        assert!(matches!(
            parse_command(&args(&["NVDA", "리서치"])).unwrap(),
            CommandKind::Research { .. }
        ));
    }

    #[test]
    fn intent_markers_beat_generic_research_terms_unless_research_flagged() {
        assert_eq!(
            parse_command(&args(&[
                "내",
                "생각",
                "반도체가",
                "시장",
                "주도인지",
                "확인"
            ]))
            .unwrap(),
            CommandKind::Think
        );
        assert_eq!(
            parse_command(&args(&["메모", "달러", "움직임", "확인"])).unwrap(),
            CommandKind::Think
        );
        assert_eq!(
            parse_command(&args(&["전에", "반도체", "확인한", "내용", "찾아줘"])).unwrap(),
            CommandKind::Find
        );
        assert_eq!(
            parse_command(&args(&["오늘", "복기", "근거"])).unwrap(),
            CommandKind::Review
        );
        assert!(matches!(
            parse_command(&args(&["내", "생각", "반도체", "확인", "--research"])).unwrap(),
            CommandKind::Research { .. }
        ));
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
    fn research_output_renders_no_provider_unavailable_and_boundary() {
        let inquiry = make_inquiry("금리 하락이 성장주에 좋은 신호임?", None);
        let query = ResearchQuery {
            question: inquiry.question.clone(),
            linked: None,
        };
        let bundle = research_bundle_from_provider(&NoopResearchProvider, &query);
        let out = render_research_inquiry(&inquiry, &bundle);
        for section in [
            "Research unavailable",
            "Sources checked",
            "Source-backed research is unavailable",
            "MARKET_PULSE_SEARCH_CMD",
            "inference scaffolding only",
            "What the sources suggest",
            "Evidence against / counter-view",
            "Data to check next",
            "Boundary",
            "not investment advice",
        ] {
            assert!(out.contains(section), "missing section: {section}");
        }
        assert!(!out.contains("Research-backed Inquiry"));
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
        assert!(out.contains("Source-backed Research Inquiry"));
        assert!(!out.contains("Research unavailable"));
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
    fn help_text_lists_earnings_command() {
        assert!(help_text().contains("mp earnings --no-save"));
    }

    #[test]
    fn routes_earnings_command_and_rejects_persistence() {
        assert_eq!(
            parse_command(&args(&["earnings"])).unwrap(),
            CommandKind::Earnings
        );
        assert_eq!(
            parse_command(&args(&["earnings", "--no-save"])).unwrap(),
            CommandKind::Earnings
        );
        assert!(parse_command(&args(&["earnings", "--save"]))
            .unwrap_err()
            .contains("deferred in v1"));
        assert!(parse_command(&args(&["earnings", "--bad"]))
            .unwrap_err()
            .contains("unknown earnings option"));
    }

    #[test]
    fn earnings_no_provider_renders_safe_fallback() {
        let bundle = earnings_bundle_from_provider(&NoopEarningsProvider);
        let out = render_earnings(&bundle);
        assert!(out.contains("Earnings Pulse"));
        assert!(out.contains("No source-backed earnings hints available"));
        assert!(out.contains("No built-in earnings database"));
        assert!(out.contains("not investment advice"));
        assert!(out.contains("not an official filings or earnings database replacement"));
    }

    #[test]
    fn earnings_search_provider_buckets_by_query_origin() {
        let provider = SearchCommandEarningsProvider {
            template: "./adapters/search-command/fixture-jsonl {query}".into(),
        };
        let bundle = earnings_bundle_from_provider(&provider);
        assert_eq!(bundle.provider, "search-cmd");
        assert_eq!(bundle.recent.len(), 1);
        assert_eq!(bundle.upcoming.len(), 1);
        assert!(bundle.recent[0]
            .source
            .evidence
            .contains("recent major US earnings results"));
        assert!(bundle.upcoming[0]
            .source
            .evidence
            .contains("upcoming major US earnings"));
        assert!(bundle.notes.iter().any(|n| n.contains("recent-results")));
        assert!(bundle.notes.iter().any(|n| n.contains("upcoming-radar")));
        assert_eq!(bundle.recent[0].freshness, "unknown");
    }

    #[test]
    fn earnings_render_does_not_infer_exact_fields_from_unstructured_sources() {
        let bundle = EarningsBundle {
            timestamp: "2026-04-28T00:00:00+0000".into(),
            provider: "fixture".into(),
            recent: vec![earnings_hint_fixture(Some("2026-04-28T01:00:00Z"), "fresh")],
            upcoming: Vec::new(),
            notes: vec!["fixture note".into()],
        };
        let out = render_earnings(&bundle);
        assert!(out.contains("EPS: actual=unknown estimate=unknown surprise=unknown"));
        assert!(out.contains("Revenue: actual=unknown estimate=unknown"));
        assert!(out.contains("Guidance: unknown"));
        assert!(out.contains("freshness: fresh"));
        assert!(out.contains("Evidence: Fixture says EPS beat by $1 and revenue was huge"));
    }

    #[test]
    fn earnings_render_uses_explicit_structured_fields() {
        let line = r#"{"title":"Nvidia earnings","publisher":"fixture","url":"fixture://nvda","evidence":"structured row","relevance":"earnings","published_at":"2026-04-28","ticker":"NVDA","company":"Nvidia","report_date":"2026-05-20","timing":"after close","eps_actual":"1.23","eps_estimate":"1.10","revenue_actual":"10B","revenue_estimate":"9B","surprise":"beat","guidance":"raised","price_reaction":"+4%"}"#;
        let (rows, invalid) = parse_earnings_jsonl(line, 10);
        assert_eq!(invalid, 0);
        let (source, fields) = rows.into_iter().next().unwrap();
        let bundle = EarningsBundle {
            timestamp: "2026-04-28T00:00:00+0000".into(),
            provider: "fixture".into(),
            recent: vec![EarningsHint {
                source,
                fields,
                freshness: "fresh",
            }],
            upcoming: Vec::new(),
            notes: Vec::new(),
        };
        let out = render_earnings(&bundle);
        assert!(out.contains("Company: NVDA · Nvidia"));
        assert!(out.contains("Report: date=2026-05-20 timing=after close price_reaction=+4%"));
        assert!(out.contains("EPS: actual=1.23 estimate=1.10 surprise=beat"));
        assert!(out.contains("Revenue: actual=10B estimate=9B"));
        assert!(out.contains("Guidance: raised"));
    }

    #[test]
    fn earnings_freshness_markers_are_deterministic() {
        assert_eq!(
            earnings_freshness_with_today(None, Some("2026-04-28")),
            "unknown"
        );
        assert_eq!(
            earnings_freshness_with_today(Some("fixture-time"), Some("2026-04-28")),
            "unknown"
        );
        assert_eq!(
            earnings_freshness_with_today(Some("2026-04-20"), Some("2026-04-28")),
            "fresh"
        );
        assert_eq!(
            earnings_freshness_with_today(Some("2026-04-28T01:00:00Z"), Some("2026-04-28")),
            "fresh"
        );
        assert_eq!(
            earnings_freshness_with_today(Some("2026-04-01"), Some("2026-04-28")),
            "stale"
        );
    }

    #[test]
    fn earnings_search_provider_degrades_gracefully() {
        let provider = SearchCommandEarningsProvider {
            template: "/definitely/missing-market-pulse-search {query}".into(),
        };
        let bundle = earnings_bundle_from_provider(&provider);
        assert_eq!(bundle.provider, "search-cmd");
        assert!(bundle.recent.is_empty());
        assert!(bundle.upcoming.is_empty());
        assert!(bundle.notes.iter().any(|n| n.contains("failed gracefully")));
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
        assert!(f.next.iter().any(|x| x.contains("증거")));
        assert!(f.next.iter().any(|x| x.contains("틀리게")));
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
        assert!(out.contains("검증 루프"));
        assert!(out.contains("대안"));
        assert!(out.contains("반증"));
    }

    #[test]
    fn review_counts_radar_and_fomo_checkpoint_events() {
        let events = vec![
            "{\"type\":\"radar\",\"timestamp\":\"2026-04-23T09:00:00+0900\",\"scenario\":\"Semis-led growth risk-on; watch BTC/KOSPI confirmation\",\"confirm\":\"Semis and Nasdaq keep leading\",\"falsify\":\"Semis lose leadership first\",\"watch\":\"Semis vs Nasdaq\",\"prompt\":\"Run mp think\"}".into(),
            "{\"type\":\"fomo_check\",\"timestamp\":\"2026-04-23T09:05:00+0900\",\"linked_radar_timestamp\":\"2026-04-23T09:00:00+0900\",\"scenario\":\"Semis-led growth risk-on; watch BTC/KOSPI confirmation\",\"prompt\":\"Run mp think\"}".into(),
        ];
        let out = render_review_from_events(&events, "/tmp/journal.jsonl");

        assert!(out.contains("radars 1"));
        assert!(out.contains("fomo 1"));
        assert!(out.contains("semis"));
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
    fn week_date_filter_keeps_last_matching_dates() {
        let events = vec![
            "{\"type\":\"thought\",\"timestamp\":\"2026-04-19T09:00:00+0900\",\"text\":\"유가\",\"linked_pulse_timestamp\":null}".into(),
            "{\"type\":\"thought\",\"timestamp\":\"2026-04-20T10:00:00+0900\",\"text\":\"금리\",\"linked_pulse_timestamp\":null}".into(),
            "{\"type\":\"week\",\"timestamp\":\"2026-04-20T11:00:00+0900\",\"label\":\"mixed\"}".into(),
            "{\"type\":\"thought\",\"timestamp\":\"2026-04-21T11:00:00+0900\",\"text\":\"달러\",\"linked_pulse_timestamp\":null}".into(),
        ];
        let dates = vec!["2026-04-20".into(), "2026-04-21".into()];
        let filtered = filter_events_by_dates(events, &dates, 10);
        assert_eq!(filtered.len(), 3);
        assert!(filtered[0].contains("10:00:00"));
        assert!(filtered[1].contains("\"type\":\"week\""));
        assert!(filtered[2].contains("11:00:00"));
    }

    #[test]
    fn review_period_filter_renders_label_and_matching_dates() {
        let events = vec![
            "{\"type\":\"thought\",\"timestamp\":\"2026-04-19T09:00:00+0900\",\"text\":\"유가\",\"linked_pulse_timestamp\":null}".into(),
            "{\"type\":\"thought\",\"timestamp\":\"2026-04-20T10:00:00+0900\",\"text\":\"금리\",\"linked_pulse_timestamp\":null}".into(),
            "{\"type\":\"inquiry\",\"timestamp\":\"2026-04-21T11:00:00+0900\",\"question\":\"달러와 코스피?\",\"thesis_type\":\"dollar-liquidity transmission thesis\",\"concepts\":\"dollar liquidity\"}".into(),
        ];
        let dates = vec!["2026-04-20".into(), "2026-04-21".into()];
        let filtered = filter_events_by_dates(events, &dates, 10);
        let out = render_review_for_dates_from_events(
            &filtered,
            "/tmp/journal.jsonl",
            "this-week 2026-04-20..2026-04-21",
        );
        assert!(out.contains("Review period filter: this-week 2026-04-20..2026-04-21"));
        assert!(out.contains("Entries scanned: 2"));
        assert!(out.contains("rates"));
        assert!(out.contains("fx"));
        assert!(!out.contains("oil"));
    }

    #[test]
    fn review_rejects_multiple_period_selectors() {
        assert!(review(&["review".into(), "--today".into(), "--this-week".into()]).is_err());
        assert!(review(&[
            "review".into(),
            "--date".into(),
            "2026-04-21".into(),
            "--last-week".into(),
        ])
        .is_err());
    }

    fn fixture_dt(date: &str, time: &str, weekday: u32, zone: &str) -> ExchangeDateTime {
        ExchangeDateTime {
            date: date.into(),
            time: time.into(),
            weekday,
            zone: zone.into(),
        }
    }

    struct FailingClock;

    impl ExchangeClock for FailingClock {
        fn now_in(&self, _tz: &str) -> Option<ExchangeDateTime> {
            None
        }
    }

    #[test]
    fn calendar_renders_review_shortcuts_and_boundary() {
        let out = render_calendar();
        assert!(out.contains("Market Pulse Calendar"));
        assert!(out.contains("Local date windows"));
        assert!(out.contains("today:"));
        assert!(out.contains("yesterday:"));
        assert!(out.contains("this-week:"));
        assert!(out.contains("last-week:"));
        assert!(out.contains("mp review --today"));
        assert!(out.contains("mp review --yesterday"));
        assert!(out.contains("mp review --this-week"));
        assert!(out.contains("mp review --last-week"));
        assert!(out.contains("not fuzzy full-text search"));
        assert!(out.contains("curated static exchange-calendar context"));
        assert!(out.contains("not a live official exchange feed"));
        assert!(out.contains("not") && out.contains("trading signal"));
    }

    #[test]
    fn calendar_renders_exchange_session_section() {
        let out = render_calendar();
        assert!(out.contains("Exchange sessions (curated static rules)"));
        assert!(out.contains("US equities"));
        assert!(out.contains("NYSE/Nasdaq"));
        assert!(out.contains("Korea equities"));
        assert!(out.contains("KRX/KOSPI"));
        assert!(out.contains("ET") || out.contains("EDT") || out.contains("EST"));
        assert!(out.contains("KST"));
        assert!(out.contains("09:30-16:00 ET"));
        assert!(out.contains("09:00-15:30 KST"));
        assert!(out.contains("Source / freshness"));
        assert!(out.contains("last curated 2026-04-23"));
    }

    #[test]
    fn calendar_renders_pulse_bridge() {
        let out = render_calendar();
        assert!(out.contains("Calendar ↔ pulse bridge"));
        assert!(out.contains("mp now: close-to-close daily pulse"));
        assert!(out.contains("mp week: local journal week"));
        assert!(out.contains("first matching Yahoo daily close"));
        assert!(out.contains("session dates can differ"));
    }

    #[test]
    fn us_calendar_has_coverage_metadata() {
        assert_eq!(US_EQUITIES_CALENDAR.coverage_years, &[2026, 2027]);
        assert_eq!(US_EQUITIES_CALENDAR.last_curated, "2026-04-23");
        assert!(US_EQUITIES_CALENDAR
            .source_labels
            .iter()
            .any(|s| s.contains("Nasdaq")));
        assert!(US_EQUITIES_CALENDAR
            .source_labels
            .iter()
            .any(|s| s.contains("NYSE")));
        let fixture_2027 = fixture_dt("2027-01-01", "10:00", 5, "EST");
        assert!(
            session_status(&US_EQUITIES_CALENDAR, Some(&fixture_2027)).contains("source-limited")
        );
    }

    #[test]
    fn us_calendar_marks_2026_closures() {
        for (date, reason) in [
            ("2026-01-01", "New Year's Day"),
            ("2026-01-19", "Martin Luther King"),
            ("2026-04-03", "Good Friday"),
            ("2026-11-26", "Thanksgiving"),
            ("2026-12-25", "Christmas"),
        ] {
            let rule = closure_for_date(&US_EQUITIES_CALENDAR, date).expect("closure fixture");
            assert!(
                rule.reason.contains(reason),
                "{date} should contain {reason}"
            );
        }
    }

    #[test]
    fn us_calendar_marks_2026_early_closes() {
        for date in ["2026-11-27", "2026-12-24"] {
            let rule =
                early_close_for_date(&US_EQUITIES_CALENDAR, date).expect("early close fixture");
            assert_eq!(rule.close_minutes, 13 * 60);
            assert_eq!(rule.close_label, "13:00 ET");
        }
    }

    #[test]
    fn kr_calendar_exposes_partial_coverage_metadata() {
        assert_eq!(
            KRX_EQUITIES_CALENDAR.default_coverage,
            CalendarCoverage::Partial
        );
        assert!(KRX_EQUITIES_CALENDAR.coverage_note.contains("partial"));
        let regular_day = fixture_dt("2026-04-23", "10:00", 4, "KST");
        let status = session_status(&KRX_EQUITIES_CALENDAR, Some(&regular_day));
        assert!(status.contains("regular session by partial KRX rules"));
        assert!(!status.contains("open under full curated calendar rules"));
    }

    #[test]
    fn us_grouped_row_handles_source_divergence_matrix() {
        let cases = [
            (SourceAgreement::Agree, CalendarCoverage::Full),
            (SourceAgreement::NyseOnly, CalendarCoverage::SourceLimited),
            (SourceAgreement::NasdaqOnly, CalendarCoverage::SourceLimited),
            (SourceAgreement::BothMissing, CalendarCoverage::Unavailable),
            (SourceAgreement::Disagree, CalendarCoverage::SourceLimited),
        ];
        for (agreement, expected) in cases {
            assert_eq!(grouped_us_coverage_from_agreement(agreement), expected);
        }
        let out = render_calendar();
        assert!(out.contains("NYSE+Nasdaq agree"));
        assert!(out.contains("NYSE only"));
        assert!(out.contains("Nasdaq only"));
        assert!(out.contains("both missing"));
        assert!(out.contains("sources disagree"));
    }

    #[test]
    fn exchange_clock_runtime_failure_degrades_to_unavailable() {
        let rows = exchange_session_rows_with_clock(&FailingClock);
        for row in rows {
            assert!(row.status.contains("status unavailable"));
            assert_eq!(row.coverage, CalendarCoverage::Unavailable);
            assert!(row.exchange_local.contains("unavailable"));
        }
    }

    #[test]
    fn session_logic_is_fixture_driven() {
        let dt = fixture_dt("2026-04-23", "10:00", 4, "EDT");
        let status = session_status(&US_EQUITIES_CALENDAR, Some(&dt));
        assert!(status.contains("open under curated static calendar rules"));
    }

    #[test]
    fn session_status_pre_open_open_after_weekend_holiday_and_early_close() {
        let pre = fixture_dt("2026-04-23", "08:00", 4, "EDT");
        assert!(
            session_status(&US_EQUITIES_CALENDAR, Some(&pre)).contains("before regular session")
        );

        let open = fixture_dt("2026-04-23", "10:00", 4, "EDT");
        assert!(session_status(&US_EQUITIES_CALENDAR, Some(&open))
            .contains("open under curated static calendar rules"));

        let after = fixture_dt("2026-04-23", "16:30", 4, "EDT");
        assert!(
            session_status(&US_EQUITIES_CALENDAR, Some(&after)).contains("after regular session")
        );

        let weekend = fixture_dt("2026-04-25", "10:00", 6, "EDT");
        assert_eq!(
            session_status(&US_EQUITIES_CALENDAR, Some(&weekend)),
            "closed: weekend"
        );

        let holiday = fixture_dt("2026-11-26", "10:00", 4, "EST");
        assert!(session_status(&US_EQUITIES_CALENDAR, Some(&holiday))
            .contains("closed: holiday (Thanksgiving"));

        let early_open = fixture_dt("2026-11-27", "12:00", 5, "EST");
        assert!(session_status(&US_EQUITIES_CALENDAR, Some(&early_open))
            .contains("early close today: closes 13:00 ET"));

        let early_after = fixture_dt("2026-11-27", "13:30", 5, "EST");
        assert!(
            session_status(&US_EQUITIES_CALENDAR, Some(&early_after)).contains("after early close")
        );
    }

    #[test]
    fn session_status_unavailable_when_outside_coverage() {
        let dt = fixture_dt("2028-04-23", "10:00", 1, "EDT");
        assert!(session_status(&US_EQUITIES_CALENDAR, Some(&dt)).contains("status unavailable"));
    }

    #[test]
    fn session_date_differs_between_korea_and_us_fixture() {
        let us = exchange_session_row(
            &US_EQUITIES_CALENDAR,
            Some(fixture_dt("2026-04-22", "20:30", 3, "EDT")),
        );
        let kr = exchange_session_row(
            &KRX_EQUITIES_CALENDAR,
            Some(fixture_dt("2026-04-23", "09:30", 4, "KST")),
        );
        assert!(us.exchange_local.contains("2026-04-22"));
        assert!(kr.exchange_local.contains("2026-04-23"));
    }

    #[test]
    fn session_status_precedence_is_deterministic() {
        assert!(session_status(&US_EQUITIES_CALENDAR, None).contains("timestamp unavailable"));
        let weekend_holiday = fixture_dt("2026-07-04", "10:00", 6, "EDT");
        assert_eq!(
            session_status(&US_EQUITIES_CALENDAR, Some(&weekend_holiday)),
            "closed: weekend"
        );
        let source_limited_holiday = fixture_dt("2027-01-01", "10:00", 5, "EST");
        assert!(
            session_status(&US_EQUITIES_CALENDAR, Some(&source_limited_holiday))
                .contains("source-limited")
        );
        let holiday = fixture_dt("2026-01-01", "10:00", 4, "EST");
        assert!(session_status(&US_EQUITIES_CALENDAR, Some(&holiday)).contains("closed: holiday"));
        let early = fixture_dt("2026-11-27", "12:00", 5, "EST");
        assert!(session_status(&US_EQUITIES_CALENDAR, Some(&early)).contains("early close"));
        let regular = fixture_dt("2026-04-23", "10:00", 4, "EDT");
        assert!(
            session_status(&US_EQUITIES_CALENDAR, Some(&regular)).contains("open under curated")
        );
        let partial = fixture_dt("2026-04-23", "10:00", 4, "KST");
        assert!(session_status(&KRX_EQUITIES_CALENDAR, Some(&partial)).contains("partial KRX"));
    }

    #[test]
    fn calendar_does_not_render_live_event_agenda() {
        let out = render_calendar();
        for forbidden in [
            "CPI",
            "FOMC",
            "earnings agenda",
            "IPO agenda",
            "news agenda",
        ] {
            assert!(
                !out.contains(forbidden),
                "calendar should not include live agenda term {forbidden}"
            );
        }
    }

    #[test]
    fn calendar_output_avoids_trading_advice_language() {
        let out = render_calendar().to_lowercase();
        for forbidden in [
            "buy",
            "sell",
            "price target",
            "stop-loss",
            "portfolio",
            "매수",
            "매도",
            "손절",
            "목표가",
        ] {
            assert!(
                !out.contains(forbidden),
                "calendar output should avoid advice term: {forbidden}"
            );
        }
    }

    #[test]
    fn find_args_collect_query_and_period_selector() {
        let args = vec![
            "find".into(),
            "금리".into(),
            "성장주".into(),
            "--this-week".into(),
            "--limit".into(),
            "3".into(),
        ];
        let parsed = parse_find_args(&args).unwrap();
        assert_eq!(parsed.query, "금리 성장주");
        assert_eq!(parsed.limit, 3);
        match parsed.filter.unwrap() {
            ReviewFilter::Dates { label, dates } => {
                assert!(label.starts_with("this-week"));
                assert!(!dates.is_empty());
            }
            ReviewFilter::Date(_) => panic!("expected period filter"),
        }
    }

    #[test]
    fn find_rejects_empty_query_and_multiple_selectors() {
        assert!(parse_find_args(&["find".into(), "--this-week".into()]).is_err());
        assert!(parse_find_args(&[
            "find".into(),
            "달러".into(),
            "--today".into(),
            "--last-week".into(),
        ])
        .is_err());
    }

    #[test]
    fn find_events_filters_by_query_after_date_window() {
        let events = vec![
            "{\"type\":\"thought\",\"timestamp\":\"2026-04-19T09:00:00+0900\",\"text\":\"지난주 유가와 달러\",\"linked_pulse_timestamp\":null}".into(),
            "{\"type\":\"thought\",\"timestamp\":\"2026-04-20T10:00:00+0900\",\"text\":\"이번주 금리와 성장주\",\"linked_pulse_timestamp\":null}".into(),
            "{\"type\":\"inquiry\",\"timestamp\":\"2026-04-21T11:00:00+0900\",\"question\":\"달러가 코스피에 부담?\",\"thesis_type\":\"dollar-liquidity transmission thesis\",\"concepts\":\"dollar liquidity\"}".into(),
        ];
        let dates = vec!["2026-04-20".into(), "2026-04-21".into()];
        let filtered = filter_events_by_dates(events, &dates, 10);
        let found = filtered
            .into_iter()
            .filter(|line| line.to_lowercase().contains("달러"))
            .collect::<Vec<_>>();
        assert_eq!(found.len(), 1);
        assert!(found[0].contains("코스피"));
    }

    #[test]
    fn find_renders_recall_card_and_boundary() {
        let events = vec![
            "{\"type\":\"thought\",\"timestamp\":\"2026-04-20T10:00:00+0900\",\"text\":\"이번주 금리와 성장주 긴장\",\"linked_pulse_timestamp\":null}".into(),
            "{\"type\":\"inquiry\",\"timestamp\":\"2026-04-21T11:00:00+0900\",\"question\":\"금리 하락이 성장주에 좋은 신호임?\",\"thesis_type\":\"rates / policy-expectation thesis\",\"concepts\":\"rates policy\"}".into(),
        ];
        let out = render_find_from_events(
            &events,
            "/tmp/journal.jsonl",
            "금리",
            "this-week 2026-04-20..2026-04-21".into(),
        );
        assert!(out.contains("Market Pulse Find"));
        assert!(out.contains("Query: \"금리\""));
        assert!(out.contains("Entries matched: 2"));
        assert!(out.contains("Next recall question"));
        assert!(out.contains("this-week 2026-04-20..2026-04-21"));
        assert!(out.contains("금리·달러"));
        assert!(out.contains("local journal only"));
        assert!(out.contains("not live research or trading advice"));
    }

    #[test]
    fn find_renders_radar_and_fomo_snippets() {
        let events = vec![
            "{\"type\":\"radar\",\"timestamp\":\"2026-04-23T09:00:00+0900\",\"scenario\":\"Semis-led growth risk-on; watch BTC/KOSPI confirmation\",\"prompt\":\"Run mp think\"}".into(),
            "{\"type\":\"fomo_check\",\"timestamp\":\"2026-04-23T09:05:00+0900\",\"scenario\":\"Semis-led growth risk-on; watch BTC/KOSPI confirmation\",\"prompt\":\"Run mp think\"}".into(),
        ];
        let out = render_find_from_events(&events, "/tmp/journal.jsonl", "Semis", "today".into());

        assert!(out.contains("radars 1"));
        assert!(out.contains("fomo 1"));
        assert!(out.contains("radar · Semis-led growth"));
        assert!(out.contains("fomo_check · Semis-led growth"));
    }

    #[test]
    fn find_empty_result_points_to_calendar() {
        let out = render_find_from_events(
            &[],
            "/tmp/journal.jsonl",
            "반도체",
            "last-week 2026-04-13..2026-04-19".into(),
        );
        assert!(out.contains("No market-pulse journal entries matched"));
        assert!(out.contains("mp calendar"));
    }

    #[test]
    fn week_basis_names_current_calendar_window() {
        let dates = vec!["2026-04-20".into(), "2026-04-21".into()];
        let basis = week_basis(&dates);
        assert!(basis[0].contains("current-week window 2026-04-20..2026-04-21"));
        assert!(basis[1].contains("range=1mo interval=1d"));
        assert!(basis[1].contains("latest daily close"));
        assert!(basis[1].contains("regularMarketPrice is fallback only"));
        assert!(basis[2].contains("current local calendar week"));
    }

    #[test]
    fn first_close_for_dates_uses_matching_timestamp_date() {
        let body =
            "{\"timestamp\":[1776643200,1776729600],\"indicators\":{\"quote\":[{\"close\":[100.0,110.0]}]}}";
        assert_eq!(
            first_close_for_dates(body, &["2026-04-21".into()]).unwrap(),
            110.0
        );
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
            "--days".into(),
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
        assert!(rendered.contains("close-to-close pulse"));
        assert!(rendered.contains("not high/low gap, exact 24h, or weekly return"));
    }

    #[test]
    fn pulse_renders_daily_decision_checklist_in_non_compact_output() {
        let p = compose_cross_asset_test_pulse();
        let rendered = render_pulse(&p, false);

        assert_eq!(rendered.matches("Daily Decision Checklist").count(), 1);
        for label in checklist_labels() {
            assert!(rendered.contains(label), "missing {label}");
        }
    }

    #[test]
    fn pulse_daily_decision_checklist_labels_stay_in_order() {
        let p = compose_cross_asset_test_pulse();
        let rendered = render_pulse(&p, false);
        let checklist = checklist_section(&rendered);

        assert_eq!(
            checklist
                .lines()
                .filter(|line| line.starts_with("  "))
                .count(),
            6
        );
        let mut previous = 0;
        for label in checklist_labels() {
            assert_eq!(checklist.matches(label).count(), 1);
            let current = checklist.find(label).expect("label should be present");
            assert!(current >= previous, "{label} rendered out of order");
            previous = current;
        }
    }

    #[test]
    fn pulse_daily_decision_checklist_sits_between_seeds_and_source_notes() {
        let mut p = compose_cross_asset_test_pulse();
        p.notes = vec!["fixture quote source".into()];
        let rendered = render_pulse(&p, false);

        let seeds = rendered
            .find("Market puzzle / question seeds")
            .expect("question seeds should render");
        let checklist = rendered
            .find("Daily Decision Checklist")
            .expect("checklist should render");
        let notes = rendered
            .find("Source notes")
            .expect("source notes should render");

        assert!(seeds < checklist);
        assert!(checklist < notes);
    }

    #[test]
    fn radar_renders_daily_context_and_fomo_checkpoint() {
        let p = compose_cross_asset_test_pulse();
        let checklist = daily_decision_checklist(&p);
        let rendered = render_radar(&p, &checklist);

        assert!(rendered.contains("Market Radar"));
        assert!(rendered.contains("Scenario:"));
        assert!(rendered.contains("Watch:"));
        assert!(rendered.contains("Confirm:"));
        assert!(rendered.contains("Falsify:"));
        assert!(rendered.contains("FOMO checkpoint"));
        assert!(rendered.contains("opportunity-cost fear"));
        assert!(rendered.contains("Reasoning support only"));
    }

    #[test]
    fn radar_output_avoids_trading_advice_language() {
        let p = compose_cross_asset_test_pulse();
        let checklist = daily_decision_checklist(&p);
        let rendered = render_radar(&p, &checklist).to_lowercase();
        for forbidden in [
            "buy",
            "sell",
            "price target",
            "stop-loss",
            "portfolio",
            "매수",
            "매도",
            "손절",
            "목표가",
        ] {
            assert!(
                !rendered.contains(forbidden),
                "radar output should avoid advice term: {forbidden}"
            );
        }
    }

    #[test]
    fn fomo_checkpoint_renders_linked_context_and_prompts() {
        let checkpoint = FomoCheckpoint {
            timestamp: "2026-04-23T10:00:00+0900".into(),
            linked_pulse: Some("2026-04-23T09:55:00+0900".into()),
            linked_radar: Some("2026-04-23T09:56:00+0900".into()),
            scenario: Some("Semis-led growth risk-on; watch BTC/KOSPI confirmation".into()),
            confirm: Some("Semis and Nasdaq keep leading while BTC and KOSPI confirm.".into()),
            falsify: Some("Semis lose leadership first.".into()),
            watch: Some("Semis vs Nasdaq, BTC, KOSPI, DXY, USD/KRW, and US 10Y.".into()),
            prompt: FOMO_JOURNAL_PROMPT.into(),
        };
        let rendered = render_fomo_checkpoint(&checkpoint);

        assert!(rendered.contains("FOMO Checkpoint"));
        assert!(rendered.contains("Latest radar: 2026-04-23T09:56:00+0900"));
        assert!(rendered.contains("Latest pulse: 2026-04-23T09:55:00+0900"));
        assert!(rendered.contains("opportunity-cost fear"));
        assert!(rendered.contains("Carry-over checks"));
        assert!(rendered.contains("Next journal prompt"));
        assert!(rendered.contains("Reasoning support only"));
    }

    #[test]
    fn fomo_checkpoint_falls_back_without_prior_radar() {
        let checkpoint = FomoCheckpoint {
            timestamp: "2026-04-23T10:00:00+0900".into(),
            linked_pulse: None,
            linked_radar: None,
            scenario: None,
            confirm: None,
            falsify: None,
            watch: None,
            prompt: FOMO_JOURNAL_PROMPT.into(),
        };
        let rendered = render_fomo_checkpoint(&checkpoint);

        assert!(rendered.contains("Latest radar: not recorded yet"));
        assert!(rendered.contains("run `mp watch`"));
        assert!(!rendered.contains("Carry-over checks"));
    }

    #[test]
    fn radar_and_fomo_json_are_reviewable_events() {
        let p = compose_cross_asset_test_pulse();
        let checklist = daily_decision_checklist(&p);
        let radar = radar_json(&p, &checklist);
        assert!(radar.contains("\"type\":\"radar\""));
        assert!(radar.contains("\"linked_pulse_timestamp\""));
        assert!(radar.contains("\"scenario\""));
        assert!(radar.contains("\"prompt\""));

        let checkpoint = FomoCheckpoint {
            timestamp: "2026-04-23T10:00:00+0900".into(),
            linked_pulse: Some("pulse-ts".into()),
            linked_radar: Some("radar-ts".into()),
            scenario: Some(checklist.scenario.into()),
            confirm: Some(checklist.confirm.into()),
            falsify: Some(checklist.falsify.into()),
            watch: Some(checklist.watch.into()),
            prompt: FOMO_JOURNAL_PROMPT.into(),
        };
        let fomo = fomo_check_json(&checkpoint);
        assert!(fomo.contains("\"type\":\"fomo_check\""));
        assert!(fomo.contains("\"linked_radar_timestamp\":\"radar-ts\""));
        assert!(fomo.contains("\"linked_pulse_timestamp\":\"pulse-ts\""));
        assert!(fomo.contains("\"scenario\""));
    }

    #[test]
    fn pulse_compact_output_excludes_daily_decision_checklist() {
        let p = compose_cross_asset_test_pulse();
        let rendered = render_pulse(&p, true);

        assert!(rendered.starts_with("[mp] "));
        assert_eq!(rendered.lines().count(), 1);
        assert!(rendered.contains(" · "));
        assert!(rendered.contains("Puzzle: "));
        assert!(!rendered.contains("Daily Decision Checklist"));
        for label in checklist_labels() {
            assert!(!rendered.contains(label), "compact output leaked {label}");
        }
    }

    #[test]
    fn pulse_daily_decision_checklist_avoids_advice_language_and_unsupported_semis_claims() {
        let p = compose_cross_asset_test_pulse();
        let rendered = render_pulse(&p, false);
        let checklist = checklist_section(&rendered).to_lowercase();

        for forbidden in [
            "buy",
            "sell",
            "price target",
            "stop-loss",
            "position size",
            "portfolio",
            "매수",
            "매도",
            "손절",
            "비중",
            "semis-led",
            "semiconductor strength",
        ] {
            assert!(
                !checklist.contains(forbidden),
                "checklist should not contain {forbidden}"
            );
        }
    }

    #[test]
    fn pulse_daily_decision_checklist_is_not_persisted_in_pulse_json() {
        let p = compose_cross_asset_test_pulse();
        let json = pulse_json(&p);

        assert!(!json.contains("Daily Decision Checklist"));
        assert!(!json.contains("Scenario"));
        assert!(!json.contains("Confirm"));
        assert!(!json.contains("Falsify"));
        assert!(!json.contains("Discipline"));
        assert!(!json.contains("Journal"));
    }

    #[test]
    fn sox_proxy_is_pulse_only_not_shared_with_week_or_regime() {
        assert!(!SYMBOLS.iter().any(|(symbol, _, _)| *symbol == "^SOX"));
        assert_eq!(PULSE_ONLY_SYMBOLS, &[("^SOX", "Semis", "")]);

        let week = render_week(&compose_test_week(compose_base_test_assets()), &[]);
        let regime = render_regime(&compose_test_regime(compose_base_test_assets()));

        assert!(!week.contains("Semis"));
        assert!(!regime.contains("Semis"));
    }

    #[test]
    fn pulse_renders_pulse_only_semis_asset_in_non_compact_output() {
        let p = compose_semis_test_pulse(Some(2.0), 1.2, 0.7, 0.0);
        let rendered = render_pulse(&p, false);

        assert!(rendered.contains("  - Semis: 100.00 (+2.00%)"));
    }

    #[test]
    fn pulse_daily_decision_checklist_detects_semis_led_growth_without_macro_pressure() {
        let p = compose_semis_test_pulse(Some(2.0), 1.2, 0.7, 0.0);
        let rendered = render_pulse(&p, false);
        let checklist = checklist_section(&rendered);

        assert!(
            checklist.contains("Scenario: Semis-led growth risk-on; watch BTC/KOSPI confirmation")
        );
        assert!(checklist.contains("Confirm: Semis and Nasdaq keep leading"));
        assert!(checklist.contains("Watch: Semis vs Nasdaq"));
    }

    #[test]
    fn pulse_daily_decision_checklist_detects_semis_led_growth_with_macro_pressure() {
        let p = compose_semis_test_pulse(Some(2.0), 1.2, 0.7, 0.8);
        let rendered = render_pulse(&p, false);
        let checklist = checklist_section(&rendered);

        assert!(
            checklist.contains("Scenario: Semis-led growth risk-on; macro confirmation incomplete")
        );
        assert!(checklist.contains("dollar/rates pressure stops rising"));
    }

    #[test]
    fn pulse_daily_decision_checklist_detects_weak_semis_when_available() {
        let p = compose_semis_test_pulse(Some(-0.6), 1.2, 0.7, 0.0);
        let rendered = render_pulse(&p, false);
        let checklist = checklist_section(&rendered);

        assert!(checklist.contains(
            "Scenario: Growth leadership fading; watch whether semis or BTC breaks first"
        ));
    }

    #[test]
    fn pulse_daily_decision_checklist_detects_nasdaq_without_semis_confirmation() {
        let p = compose_semis_test_pulse(Some(0.2), 1.2, 0.7, 0.0);
        let rendered = render_pulse(&p, false);
        let checklist = checklist_section(&rendered);

        assert!(
            checklist.contains("Scenario: Nasdaq risk-on with semis confirmation still pending")
        );
    }

    #[test]
    fn pulse_daily_decision_checklist_does_not_claim_semis_leadership_without_semis_change() {
        let p = compose_semis_test_pulse(None, 1.2, 0.7, 0.0);
        let rendered = render_pulse(&p, false);
        let checklist = checklist_section(&rendered).to_lowercase();

        assert!(!checklist.contains("semis-led"));
        assert!(!checklist.contains("semiconductor leadership"));
    }

    #[test]
    fn now_change_prefers_daily_close_series_over_market_quote() {
        let body = r#"{"meta":{"regularMarketPrice":200,"chartPreviousClose":150},"indicators":{"quote":[{"close":[100,110,121]}]}}"#;

        let (value, previous) = value_and_previous_for_window(
            body,
            &close_values(body),
            &WindowChange::PriorDailyClose,
        );

        assert_eq!(value, Some(121.0));
        assert_eq!(previous, Some(110.0));
    }

    #[test]
    fn week_change_prefers_daily_close_series_over_market_quote() {
        let body = r#"{"timestamp":[1776643200,1776729600,1776816000],"meta":{"regularMarketPrice":200,"chartPreviousClose":150},"indicators":{"quote":[{"close":[100,110,121]}]}}"#;

        let (value, previous) = value_and_previous_for_window(
            body,
            &close_values(body),
            &WindowChange::FirstMatchingDate(vec!["2026-04-21".into()]),
        );

        assert_eq!(value, Some(121.0));
        assert_eq!(previous, Some(110.0));
    }

    #[test]
    fn week_change_without_matching_date_falls_back_to_latest_close() {
        let body = r#"{"timestamp":[1776643200,1776729600],"meta":{"regularMarketPrice":200,"chartPreviousClose":150},"indicators":{"quote":[{"close":[100,110]}]}}"#;

        let (value, previous) = value_and_previous_for_window(
            body,
            &close_values(body),
            &WindowChange::FirstMatchingDate(vec!["2026-04-24".into()]),
        );

        assert_eq!(value, Some(110.0));
        assert_eq!(previous, Some(110.0));
    }

    #[test]
    fn regime_change_prefers_weekly_close_series_over_market_quote() {
        let body = r#"{"meta":{"regularMarketPrice":200,"chartPreviousClose":150},"indicators":{"quote":[{"close":[100,110,121]}]}}"#;

        let (value, previous) =
            value_and_previous_for_window(body, &close_values(body), &WindowChange::FirstClose);

        assert_eq!(value, Some(121.0));
        assert_eq!(previous, Some(100.0));
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
        assert!(rendered.contains("latest Yahoo weekly close value"));
        assert!(rendered.contains("regularMarketPrice is fallback only"));
        assert!(rendered.contains("Next better regime question"));
        assert!(rendered.contains("not investment advice"));
    }

    #[test]
    fn week_renders_hybrid_market_and_learning_card() {
        let week = compose_test_week(vec![
            Asset {
                symbol: "^IXIC",
                label: "Nasdaq",
                unit: "",
                value: Some(100.0),
                change: Some(2.0),
                note: None,
            },
            Asset {
                symbol: "^GSPC",
                label: "S&P 500",
                unit: "",
                value: Some(100.0),
                change: Some(1.0),
                note: None,
            },
            Asset {
                symbol: "^TNX",
                label: "US 10Y",
                unit: "%",
                value: Some(4.8),
                change: Some(1.8),
                note: None,
            },
        ]);
        let events = vec![
            "{\"type\":\"inquiry\",\"timestamp\":\"2026-04-21T09:00:00+0900\",\"question\":\"금리와 반도체가 같이 움직이나?\",\"thesis_type\":\"rates/growth tension thesis\",\"concepts\":\"rates vs growth\"}".into(),
            "{\"type\":\"thought\",\"timestamp\":\"2026-04-21T10:00:00+0900\",\"text\":\"달러가 강한데 코스피가 버틴다\",\"linked_pulse_timestamp\":null}".into(),
        ];
        let rendered = render_week(&week, &events);
        assert!(rendered.contains("Weekly Market Pulse"));
        assert!(rendered.contains("1W Asset Map"));
        assert!(rendered.contains("latest daily close"));
        assert!(rendered.contains("regularMarketPrice is fallback only"));
        assert!(rendered.contains("This week's learning loop"));
        assert!(rendered.contains("Recurring journal themes"));
        assert!(rendered.contains("Next week check questions"));
        assert!(rendered.contains("evidence"));
        assert!(rendered.contains("rename this week"));
        assert!(rendered.contains("rates"));
        assert!(rendered.contains("not investment advice"));
    }

    fn compose_base_test_assets() -> Vec<Asset> {
        vec![
            Asset {
                symbol: "^GSPC",
                label: "S&P 500",
                unit: "",
                value: Some(100.0),
                change: Some(1.2),
                note: None,
            },
            Asset {
                symbol: "^IXIC",
                label: "Nasdaq",
                unit: "",
                value: Some(100.0),
                change: Some(2.0),
                note: None,
            },
            Asset {
                symbol: "^KS11",
                label: "KOSPI",
                unit: "",
                value: Some(100.0),
                change: Some(0.8),
                note: None,
            },
            Asset {
                symbol: "KRW=X",
                label: "USD/KRW",
                unit: "KRW",
                value: Some(1400.0),
                change: Some(0.2),
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
            Asset {
                symbol: "^TNX",
                label: "US 10Y",
                unit: "%",
                value: Some(4.8),
                change: Some(0.8),
                note: None,
            },
            Asset {
                symbol: "CL=F",
                label: "WTI",
                unit: "USD",
                value: Some(70.0),
                change: Some(1.2),
                note: None,
            },
            Asset {
                symbol: "BTC-USD",
                label: "BTC",
                unit: "USD",
                value: Some(100000.0),
                change: Some(3.0),
                note: None,
            },
        ]
    }

    fn compose_cross_asset_test_pulse() -> Pulse {
        compose_test_pulse(compose_base_test_assets())
    }

    fn compose_semis_test_pulse(
        semis_change: Option<f64>,
        nasdaq_change: f64,
        spx_change: f64,
        rates_change: f64,
    ) -> Pulse {
        let mut assets = compose_base_test_assets();
        for asset in &mut assets {
            match asset.symbol {
                "^GSPC" => asset.change = Some(spx_change),
                "^IXIC" => asset.change = Some(nasdaq_change),
                "^TNX" => asset.change = Some(rates_change),
                "KRW=X" | "DX-Y.NYB" | "CL=F" => asset.change = Some(0.0),
                "^KS11" => asset.change = Some(0.8),
                "BTC-USD" => asset.change = Some(2.0),
                _ => {}
            }
        }
        assets.push(Asset {
            symbol: "^SOX",
            label: "Semis",
            unit: "",
            value: Some(100.0),
            change: semis_change,
            note: None,
        });
        compose_test_pulse(assets)
    }

    fn checklist_labels() -> [&'static str; 6] {
        [
            "  Scenario:",
            "  Confirm:",
            "  Falsify:",
            "  Watch:",
            "  Discipline:",
            "  Journal:",
        ]
    }

    fn checklist_section(rendered: &str) -> &str {
        rendered
            .split("Daily Decision Checklist")
            .nth(1)
            .expect("checklist section should render")
            .split("\nSource notes")
            .next()
            .expect("checklist section should be split from notes")
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

    fn compose_test_week(assets: Vec<Asset>) -> Weekly {
        let avg_equity = avg_change(&assets, &["^GSPC", "^IXIC", "^KS11"]).unwrap_or(0.0);
        let label = infer_week_label(
            avg_equity,
            change_for(&assets, "DX-Y.NYB"),
            change_for(&assets, "^TNX"),
            change_for(&assets, "CL=F"),
            change_for(&assets, "BTC-USD"),
        );
        let drivers = infer_week_drivers(&assets);
        let tensions = infer_week_tensions(&assets, &label);
        Weekly {
            timestamp: timestamp(),
            basis: week_basis(&["2026-04-20".into(), "2026-04-21".into()]),
            questions: infer_week_questions(&assets, &label, &tensions),
            label,
            assets,
            drivers,
            tensions,
            notes: vec![],
        }
    }

    fn earnings_hint_fixture(published_at: Option<&str>, freshness: &'static str) -> EarningsHint {
        EarningsHint {
            freshness,
            fields: EarningsFields::default(),
            source: ResearchSource {
                title: "Fixture earnings source".into(),
                publisher: "market-pulse fixture".into(),
                url: "fixture://earnings".into(),
                published_at: published_at.map(str::to_string),
                relevance: "earnings evidence".into(),
                evidence: "Fixture says EPS beat by $1 and revenue was huge".into(),
            },
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

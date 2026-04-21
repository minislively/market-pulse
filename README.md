# market-pulse

`market-pulse` is a Rust CLI for terminal-native market inquiry.

It turns rough market/economic questions into structured thinking: question breakdown, competing explanations, evidence checks, counter-views, and a better next question — without leaving the terminal.

```bash
mp "금리가 내려간 게 진짜 완화 기대 때문임?"
mp research "금리 하락이 성장주에 좋은 신호임?"
mp "대형 IPO 때문에 성장주가 강한 걸까?" --research
mp ask "대형 IPO 때문에 성장주가 강한 걸까?"
mp now
mp regime
mp think "금리가 부담인데도 반도체가 버티는 걸 보면 성장 기대가 아직 남아있는 것 같다"
mp review
mp review --date 2026-04-21
mp review --days 1
```

## North star

> A terminal-native market inquiry companion: ask rough market questions, pressure-test interpretations, and build market literacy through repeated exposure.

## What it is not

- Not a trading bot.
- Not investment advice.
- Not a buy/sell recommender.
- Not a price-target or stop-loss generator.
- Not a portfolio manager.
- Not a charting terminal.
- Not a backtesting engine.

The project is for market literacy and reasoning practice, not trading decisions.

## MVP commands

### `mp "question"`

Primary flow. Ask a rough market question directly:

```bash
mp "성장주는 강한데 달러랑 유가가 애매하면 리스크온이 맞나?"
```

Output includes:

- question breakdown
- possible explanations / thesis candidates
- evidence to check
- counter-view
- next better question
- explicit market-literacy boundary

### `mp ask "question"`

Explicit alias for the same inquiry flow:

```bash
mp ask "대형 IPO가 시장 상승 이유가 될 수 있음?"
```

### `mp research "question"` / `mp "question" --research`

Research mode keeps the source-backed inquiry contract without
making built-in network/news providers mandatory yet:

```bash
mp research "금리 하락이 성장주에 좋은 신호임?"
mp "대형 IPO 때문에 성장주가 강한 걸까?" --research
```

Output includes:

- provider/source metadata section
- clear no-provider fallback when no live source is configured
- distinction between source-backed material and inference scaffolding
- evidence against / counter-view
- data to check next
- next better question
- explicit market-literacy boundary

By default, research mode does not fetch RSS/SEC/news/API data over the network.
Built-in providers can be added behind the provider boundary later. For now,
use the search-command adapter kit to prototype provider wrappers outside the
core.

For an opt-in external search bridge, set `MARKET_PULSE_SEARCH_CMD` to a command
template that contains `{query}` and emits JSON Lines source rows. A runnable
provider-agnostic starter kit lives in [`adapters/search-command/`](adapters/search-command/):

```bash
MARKET_PULSE_SEARCH_CMD='./adapters/search-command/fixture-jsonl {query}' \
  mp "금리 하락이 성장주에 좋은 신호임?" --research
```

For opt-in live web source metadata through Brave Web Search, set a user-owned
API key and point the hook at the Brave wrapper:

```bash
export BRAVE_SEARCH_API_KEY='...'
MARKET_PULSE_SEARCH_CMD='./adapters/search-command/brave-jsonl {query}' \
  mp "달러 강세가 코스피에 부담임?" --research --no-save
```

The Brave wrapper also has a key-free fixture mode for deterministic smoke tests:

```bash
MARKET_PULSE_SEARCH_CMD='./adapters/search-command/brave-jsonl --fixture {query}' \
  mp "달러 강세가 코스피에 부담임?" --research --no-save
```

Each output line should be source metadata:

```json
{"title":"...", "publisher":"...", "url":"...", "evidence":"...", "relevance":"...", "published_at":"..."}
```

The hook is deliberately restricted: no shell execution, quote-aware argv
parsing, `{query}` substitution, 5 second timeout, and at most 20 JSONL source
rows. JSON string escapes such as `\"`, `\n`, and `\u2713` are decoded. If the
command fails, `mp` falls back to the normal inference scaffold instead of
crashing.

### `mp now`

Prints a compact market pulse card:

- market mood
- asset pressure
- likely tensions
- market puzzle / question seeds
- one concept to watch

Live quotes are fetched through the Yahoo Finance chart endpoint by shelling out to `curl`. If a quote fails, the card still renders so the learning loop is not blocked.

`mp now` is a session/daily pulse, not a weekly return screen. The timestamp and session label use the local machine clock, while each percentage move is calculated from Yahoo `regularMarketPrice` versus `chartPreviousClose` from `range=5d&interval=1d` — usually the prior regular-session close for that asset. Because assets trade in different time zones, treat the card as a cross-asset snapshot and use `$mp-research` when the exact session calendar matters.

### `mp regime`

Prints a broader 1-3 month market regime card:

- regime label / mood
- explicit 1-3 month basis
- cross-asset map
- regime drivers
- regime tensions
- checks for the next session/week
- next better regime question

`mp regime` is intentionally separate from `mp now`: `now` is a session/daily pulse, while `regime` is the larger backdrop traders compare against short-term moves. Date-based journal lookup now lives in `mp review --date YYYY-MM-DD` and the easier `mp review --days N`; broader search-style review remains a later phase.

### `mp think "..."`

Records your market interpretation and returns structured feedback:

- extracted claim
- thesis type
- what was good
- what needs evidence
- counter-view
- next observation question
- related concepts

### `mp review`

Reviews recent pulses, regimes, inquiries, thoughts, and feedback to surface recurring themes, question habits, and reasoning drills.

Use `--date YYYY-MM-DD` to review only entries recorded on a specific timestamp date, or `--days N` for the common “N days back” flow:

```bash
mp review --date 2026-04-21
mp review --days 1
```

`--limit N` can be combined with `--date` or `--days`; the limit is applied after date matching, keeping the most recent matching entries. `--ago N` and `--days-ago N` remain accepted as compatibility aliases, but `--days N` is the preferred spelling. The date filter is explicit by design. Natural language lookup such as `어제` or `지난주` remains a later search/review phase.


## OMX/Codex aliases

When the Codex/OMX skill adapter is installed, you can use readable `$mp-*`
aliases inside Codex/OMX sessions without changing the standalone shell CLI:

```text
$mp-now
$mp-regime
$mp-ask "대형 IPO 때문에 성장주가 강한 걸까?"
$mp-research "금리 하락이 성장주에 좋은 신호임?"
$mp-think "금리가 부담인데도 반도체가 버티는 것 같다"
$mp-review
$mp-review --date 2026-04-21
$mp-review --days 1
```

These aliases are thin wrappers around the same local `mp` binary:

- `$mp-now` -> `mp now`
- `$mp-regime` -> `mp regime`
- `$mp-ask` -> `mp ask`
- `$mp-research` -> `mp research`
- `$mp-think` -> `mp think`
- `$mp-review` -> `mp review`, `mp review --date YYYY-MM-DD`, or `mp review --days N`

The canonical `$mp ...` skill remains available for flexible calls. The aliases
are explicit only: they do not auto-capture arbitrary market/economy sentences,
and live/source-backed lookup still requires `$mp-research` or an existing
`mp ... --research` flow.

## Storage

By default, entries are written to:

```text
~/.local/share/market-pulse/journal.jsonl
```

Override with:

```bash
MARKET_PULSE_HOME=/tmp/mp mp "금리가 내려간 이유가 뭘까?"
```

Use `--no-save` for one-off inquiries:

```bash
mp ask "오늘 달러 강세가 왜 중요함?" --no-save
```

## Install locally

Prerequisites:

- Rust/Cargo
- `curl` on `PATH`

```bash
git clone https://github.com/minislively/market-pulse.git
cd market-pulse
cargo install --path . --force
mp "금리가 내려간 게 진짜 완화 기대 때문임?"
```

For local development:

```bash
make test
make smoke
```

## Adapters

The repo includes thin adapters that call the same standalone `mp` binary:

- Codex skill: `adapters/codex-skill/SKILL.md`
- Codex readable alias skills: `adapters/codex-skill/aliases/`
- Claude Code slash command: `adapters/claude-command/mp/COMMAND.md`

## Design

See:

- [`docs/vision.md`](docs/vision.md)
- [`docs/mvp.md`](docs/mvp.md)
- [`docs/architecture.md`](docs/architecture.md)

## License

MIT

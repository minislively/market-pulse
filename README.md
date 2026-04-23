# market-pulse

`market-pulse` is a Rust CLI for terminal-native market inquiry.

It turns rough market/economic questions into structured thinking: question breakdown, competing explanations, evidence checks, counter-views, and a better next question — without leaving the terminal.

```bash
mp "금리가 내려간 게 진짜 완화 기대 때문임?"
mp research "금리 하락이 성장주에 좋은 신호임?"
mp "대형 IPO 때문에 성장주가 강한 걸까?" --research
mp ask "대형 IPO 때문에 성장주가 강한 걸까?"
mp 오늘 시황
mp NVDA
mp NVDA 리서치
mp 전에 금리 찾아줘 --limit 3
mp 이번주 복기
mp now
mp watch
mp fomo
mp week
mp calendar
mp regime
mp think "금리가 부담인데도 반도체가 버티는 걸 보면 성장 기대가 아직 남아있는 것 같다"
mp review
mp review --date 2026-04-21
mp review --days 1
mp review --this-week
mp find "금리" --this-week
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

### Natural Korean/ticker aliases

The CLI includes a small deterministic router for common Korean daily-use
phrases and ticker/company/asset-like text. It is an alias layer, not a full
NLP engine:

```bash
mp 오늘 시황              # same current-market card as mp now
mp 지금 시장
mp 시장 펄스
mp 이번주                # same weekly card as mp week
mp 레짐                  # same broader backdrop card as mp regime
mp 캘린더                # same local calendar card as mp calendar
mp 오늘 복기             # normalized to mp review --today
mp 지난주 리뷰 --limit 5 # normalized to mp review --last-week --limit 5
mp 전에 금리 찾아줘      # normalized to mp find "금리"
mp 내 생각 나스닥은 너무 오른듯
mp NVDA                  # safe inquiry scaffold
mp 비트코인              # safe inquiry scaffold
mp NVDA 리서치           # source-aware research scaffold
mp 반도체 왜 오름?       # source-aware research scaffold
```

Explicit commands still win. For example, `mp now 리서치` remains `mp now`,
while `mp 오늘 시황 근거` and `mp 오늘 시황 --research` intentionally route to
research mode. Ticker/company/asset-like text by itself stays in the safe local
inquiry scaffold; source-aware output requires `research`, `--research`, or an
evidence/source marker such as `리서치`, `근거`, `출처`, `왜`, `뉴스`, `자료`, or
`확인`, `팩트체크`.

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
- Semis (`^SOX`) as a non-compact semiconductor index proxy, not a full AI basket
- likely tensions
- market puzzle / question seeds
- one concept to watch
- a six-line Daily Decision Checklist in non-compact output: scenario,
  confirm, falsify, watch, discipline, and journal prompt

Live quotes are fetched through the Yahoo Finance chart endpoint by shelling out to `curl`. If a quote fails, the card still renders so the learning loop is not blocked.

`mp now` is a close-to-close pulse, not a high/low gap, exact 24-hour, local calendar-day, or weekly return screen. The timestamp and session label use the local machine clock, while each percentage move is calculated from Yahoo `range=5d&interval=1d` daily closes: latest daily close value versus prior daily close. `regularMarketPrice` is fallback only when the close series is unavailable. Because US indices, Korea, FX, futures, and crypto use different clocks, treat the card as a cross-asset directional snapshot and use `$mp-research` when the exact exchange calendar matters.

The Daily Decision Checklist is an observation and journaling routine, not a
trade instruction. It helps name a scenario, define confirmation/falsification,
watch cross-asset relationships, and write a `mp think` note before forming a
market view. When `^SOX` data is available, the checklist may use Semis-led
scenario language; without that data it falls back to non-semis wording. `mp now
--compact` remains a one-line pulse and does not render the checklist.

### `mp watch`

Prints a Daily Radar card for the “what should I keep in mind today?” use case:

- current mood and quote/change basis
- scenario / watch / confirm / falsify lines derived from the same cross-asset pulse logic as `mp now`
- a short driver/tension summary
- a FOMO checkpoint prompt that separates evidence from opportunity-cost fear
- explicit boundary: reasoning support only, no trading instructions

`mp watch` saves a `radar` journal event unless `--no-save` is passed. The saved event is intentionally compact JSONL: timestamp, linked pulse timestamp, mood, scenario, confirm, falsify, watch, and prompt. It is designed to make `mp review` / `mp find` catch daily market context later without requiring a daemon, OS push notification, external messenger, paid API, or GUI.

### `mp fomo`

Prints a lightweight FOMO checkpoint for moments when the tape feels hard to ignore:

- latest saved radar and pulse timestamps when available
- latest scenario carried forward from `mp watch`
- pause questions: what exactly am I reacting to, what confirms it, what falsifies it, and is this evidence or opportunity-cost fear?
- a next journal prompt for `mp think`
- explicit boundary: reasoning support only, no trading instructions

`mp fomo` saves a `fomo_check` journal event unless `--no-save` is passed. If no prior radar exists, it still renders a useful pause card and suggests running `mp watch` for a fresh context card.

### `mp regime`

Prints a broader 1-3 month market regime card:

- regime label / mood
- explicit 1-3 month basis
- cross-asset map
- regime drivers
- regime tensions
- checks for the next session/week
- next better regime question

`mp regime` is intentionally separate from `mp now`: `now` is a close-to-close pulse, while `regime` is the larger backdrop traders compare against short-term moves. Regime market change uses Yahoo `range=3mo&interval=1wk` weekly closes: latest weekly close value versus the first available weekly close, with `regularMarketPrice` as fallback only. Date-based journal lookup now lives in `mp review --date YYYY-MM-DD`, the easier `mp review --days N`, and calendar aliases like `mp review --this-week`; broader search-style review remains a later phase.

### `mp week`

Prints a hybrid weekly market-and-learning card:

- current-week market story
- explicit market and journal basis
- 1W cross-asset map
- weekly market themes and tensions
- this week's saved pulse/regime/inquiry/thought counts
- recurring journal themes and thesis habits
- next-week check questions

`mp week` fills the gap between `mp now` and `mp regime`: it is not a separate daily command and not a 1-3 month regime read. The default window is current-date based: the journal review uses the current local calendar week, while the market change uses Yahoo `range=1mo&interval=1d` to find the first close matching the current local week before comparing it with the latest daily close value. `regularMarketPrice` is fallback only. If an asset has not traded during the current local week yet, the weekly card falls back to the latest available close instead of pretending a week-to-date move exists.

### `mp calendar`

Prints a compact market-calendar report. It still shows the local-date windows
market-pulse uses for journal review, but now adds deterministic curated
US/Korea equity exchange-calendar context so `mp now` / `mp week` close-based
readings are easier to interpret:

- today
- yesterday
- this-week
- last-week
- US equities (NYSE/Nasdaq) exchange-local context with 09:30-16:00 ET regular hours
- Korea equities (KRX/KOSPI) exchange-local context with 09:00-15:30 KST regular hours
- curated static coverage/freshness labels, including US 2026/2027 metadata
- NYSE/Nasdaq grouped-row source handling: full only when sources cover and agree; otherwise `source-limited`
- KRX partial-coverage wording when full year-specific Korean holiday coverage is not curated
- a calendar-to-pulse bridge for daily close-to-close and current-week close basis
- matching `mp review` shortcut commands
- explicit boundary that this is deterministic curated static context, not a live official exchange feed, live event calendar/news agenda, or trading signal

Official source anchors used for the curated static data are Nasdaq's holiday/trading-hours page, NYSE Group's 2026-2028 holiday/early-close release, the KRX trading guide, and the KRX Market Closing(Holiday) page. The CLI keeps labels concise; docs carry the full source/coverage limitations.

Source anchors:

- Nasdaq holiday/trading hours: https://www.nasdaq.com/market-activity/stock-market-holiday-schedule
- NYSE 2026/2027/2028 holiday + early close release: https://s2.q4cdn.com/154085107/files/doc_news/NYSE-Group-Announces-2026-2027-and-2028-Holiday-and-Early-Closings-Calendar-2025.pdf
- KRX trading guide: https://global.krx.co.kr/contents/GLB/01/0109/0109000000/guide_to_trading_in_the_korean_stock_market.pdf
- KRX Market Closing(Holiday): https://global.krx.co.kr/contents/GLB/05/0501/0501110000/GLB0501110000.jsp

Use it when you want to check both the journal date window and the equity-session
context behind the latest market pulse:

```bash
mp calendar
mp review --this-week
mp review --last-week
```

### `mp think "..."`

Records your market interpretation and returns structured feedback:

- extracted claim
- thesis type
- what was good
- what needs evidence
- counter-view
- next observation question
- related concepts

### `mp find "query"`

Searches your local market-pulse journal and renders a recall card, not just raw grep output:

- explicit query and date/window basis
- matching entries with timestamp/type snippets
- matched event counts
- recurring themes in matches
- next recall question
- local-journal-only boundary

Examples:

```bash
mp find "금리"
mp find "달러" --this-week
mp find "유가" --last-week
mp find "반도체" --days 3 --limit 5
```

`mp search` is accepted as a thin alias for `mp find`. A small deterministic
Korean recall alias layer also accepts phrases such as `mp 전에 금리 찾아줘` and
normalizes them to command-shaped `mp find` arguments. It does not parse
arbitrary natural-language dates, does not use exchange calendars, does not call
live web providers, and does not add semantic/vector search.

### `mp review`

Reviews recent pulses, radars, FOMO checkpoints, regimes, inquiries, thoughts, and feedback to surface recurring themes, question habits, and reasoning drills.

Use `--date YYYY-MM-DD` to review only entries recorded on a specific timestamp date, `--days N` for the common “N days back” flow, or small readable aliases for common calendar windows:

```bash
mp review --date 2026-04-21
mp review --days 1
mp review --today
mp review --yesterday
mp review --this-week
mp review --last-week
```

`--limit N` can be combined with one date/window selector; the limit is applied after matching, keeping the most recent matching entries. `--ago N` and `--days-ago N` remain accepted as compatibility aliases, but `--days N` is the preferred relative-day spelling. Use `mp calendar` when you want to inspect what `today`, `yesterday`, `this-week`, and `last-week` mean on the current local machine clock. The date filter is explicit by design. A small deterministic Korean review alias layer accepts common phrases such as `mp 오늘 복기`, `mp 어제 복기`, `mp 이번주 복기`, and `mp 지난주 리뷰`; broader natural-language review remains out of scope.


## OMX/Codex aliases

When the Codex/OMX skill adapter is installed, you can use readable `$mp-*`
aliases and canonical `$mp ...` subcommands inside Codex/OMX sessions without
changing the standalone shell CLI:

```text
$mp-now
$mp watch
$mp fomo
$mp-week
$mp-calendar
$mp-regime
$mp-ask "대형 IPO 때문에 성장주가 강한 걸까?"
$mp-research "금리 하락이 성장주에 좋은 신호임?"
$mp-think "금리가 부담인데도 반도체가 버티는 것 같다"
$mp-review
$mp-review --date 2026-04-21
$mp-review --days 1
$mp-review --this-week
$mp-find "금리" --this-week
```

These aliases are thin wrappers around the same local `mp` binary:

- `$mp-now` -> `mp now`
- `$mp watch` -> `mp watch`
- `$mp fomo` -> `mp fomo`
- `$mp-week` -> `mp week`
- `$mp-calendar` -> `mp calendar`
- `$mp-regime` -> `mp regime`
- `$mp-ask` -> `mp ask`
- `$mp-research` -> `mp research`
- `$mp-think` -> `mp think`
- `$mp-review` -> `mp review`, `mp review --date YYYY-MM-DD`, `mp review --days N`, or a small period alias such as `mp review --this-week`
- `$mp-find` -> `mp find`, optionally with the same small date/window selectors

The canonical `$mp ...` skill remains available for flexible calls and passes
Korean/ticker text through to the CLI's deterministic alias router. The named
`$mp-*` aliases are explicit wrappers, and live/source-backed lookup still
requires `$mp-research`, an existing `mp ... --research` flow, or an evidence
marker that the CLI router recognizes.

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

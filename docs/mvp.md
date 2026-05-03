# MVP

The MVP is intentionally small and implemented as a Rust CLI.

## Commands

### `mp "question"`

Acceptance criteria:

- Treats bare non-command arguments as a market inquiry.
- Generates a structured response with question breakdown, possible explanations, evidence checks, counter-view, next better question, and safety boundary.
- Saves an `inquiry` journal event unless `--no-save` is passed.
- Avoids trading advice, buy/sell recommendations, price targets, stop-loss, and portfolio instructions.

### `mp ask "question"`

Acceptance criteria:

- Runs the same inquiry flow as bare `mp "question"`.
- Supports `--no-save`.

### `mp research "question"` / `mp "question" --research`

Acceptance criteria:

- Routes both the explicit subcommand and `--research` flag to research mode.
- Uses a `ResearchProvider`-style boundary with deterministic no-provider/unavailable behavior when no provider is configured.
- Supports an opt-in `MARKET_PULSE_SEARCH_CMD` external command hook for JSONL source metadata.
- Executes the search hook without a shell, with quote-aware argv parsing, `{query}` substitution, a 5 second timeout, and at most 20 JSONL rows parsed.
- Renders `Research unavailable` plus inference scaffolding if the hook is unset, invalid, times out, exits non-zero, or returns no valid source rows; no-source output must not masquerade as source-backed research.
- Renders source metadata when available, including decoded JSON string escapes.
- Renders a clear no-provider/no-source fallback when no live provider is configured.
- Distinguishes source-backed material from inference scaffolding.
- Saves a `research_inquiry` journal event with provider/source metadata unless `--no-save` is passed.
- Avoids trading advice, buy/sell recommendations, price targets, stop-loss, and portfolio instructions.

### Natural Korean/ticker input router

Acceptance criteria:

- Runs only after explicit command/help/option routing, so existing subcommands remain authoritative.
- Preserves payload-bearing `Inquiry` and `Research` commands while using normalized command-shaped args for `Review`, `Find`, and `Think`.
- Routes current-market aliases such as `오늘 시황`, `지금 시장`, and `시장 펄스` to `mp now`.
- Routes period/context aliases such as `이번주`, `주간`, `레짐`, `국면`, and `캘린더` to the existing week/regime/calendar modes.
- Routes common review phrases to existing selectors: `오늘 복기`, `어제 복기`, `이번주 복기`, and `지난주 리뷰`.
- Routes recall phrases with remaining query text, such as `전에 금리 찾아줘`, to `mp find`.
- Routes thought/journal phrases with remaining text, such as `내 생각 ...` and `메모 ...`, to `mp think`.
- Routes explicit evidence/source intent such as `리서치`, `근거`, `출처`, `왜`, `확인`, `뉴스`, `자료`, `팩트체크`, or `--research` to research-backed inquiry.
- Keeps ticker/company/asset-like text such as `NVDA`, `BTC`, `비트코인`, `삼성전자`, and `반도체` in the safe inquiry scaffold unless evidence/source intent is present.
- Documents the feature as deterministic alias routing, not full natural-language understanding.
- Does not add dependencies, MCP integration, live provider changes, ticker detail screens, or trading advice.

### `mp now`

Acceptance criteria:

- Prints a compact market card.
- Includes mood, explicit quote/change basis, assets, drivers, tensions, question seeds / market puzzle prompts, and concept.
- Adds `Semis` (`^SOX`) to non-compact `mp now` as a pulse-only semiconductor index proxy; `mp week` and `mp regime` do not inherit it in this phase.
- In non-compact output, includes one six-line Daily Decision Checklist with Scenario, Confirm, Falsify, Watch, Discipline, and Journal lines.
- Renders the checklist as a market-literacy observation/journaling routine, not investment advice or trade instruction.
- Uses semis-led checklist wording only when `^SOX` data is available; otherwise falls back to non-semis wording.
- Preserves `mp now --compact` as a one-line output with no checklist section.
- Uses a close-to-close basis by default: local timestamp/session label plus latest Yahoo daily close value vs prior daily close from `range=5d&interval=1d`, with `regularMarketPrice` as fallback only; this is not a high/low gap, exact 24-hour, local calendar-day, or weekly return view.
- Saves a `pulse` journal event unless `--no-save` is passed.
- Does not block the loop if live quote fetch fails.

### `mp watch`

Acceptance criteria:

- Prints a Daily Radar card for same-day market context, distinct from `mp now` but derived from the same cross-asset pulse/checklist logic.
- Includes mood, basis, scenario, watch, confirm, falsify, compact drivers/tensions, and FOMO checkpoint prompts.
- Saves a `radar` journal event unless `--no-save` is passed.
- The `radar` event includes timestamp, linked pulse timestamp, mood, scenario, confirm, falsify, watch, and prompt fields.
- Avoids trading advice, external messaging, OS push, daemon/launchd, paid API, and GUI requirements in this MVP.

### `mp fomo`

Acceptance criteria:

- Prints a FOMO Checkpoint card for moments when the user wants to pause and separate evidence from opportunity-cost fear.
- Links to the latest saved `radar` and `pulse` timestamps when available.
- Carries forward the latest scenario/confirm/falsify/watch context when a prior radar exists.
- Falls back gracefully when no radar exists and suggests `mp watch` for fresh context.
- Saves a `fomo_check` journal event unless `--no-save` is passed.
- Avoids trading advice, external messaging, OS push, daemon/launchd, paid API, and GUI requirements in this MVP.

### `mp week`

Acceptance criteria:

- Prints a hybrid weekly market-and-learning card distinct from `mp now` and `mp regime`.
- Uses a current-week market basis by default: local timestamp plus latest Yahoo daily close value vs the first close matching the current local calendar week from `range=1mo&interval=1d`, with `regularMarketPrice` as fallback only and latest-close fallback if the asset has not traded this week.
- Scans the current local calendar week of JSONL journal entries before saving the weekly card.
- Includes weekly story, explicit basis, 1W asset map, weekly market themes, tensions, journal theme counts, thesis habits, next-week check questions, and a weekly drill.
- Saves a `week` journal event unless `--no-save` is passed.
- Avoids trading advice.

### `mp calendar`

Acceptance criteria:

- Prints a compact market-calendar report, not only local date shortcuts.
- Preserves local-date windows for `today`, `yesterday`, `this-week`, and `last-week`.
- Shows exchange-local context for US equities (NYSE/Nasdaq, 09:30-16:00 ET) and Korea equities (KRX/KOSPI, 09:00-15:30 KST).
- Uses deterministic curated static calendar rules: US coverage metadata includes 2026 and 2027; KRX coverage is explicit about full vs partial status.
- Handles NYSE/Nasdaq grouped-row source divergence by rendering `source-limited` instead of full official proof when source coverage is incomplete or disagrees.
- Renders KRX partial coverage as partial wording such as `regular session by partial KRX rules`; it must not imply full year-specific Korean holiday proof unless that data is curated.
- Includes source/freshness boundaries and a calendar-to-pulse bridge explaining that `mp now` is close-to-close daily context and `mp week` combines the local journal week with the first matching Yahoo daily close.
- Shows the matching `mp review` shortcut commands.
- Names the boundary that this is deterministic curated static context, not a live official exchange feed, live event/news calendar, or trading signal.
- Does not save a journal event.

Source anchors:

- Nasdaq holiday/trading hours: https://www.nasdaq.com/market-activity/stock-market-holiday-schedule
- NYSE 2026/2027/2028 holiday + early close release: https://s2.q4cdn.com/154085107/files/doc_news/NYSE-Group-Announces-2026-2027-and-2028-Holiday-and-Early-Closings-Calendar-2025.pdf
- KRX trading guide: https://global.krx.co.kr/contents/GLB/01/0109/0109000000/guide_to_trading_in_the_korean_stock_market.pdf
- KRX Market Closing(Holiday): https://global.krx.co.kr/contents/GLB/05/0501/0501110000/GLB0501110000.jsp

### `mp regime`

Acceptance criteria:

- Prints a broader market regime card distinct from `mp now`.
- Uses a 1-3 month basis by default: local timestamp plus latest Yahoo weekly close value vs first available weekly close from `range=3mo&interval=1wk`, with `regularMarketPrice` as fallback only.
- Includes regime label, explicit basis, cross-asset map, regime drivers, tensions, checks, and next better regime question.
- Saves a `regime` journal event unless `--no-save` is passed.
- Avoids trading advice.

### `mp think "..."`

Acceptance criteria:

- Records the user's text as a `thought` event.
- Generates structured feedback with claim, thesis type, good, check, counter-view, next questions, and concepts.
- Saves both thought and feedback unless `--no-save` is passed.
- Avoids trading advice.

### `mp find "query"`

Acceptance criteria:

- Searches local JSONL journal entries by explicit query.
- Supports `--date YYYY-MM-DD`, `--days N`, `--today`, `--yesterday`, `--this-week`, `--last-week`, and `--limit N`.
- Uses the same single-selector conflict rule as `mp review`.
- Renders a recall card with query, filter basis, matching entries, matched event counts including `radar` / `fomo_check`, recurring themes, next recall question, and a local-journal-only boundary.
- Shows a helpful empty result when no entries match.
- Accepts small deterministic Korean recall aliases only when query text remains.
- Avoids fuzzy natural-language date parsing, exchange calendars, live provider changes, semantic/vector search, and trading advice.

### `mp review`

Acceptance criteria:

- Reads recent JSONL journal events.
- Supports `--date YYYY-MM-DD` to filter by the timestamp date stored in journal entries.
- Supports preferred `--days N` plus compatibility `--ago N` / `--days-ago N` for common relative-day lookup without broad natural-language parsing.
- Supports small calendar aliases: `--today`, `--yesterday`, `--this-week`, and `--last-week`.
- Rejects multiple date/window selectors in the same review command.
- Applies `--limit N` after date/window matching when combined with a selector, keeping the most recent matching entries.
- Counts `radar` and `fomo_check` events alongside existing pulse/week/regime/inquiry/research/thought/feedback counts.
- Shows a distinct empty-date or empty-period message when no entries match the requested selector.
- Surfaces repeated themes, concepts, inquiry counts, and question/thesis habits.
- Suggests one reasoning drill.
- Accepts small deterministic Korean review aliases for today, yesterday, this week, and last week.

## Install target

`cargo install --path . --force` should provide both executable names:

- `mp`
- `market-pulse`

## Adapter kits

- `adapters/search-command/` provides a provider-agnostic fixture kit for
  `MARKET_PULSE_SEARCH_CMD` plus an optional Brave Web Search wrapper that
  emits the same JSONL source metadata contract. Standard smoke tests remain
  key-free; live Brave smoke is opt-in via `BRAVE_SEARCH_API_KEY`.

## Deferred

- TUI
- charts
- portfolio support
- backtesting
- built-in live RSS/SEC/news network providers
- built-in Brave/Tavily/NewsAPI/SerpApi/Alpha Vantage/FRED integrations
- article body storage or summarization
- external plugin loading
- paid data vendors

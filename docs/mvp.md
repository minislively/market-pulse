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

- Routes both the explicit subcommand and `--research` flag to research-backed inquiry mode.
- Uses a `ResearchProvider`-style boundary with deterministic noop behavior when no provider is configured.
- Supports an opt-in `MARKET_PULSE_SEARCH_CMD` external command hook for JSONL source metadata.
- Executes the search hook without a shell, with quote-aware argv parsing, `{query}` substitution, a 5 second timeout, and at most 20 JSONL rows parsed.
- Falls back to inference scaffolding if the hook is unset, invalid, times out, exits non-zero, or returns no valid source rows.
- Renders source metadata when available, including decoded JSON string escapes.
- Renders a clear no-provider/no-source fallback when no live provider is configured.
- Distinguishes source-backed material from inference scaffolding.
- Saves a `research_inquiry` journal event with provider/source metadata unless `--no-save` is passed.
- Avoids trading advice, buy/sell recommendations, price targets, stop-loss, and portfolio instructions.

### `mp now`

Acceptance criteria:

- Prints a compact market card.
- Includes mood, explicit quote/change basis, assets, drivers, tensions, question seeds / market puzzle prompts, and concept.
- Uses a close-to-close basis by default: local timestamp/session label plus latest Yahoo daily close value vs prior daily close from `range=5d&interval=1d`, with `regularMarketPrice` as fallback only; this is not a high/low gap, exact 24-hour, local calendar-day, or weekly return view.
- Saves a `pulse` journal event unless `--no-save` is passed.
- Does not block the loop if live quote fetch fails.

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

- Prints local-date windows for `today`, `yesterday`, `this-week`, and `last-week`.
- Shows the matching `mp review` shortcut commands.
- Names the boundary that these windows are local-date helpers, not exchange-holiday calendars or trading signals.
- Does not save a journal event.

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
- Renders a recall card with query, filter basis, matching entries, matched event counts, recurring themes, next recall question, and a local-journal-only boundary.
- Shows a helpful empty result when no entries match.
- Avoids fuzzy natural-language date parsing, exchange calendars, live provider changes, semantic/vector search, and trading advice.

### `mp review`

Acceptance criteria:

- Reads recent JSONL journal events.
- Supports `--date YYYY-MM-DD` to filter by the timestamp date stored in journal entries.
- Supports preferred `--days N` plus compatibility `--ago N` / `--days-ago N` for common relative-day lookup without broad natural-language parsing.
- Supports small calendar aliases: `--today`, `--yesterday`, `--this-week`, and `--last-week`.
- Rejects multiple date/window selectors in the same review command.
- Applies `--limit N` after date/window matching when combined with a selector, keeping the most recent matching entries.
- Shows a distinct empty-date or empty-period message when no entries match the requested selector.
- Surfaces repeated themes, concepts, inquiry counts, and question/thesis habits.
- Suggests one reasoning drill.

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

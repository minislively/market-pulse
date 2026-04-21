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
- Includes mood, assets, drivers, tensions, question seeds / market puzzle prompts, and concept.
- Saves a `pulse` journal event unless `--no-save` is passed.
- Does not block the loop if live quote fetch fails.

### `mp think "..."`

Acceptance criteria:

- Records the user's text as a `thought` event.
- Generates structured feedback with claim, thesis type, good, check, counter-view, next questions, and concepts.
- Saves both thought and feedback unless `--no-save` is passed.
- Avoids trading advice.

### `mp review`

Acceptance criteria:

- Reads recent JSONL journal events.
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

# search-command adapter kit

This directory is a provider-agnostic starter kit for `MARKET_PULSE_SEARCH_CMD`.
It does **not** fetch live market news or call a paid search API. Its job is to
show the smallest executable contract a future provider wrapper must satisfy.

## Contract

`market-pulse` research mode can call an external command when
`MARKET_PULSE_SEARCH_CMD` is set:

```bash
MARKET_PULSE_SEARCH_CMD='./adapters/search-command/fixture-jsonl {query}' \
  mp "달러 강세가 코스피에 부담임?" --research --no-save
```

The command receives the market question through `{query}` and writes JSON Lines
to stdout. Each non-empty line is one source metadata row.

Required string fields:

- `title`
- `publisher`
- `url`
- `evidence`
- `relevance`

Optional string field:

- `published_at`

Example row:

```json
{"title":"...","publisher":"...","url":"...","evidence":"...","relevance":"...","published_at":"..."}
```

`mp` owns the inquiry scaffold, counter-view, data-to-check prompts, next
question, journaling, and safety boundary. The external command only supplies
source metadata.

## Fixture command

`fixture-jsonl` is a small POSIX `sh` fixture that uses only standard `awk` for
JSON string escaping. It accepts a query and prints one demo source row:

```bash
./adapters/search-command/fixture-jsonl "달러 강세가 코스피에 부담임?"
```

Then test it through research mode:

```bash
MARKET_PULSE_SEARCH_CMD='./adapters/search-command/fixture-jsonl {query}' \
  cargo run --quiet --bin mp -- "달러 강세가 코스피에 부담임?" --research --no-save
```

After local install:

```bash
cargo install --path . --force
MARKET_PULSE_SEARCH_CMD='./adapters/search-command/fixture-jsonl {query}' \
  mp "달러 강세가 코스피에 부담임?" --research --no-save
```

## Brave Web Search wrapper

`brave-jsonl` is an optional provider-specific wrapper for Brave Web Search. It
keeps live API access outside the Rust core and emits the same JSONL source rows
as the fixture. It requires `ruby` on `PATH` and uses Ruby standard
libraries only; no Python, `jq`, or new Rust crate dependency is required.

Key-free local fixture smoke:

```bash
./adapters/search-command/brave-jsonl --fixture "달러 강세가 코스피에 부담임?"

MARKET_PULSE_SEARCH_CMD='./adapters/search-command/brave-jsonl --fixture {query}' \
  cargo run --quiet --bin mp -- "달러 강세가 코스피에 부담임?" --research --no-save
```

Live Brave smoke is opt-in and requires a user-owned API key:

```bash
export BRAVE_SEARCH_API_KEY='...'
MARKET_PULSE_SEARCH_CMD='./adapters/search-command/brave-jsonl {query}' \
  mp "달러 강세가 코스피에 부담임?" --research --no-save
```

Optional tuning environment variables:

- `BRAVE_SEARCH_COUNT` — result count, clamped to 1..20; default `5`
- `BRAVE_SEARCH_FRESHNESS` — Brave freshness filter such as `pd`, `pw`, `pm`,
  `py`, or a custom date range
- `BRAVE_SEARCH_COUNTRY` — Brave country code such as `US` or `KR`
- `BRAVE_SEARCH_LANG` — Brave search language such as `en` or `ko`
- `BRAVE_SEARCH_UI_LANG` — UI language such as `en-US` or `ko-KR`

If `BRAVE_SEARCH_API_KEY` is absent, direct wrapper execution exits non-zero and
prints setup guidance to stderr. When invoked through `mp`, wrapper stderr may be
suppressed by the command bridge, so debug setup by running `brave-jsonl`
directly. The wrapper keeps its network timeout below the core command timeout
(currently 1 second to connect and 2 seconds to read).

## Replacing or adding providers later

A real wrapper can be any executable that follows the same stdout contract. For
example, a future Tavily/SerpApi/FRED wrapper can:

1. read the query from argv;
2. call its provider using credentials owned by that wrapper;
3. normalize provider results into JSONL rows;
4. exit non-zero or return no valid rows on failure so `mp` falls back safely.

Keep Rust core provider registry/config work deferred until at least two
provider wrappers expose repeated configuration needs.

## Non-goals

- No built-in Rust RSS/news/search provider is included here.
- No API keys or credential files are read by the fixture. `brave-jsonl` reads
  only `BRAVE_SEARCH_API_KEY` from the environment for opt-in live mode.
- No Rust core provider registry/config is added here.
- No trading advice, buy/sell guidance, price targets, stop-loss, or portfolio
  instructions are generated here.

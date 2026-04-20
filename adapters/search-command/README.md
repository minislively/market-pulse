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

## Replacing the fixture with a real provider later

A real wrapper can be any executable that follows the same stdout contract. For
example, a future Brave/Tavily/SerpApi/FRED wrapper can:

1. read the query from argv;
2. call its provider using credentials owned by that wrapper;
3. normalize provider results into JSONL rows;
4. exit non-zero or return no valid rows on failure so `mp` falls back safely.

Keep real provider selection, credentials, rate limits, and network behavior out
of this fixture kit until they are planned explicitly.

## Non-goals

- No live RSS/news/search provider is included here.
- No API keys or credential files are read here.
- No Rust core provider registry/config is added here.
- No trading advice, buy/sell guidance, price targets, stop-loss, or portfolio
  instructions are generated here.

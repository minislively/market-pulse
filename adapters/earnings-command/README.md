# earnings-command adapter contract

`mp earnings` can reuse `MARKET_PULSE_SEARCH_CMD`, but earnings rows may include
optional typed fields. The Rust core renders these fields only when the external
adapter provides them explicitly; it never extracts EPS/revenue/guidance from
unstructured prose.

Required source metadata fields match the search-command contract:

- `title`
- `publisher`
- `url`
- `evidence`
- `relevance`

Optional source metadata:

- `published_at`

Optional structured earnings fields:

- `ticker`
- `company`
- `report_date`
- `timing`
- `eps_actual`
- `eps_estimate`
- `revenue_actual`
- `revenue_estimate`
- `surprise`
- `guidance`
- `price_reaction`

Example:

```json
{"title":"NVDA earnings","publisher":"fixture","url":"fixture://nvda","evidence":"fixture structured row","relevance":"earnings","published_at":"2026-04-28","ticker":"NVDA","company":"Nvidia","report_date":"2026-05-20","timing":"after close","eps_actual":"1.23","eps_estimate":"1.10","revenue_actual":"10B","revenue_estimate":"9B","surprise":"beat","guidance":"raised","price_reaction":"+4%"}
```

## Built-in local adapters

### `fixture-jsonl`

Deterministic fixture adapter for tests and examples:

```bash
./adapters/earnings-command/fixture-jsonl "recent major US earnings results EPS revenue guidance stock reaction source"
MARKET_PULSE_SEARCH_CMD='./adapters/earnings-command/fixture-jsonl {query}' \
  mp earnings --no-save
```

### `yahoo-jsonl`

```bash
./adapters/earnings-command/yahoo-jsonl [--fixture] <query...>
```

- `--fixture`: deterministic, zero-network, exits `0`, and emits one typed JSONL
  row suitable for smoke tests.
- live mode: fetches the public Yahoo Finance earnings calendar page and emits a
  page-level source row. This is unofficial, key-free, best-effort, incomplete,
  and not trading advice.
- exit codes: `0` success / zero-row success, `1` usage or local adapter bug,
  `2` live source skipped or unavailable.

Examples:

```bash
./adapters/earnings-command/yahoo-jsonl --fixture \
  "upcoming major US earnings this week next week calendar radar source"
MARKET_PULSE_SEARCH_CMD='./adapters/earnings-command/yahoo-jsonl --fixture {query}' \
  mp earnings --no-save
MARKET_PULSE_SEARCH_CMD='./adapters/earnings-command/yahoo-jsonl {query}' \
  mp earnings --no-save
```

Smoke targets:

```bash
make smoke-earnings       # deterministic only; no live Yahoo call
make smoke-earnings-live  # skips unless MARKET_PULSE_LIVE_EARNINGS=1
```

Non-goals: no paid API requirement, no scraping in Rust core, no provider/env
registry, no complete earnings calendar guarantee, and no trading advice.

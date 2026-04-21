# Architecture

`market-pulse` should remain CLI-first and hookable, not a large harness.

The core is a dependency-light Rust binary so it can be invoked reliably from Codex, Claude, OMX, tmux, shell hooks, or long-running task wait loops without depending on a fragile Python environment.

## Layers

```text
environment adapters -> CLI -> inquiry/feedback pipeline -> JSONL memory
```

Current MVP implementation:

```text
Codex/Claude adapters -> mp Rust binary -> quote + local inquiry/research lenses -> JSONL journal
                         \-> optional MARKET_PULSE_SEARCH_CMD JSONL source hook
```

## Hook families

Initial code may keep these simple, but the boundaries should stay visible:

- source hooks: quotes, RSS, macro data, filings
- research provider hooks: noop fallback now, optional
  `MARKET_PULSE_SEARCH_CMD` JSONL bridge, `adapters/search-command/` fixture
  kit, RSS/search/API providers later
- lens hooks: risk-on/off, rates vs growth, dollar liquidity, Korea market, AI/semis, IPO/event, positioning/flow
- inquiry hooks: question breakdown, possible explanations, evidence checks, counter-view, next better question
- research inquiry hooks: source metadata, source-backed claims, no-source fallback, data-to-check prompts
- weekly hooks: current-local-week market story, current-week journal summary, next-week check questions
- feedback hooks: claim extraction, thesis typing, evidence checks, counter-view, next question, concept linking
- renderer hooks: terminal, compact, markdown, JSON
- memory hooks: JSONL first, SQLite later if needed

## Environment adapters

Adapters should call the standalone `mp` CLI instead of making the core depend on a specific runtime.

Planned or current adapters:

- Codex skill: `$mp <question>`, `$mp ask ...`, `$mp research ...`, `$mp now`, `$mp week`, `$mp regime`, `$mp think ...`, `$mp review`
- Claude Code slash command: `/mp <question>`, `/mp ask ...`, `/mp research ...`, `/mp now`, `/mp week`, `/mp regime`, `/mp think ...`, `/mp review`
- OMX hook: optional compact pulse at session start or long task completion
- tmux popup/status: optional compact pulse
- cron/launchd: optional scheduled snapshots

## Dependency stance

The MVP intentionally avoids Rust crate dependencies. It uses the standard library plus a `curl` subprocess for quote fetches. Research mode proves the provider boundary with deterministic noop behavior and an opt-in external command bridge rather than making network providers mandatory. Add HTTP/JSON crates only when source reliability, parsing complexity, or provider abstraction becomes the actual bottleneck.

`MARKET_PULSE_SEARCH_CMD` is a restricted bridge for local search tooling: the template must contain `{query}`, it is parsed into argv with simple quote-aware grouping, executed without a shell, capped by a 5 second timeout, and limited to the first 20 non-empty JSONL rows. JSON string escapes are decoded before rendering.

The external command supplies source metadata only; market-pulse keeps ownership
of rendering, counter-views, data-to-check prompts, next questions, journaling,
and safety boundaries. The current `adapters/search-command/` kit keeps the
provider-agnostic fixture and adds an optional Brave Web Search wrapper;
built-in provider selection and Rust core registry work remain deferred.

Built-in RSS/SEC/news fetches, paid data vendors, article body storage, and background daemons are outside the current MVP and require explicit confirmation before implementation.

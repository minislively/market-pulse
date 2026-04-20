# Architecture

`market-pulse` should remain CLI-first and hookable, not a large harness.

The core is a dependency-light Rust binary so it can be invoked reliably from Codex, Claude, OMX, tmux, shell hooks, or long-running task wait loops without depending on a fragile Python environment.

## Layers

```text
environment adapters -> CLI -> core pipeline -> hooks -> memory
```

Current MVP implementation:

```text
Codex/Claude adapters -> mp Rust binary -> quote + feedback pipeline -> JSONL journal
```

## Hook families

Initial code may keep these simple, but the boundaries should stay visible:

- source hooks: quotes, RSS, macro data, filings
- lens hooks: risk-on/off, rates vs growth, dollar liquidity, Korea market, AI/semis
- feedback hooks: claim extraction, evidence checks, counter-view, next question, concept linking
- renderer hooks: terminal, compact, markdown, JSON
- memory hooks: JSONL first, SQLite later if needed

## Environment adapters

Adapters should call the standalone `mp` CLI instead of making the core depend on a specific runtime.

Planned or current adapters:

- Codex skill: `$mp now`, `$mp think ...`, `$mp review`
- Claude Code slash command: `/mp now`, `/mp think ...`, `/mp review`
- OMX hook: optional compact pulse at session start or long task completion
- tmux popup/status: optional compact pulse
- cron/launchd: optional scheduled snapshots

## Dependency stance

The MVP intentionally avoids Rust crate dependencies. It uses the standard library plus a `curl` subprocess for quote fetches. Add HTTP/JSON crates only when source reliability, parsing complexity, or provider abstraction becomes the actual bottleneck.

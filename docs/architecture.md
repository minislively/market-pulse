# Architecture

`market-pulse` should remain CLI-first and hookable, not a large harness.

## Layers

```text
environment adapters -> CLI -> core pipeline -> hooks -> memory
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

Planned adapters:

- Codex skill: `$mp now`, `$mp think ...`, `$mp review`
- OMX hook: optional compact pulse at session start or long task completion
- tmux popup/status: optional compact pulse
- cron/launchd: optional scheduled snapshots

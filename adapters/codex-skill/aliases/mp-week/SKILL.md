---
name: mp-week
description: "Run market-pulse week mode for a hybrid weekly market-and-learning card. Use when the user says $mp-week or wants this week's market pulse plus their question/thought review."
argument-hint: ""
---

Use the local `mp` CLI. Keep this alias thin: do not reimplement market-pulse in the prompt.

## Command

- `$mp-week` -> run `mp week`

This alias takes no required argument. It prints a weekly learning card that combines a current-local-week market view, this week's saved market-pulse journal themes, and next-week check questions. If extra text is provided, prefer `$mp-ask` or `$mp-research` instead of guessing.

## Example CLI call

```bash
mp week
```

## Safety / Product Boundary

SOURCE: Keep this block textually consistent with `/Users/veluga/.codex/skills/mp/SKILL.md` `## Rules`.

- Never provide direct buy/sell recommendations, price targets, stop-loss, or portfolio instructions.
- Frame output as market literacy and reasoning practice.
- Prefer: question breakdown, possible explanations, evidence checks, counter-views, next better questions.
- In research mode, preserve source metadata/no-provider fallback and distinguish source-backed material from inference scaffolding.
- If `MARKET_PULSE_SEARCH_CMD` is configured, let `mp` use it as the restricted JSONL source bridge; do not reimplement search in the skill prompt.
- Avoid: sensational headlines, numbers-only dashboards, false certainty.

## Non-goals

- Do not auto-capture arbitrary market/economy sentences; only run this alias when explicitly invoked.
- Do not make live/API search automatic. Research/source lookup remains explicit through `$mp-research` or existing `mp ... --research` flows.
- Do not add trading advice or portfolio guidance.

## Fallback

If the `mp` executable is missing, install the Rust CLI locally:

```bash
cd ~/dev/market-pulse
cargo install --path . --force
```

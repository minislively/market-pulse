---
name: mp-think
description: "Run market-pulse think mode to record a user market interpretation and return structured feedback. Use when the user says $mp-think <text>."
argument-hint: "<text>"
---

Use the local `mp` CLI. Keep this alias thin: do not reimplement market-pulse in the prompt.

## Command

- `$mp-think <text>` -> run `mp think "<text>"`

If the user invokes `$mp-think` without text, show a short usage hint and ask for the thought/interpretation; do not invent one.

## Example CLI call

```bash
mp think "금리가 부담인데도 반도체가 버티는 걸 보면 성장 기대가 아직 남아있는 것 같다"
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

---
name: mp
description: "Run market-pulse CLI commands for terminal-native market inquiry, research-backed inquiry scaffolds, market context, user-thought feedback, and review. Use when the user says $mp, mp <question>, mp ask, mp research, mp now, mp think, mp review, 오늘 시황, 시장 펄스, or asks for market-thinking feedback."
argument-hint: "[question]|ask [question]|research [question]|now|think|review [text]"
---

Use the local `mp` CLI. Keep this skill thin: do not reimplement market-pulse in the prompt.

## Commands

- `$mp <question>` -> run `mp "<question>"`
- `$mp ask <question>` -> run `mp ask "<question>"`
- `$mp research <question>` -> run `mp research "<question>"`
- `$mp <question> --research` -> run `mp "<question>" --research`
- `$mp now` -> run `mp now`
- `$mp think <text>` -> run `mp think "<text>"`
- `$mp review` -> run `mp review`

If the user invokes `$mp` without arguments, treat it as `$mp now`.

## Rules

- Never provide direct buy/sell recommendations, price targets, stop-loss, or portfolio instructions.
- Frame output as market literacy and reasoning practice.
- Prefer: question breakdown, possible explanations, evidence checks, counter-views, next better questions.
- In research mode, preserve source metadata/no-provider fallback and distinguish source-backed material from inference scaffolding.
- Avoid: sensational headlines, numbers-only dashboards, false certainty.

## Fallback

If the `mp` executable is missing, install the Rust CLI locally:

```bash
cd ~/dev/market-pulse
cargo install --path . --force
```

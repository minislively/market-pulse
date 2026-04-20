---
name: mp
description: "Run market-pulse CLI commands for terminal-native market context, user-thought feedback, and review. Use when the user says $mp, mp now, mp think, mp review, 오늘 시황, 시장 펄스, or asks for market-thinking feedback."
argument-hint: "now|think|review [text]"
---

Use the local `mp` CLI. Keep this skill thin: do not reimplement market-pulse in the prompt.

## Commands

- `$mp now` -> run `mp now`
- `$mp think <text>` -> run `mp think <text>`
- `$mp review` -> run `mp review`

If the user invokes `$mp` without arguments, treat it as `$mp now`.

## Rules

- Never provide direct buy/sell recommendations, price targets, or portfolio instructions.
- Frame output as market literacy and reasoning practice.
- Prefer: mood, drivers, tensions, counter-views, next observation questions.
- Avoid: sensational headlines, numbers-only dashboards, false certainty.

## Fallback

If the `mp` executable is missing, tell the user to install the repo locally:

```bash
python -m pip install -e .
```

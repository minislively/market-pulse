---
name: mp
description: "Run market-pulse CLI commands for terminal-native market inquiry, research-backed inquiry scaffolds, market context, daily radar/FOMO checkpoints, weekly learning, calendar windows, local journal recall, user-thought feedback, and review. Use when the user says $mp, mp <question>, mp ask, mp research, mp now, mp watch, mp fomo, mp week, mp calendar, mp regime, mp think, mp review, mp find, 오늘 시황, 시장 펄스, 이번주 복기, 전에 금리 찾아줘, NVDA 리서치, 포모, or asks for market-thinking feedback."
argument-hint: "[question]|ask [question]|research [question]|now|watch|fomo|week|calendar|regime|think|review [selector]|find [query]"
---

Use the local `mp` CLI. Keep this skill thin: do not reimplement market-pulse in the prompt.

## Commands

- `$mp <question>` -> run `mp "<question>"`
- `$mp 오늘 시황` / `$mp 시장 펄스` -> pass through to `mp` for deterministic natural routing
- `$mp NVDA` / `$mp 비트코인` -> pass through to the safe inquiry scaffold
- `$mp NVDA 리서치` / `$mp 반도체 왜 오름?` -> pass through to research intent routing
- `$mp 오늘 복기` / `$mp 지난주 리뷰` -> pass through to review selector routing
- `$mp 전에 금리 찾아줘` -> pass through to find routing
- `$mp 내 생각 ...` / `$mp 메모 ...` -> pass through to think routing
- `$mp ask <question>` -> run `mp ask "<question>"`
- `$mp research <question>` -> run `mp research "<question>"`
- `$mp <question> --research` -> run `mp "<question>" --research`
- `$mp now` -> run `mp now`
- `$mp watch` -> run `mp watch`
- `$mp fomo` -> run `mp fomo`
- `$mp week` -> run `mp week`
- `$mp calendar` -> run `mp calendar`
- `$mp regime` -> run `mp regime`
- `$mp think <text>` -> run `mp think "<text>"`
- `$mp review` -> run `mp review`
- `$mp review --date YYYY-MM-DD` -> run `mp review --date YYYY-MM-DD`
- `$mp review --days N` -> run `mp review --days N`
- `$mp review --today|--yesterday|--this-week|--last-week` -> run the same `mp review` selector
- `$mp find <query> [selector]` -> run `mp find "<query>" [selector]`

Readable alias skills are also available when installed from `adapters/codex-skill/aliases/`:

- `$mp-now` -> run `mp now`
- `$mp-week` -> run `mp week`
- `$mp-calendar` -> run `mp calendar`
- `$mp-regime` -> run `mp regime`
- `$mp-ask <question>` -> run `mp ask "<question>"`
- `$mp-research <question>` -> run `mp research "<question>"`
- `$mp-think <text>` -> run `mp think "<text>"`
- `$mp-review` -> run `mp review`, `mp review --date YYYY-MM-DD`, `mp review --days N`, or a small period selector such as `mp review --this-week` when supplied
- `$mp-find <query>` -> run `mp find "<query>"` with optional date/window selectors

If the user invokes `$mp` without arguments, treat it as `$mp now`.

## Rules

- Never provide direct buy/sell recommendations, price targets, stop-loss, or portfolio instructions.
- Frame output as market literacy and reasoning practice.
- Prefer: question breakdown, possible explanations, evidence checks, counter-views, next better questions.
- For `watch` / `fomo`, preserve the terminal-only daily radar and decision-hygiene framing; do not add external notifications or trading instructions in the prompt.
- In research mode, preserve source metadata/no-provider fallback and distinguish source-backed material from inference scaffolding.
- If `MARKET_PULSE_SEARCH_CMD` is configured, let `mp` use it as the restricted JSONL source bridge; do not reimplement search in the skill prompt.
- Keep natural Korean/ticker handling as a deterministic CLI alias layer; pass text to `mp` rather than classifying it in this prompt.
- Avoid: sensational headlines, numbers-only dashboards, false certainty.

## Fallback

If the `mp` executable is missing, install the Rust CLI locally:

```bash
cd ~/dev/market-pulse
cargo install --path . --force
```

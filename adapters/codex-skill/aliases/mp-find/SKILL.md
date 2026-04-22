---
name: mp-find
description: "Run market-pulse find mode for local journal recall search. Use when the user says $mp-find <query> or wants to find prior market-pulse notes."
argument-hint: "<query> [--date YYYY-MM-DD|--days N|--today|--yesterday|--this-week|--last-week]"
---

Use the local `mp` CLI. Keep this alias thin: do not reimplement market-pulse in the prompt.

## Command

- `$mp-find <query>` -> run `mp find "<query>"`
- `$mp-find <query> --this-week` -> run `mp find "<query>" --this-week`
- `$mp-find <query> --last-week` -> run `mp find "<query>" --last-week`

If the user invokes `$mp-find` without a query, show a short usage hint and ask for the missing journal search keyword; do not invent one.

## Example CLI call

```bash
mp find "금리" --this-week
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
- Do not make live/API search automatic. `mp find` searches the local journal only.
- Do not add trading advice or portfolio guidance.
- Do not treat `--this-week` / `--last-week` as exchange-calendar semantics.
- Do not turn the alias into fuzzy natural-language date parsing or semantic/vector search.

## Fallback

If the `mp` executable is missing, install the Rust CLI locally:

```bash
cd ~/dev/market-pulse
cargo install --path . --force
```

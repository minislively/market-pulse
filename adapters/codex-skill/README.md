# Codex skill adapter

This adapter exposes `market-pulse` as thin Codex/OMX skills that call the local
Rust `mp` CLI. The skill prompts should not reimplement market-pulse logic.

## Install

First install the Rust CLI:

```bash
cd ~/dev/market-pulse
cargo install --path . --force
```

Install the canonical `$mp` skill:

```bash
mkdir -p ~/.codex/skills/mp
cp adapters/codex-skill/SKILL.md ~/.codex/skills/mp/SKILL.md
```

Install the readable alias skills:

```bash
for name in mp-now mp-week mp-calendar mp-regime mp-ask mp-research mp-think mp-review; do
  mkdir -p "$HOME/.codex/skills/$name"
  cp "adapters/codex-skill/aliases/$name/SKILL.md" "$HOME/.codex/skills/$name/SKILL.md"
done
```

You may need to restart or reload the Codex/OMX session for newly installed skill
names to appear in the session skill list.

## Commands

```text
$mp "금리가 내려간 게 진짜 완화 기대 때문임?"
$mp now
$mp week
$mp calendar
$mp regime
$mp ask "대형 IPO 때문에 성장주가 강한 걸까?"
$mp research "금리 하락이 성장주에 좋은 신호임?"
$mp think "금리가 부담인데도 반도체가 버티는 것 같다"
$mp review
$mp review --date 2026-04-21
$mp review --days 1
$mp review --this-week

$mp-now
$mp-week
$mp-calendar
$mp-regime
$mp-ask "대형 IPO 때문에 성장주가 강한 걸까?"
$mp-research "금리 하락이 성장주에 좋은 신호임?"
$mp-think "금리가 부담인데도 반도체가 버티는 것 같다"
$mp-review
$mp-review --date 2026-04-21
$mp-review --days 1
$mp-review --this-week
```

## Contract

- `$mp-*` aliases are explicit only; they do not auto-capture arbitrary market or economy text.
- `$mp-research` preserves the existing `MARKET_PULSE_SEARCH_CMD` behavior and no-provider fallback.
- All skills keep the market-literacy boundary: no buy/sell recommendations, price targets, stop-loss, portfolio instructions, sensational headlines, or false certainty.

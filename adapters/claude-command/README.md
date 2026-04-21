# Claude Code adapter

This adapter exposes `market-pulse` as a Claude Code slash command.

## Install

First install the Rust CLI:

```bash
cd ~/dev/market-pulse
cargo install --path . --force
```

Then copy or symlink the `mp` command directory into Claude's commands folder:

```bash
mkdir -p ~/.claude/commands
ln -sfn "$(pwd)/adapters/claude-command/mp" ~/.claude/commands/mp
```

Then in Claude Code:

```text
/mp 금리가 내려간 게 진짜 완화 기대 때문임?
/mp ask 대형 IPO 때문에 성장주가 강한 걸까?
/mp research 금리 하락이 성장주에 좋은 신호임?
/mp now
/mp regime
/mp think 금리가 부담인데도 반도체가 버티는 것 같다
/mp review
/mp review --date 2026-04-21
```

## Contract

The Claude command stays thin. It should call the standalone `mp` CLI and should not reimplement market-pulse logic in the prompt.

# Claude Code adapter

This adapter exposes `market-pulse` as a Claude Code slash command.

## Install

Copy or symlink the `mp` command directory into Claude's commands folder:

```bash
mkdir -p ~/.claude/commands
ln -sfn "$(pwd)/adapters/claude-command/mp" ~/.claude/commands/mp
```

Then in Claude Code:

```text
/mp now
/mp think 금리가 부담인데도 반도체가 버티는 것 같다
/mp review
```

## Contract

The Claude command stays thin. It should call the standalone `mp` CLI and should not reimplement market-pulse logic in the prompt.

# market-pulse

`market-pulse` is a CLI-first market thinking journal.

It helps you read short market context, write your own interpretation, get structured feedback, and review your reasoning over time — without leaving the terminal.

```bash
mp now
mp think "금리가 부담인데도 반도체가 버티는 걸 보면 성장 기대가 아직 남아있는 것 같다"
mp review
```

## North star

> A terminal-native market thinking gym: see the market, write your interpretation, get counter-views, and review your reasoning loops.

## What it is not

- Not a trading bot.
- Not investment advice.
- Not a portfolio manager.
- Not a charting terminal.
- Not a backtesting engine.

The project is for market literacy and reasoning practice, not buy/sell decisions.

## MVP commands

### `mp now`

Prints a compact market pulse card:

- market mood
- asset pressure
- likely tensions
- one observation question
- one concept to watch

### `mp think "..."`

Records your market interpretation and returns structured feedback:

- extracted claim
- what was good
- what needs evidence
- counter-view
- next observation question
- related concepts

### `mp review`

Reviews recent pulses, thoughts, and feedback to surface recurring themes and reasoning habits.

## Storage

By default, entries are written to:

```text
~/.local/share/market-pulse/journal.jsonl
```

Override with:

```bash
MARKET_PULSE_HOME=/tmp/mp mp now
```

## Install locally

```bash
python -m pip install -e .
mp now
```

## Design

See:

- [`docs/vision.md`](docs/vision.md)
- [`docs/mvp.md`](docs/mvp.md)
- [`docs/architecture.md`](docs/architecture.md)

## License

MIT

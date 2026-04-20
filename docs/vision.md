# Vision

`market-pulse` exists to turn terminal wait time into market reasoning practice.

The user context is simple: while AI/coding tasks run, it is easy to drift into phone scrolling. `market-pulse` keeps the loop in the terminal: ask a rough market question, get competing explanations and evidence checks, pressure-test your own interpretation, and review accumulated reasoning later.

## North star

Market literacy through repeated, low-friction, terminal-native inquiry.

## Core loop

```text
rough question -> breakdown -> competing explanations -> evidence checks -> counter-view -> next better question -> journal -> review
```

`mp now` and `mp think` support the loop, but the primary surface is:

```bash
mp "시장 질문"
mp ask "시장 질문"
mp research "시장 질문"
mp "시장 질문" --research
```

Research mode keeps the same learning loop, but adds an explicit source boundary:
show sources when a provider supplies them, otherwise say clearly that the answer
is inference scaffolding and list what data should be checked next. The current
bridge is intentionally opt-in: `MARKET_PULSE_SEARCH_CMD` can attach local search
tooling that emits JSONL source metadata, while the core remains CLI-first and
provider-agnostic.

## Boundaries

`market-pulse` should not become:

- a trading bot
- a buy/sell recommender
- a price-target or stop-loss assistant
- a portfolio tracker
- a charting-first terminal
- a backtesting platform
- a background news-alert daemon
- a default paid-news/API client

Those may be adjacent tools, but this project is a market inquiry and thinking companion.

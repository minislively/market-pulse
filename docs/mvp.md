# MVP

The MVP is intentionally small and implemented as a Rust CLI.

## Commands

### `mp now`

Acceptance criteria:

- Prints a compact market card.
- Includes mood, assets, drivers, tensions, question, and concept.
- Saves a `pulse` journal event unless `--no-save` is passed.
- Does not block the loop if live quote fetch fails.

### `mp think "..."`

Acceptance criteria:

- Records the user's text as a `thought` event.
- Generates structured feedback with claim, good, check, counter-view, next questions, and concepts.
- Saves both thought and feedback unless `--no-save` is passed.
- Avoids trading advice.

### `mp review`

Acceptance criteria:

- Reads recent JSONL journal events.
- Surfaces repeated themes and concepts.
- Suggests one reasoning drill.

## Install target

`cargo install --path . --force` should provide both executable names:

- `mp`
- `market-pulse`

## Deferred

- TUI
- charts
- portfolio support
- backtesting
- external plugin loading
- paid data vendors

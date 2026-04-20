from __future__ import annotations

import json
import os
from pathlib import Path
from typing import Any, Iterable

DEFAULT_HOME = Path.home() / ".local" / "share" / "market-pulse"


def market_pulse_home() -> Path:
    override = os.environ.get("MARKET_PULSE_HOME")
    return Path(override).expanduser() if override else DEFAULT_HOME


def journal_path() -> Path:
    return market_pulse_home() / "journal.jsonl"


def append_event(event: dict[str, Any]) -> Path:
    path = journal_path()
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("a", encoding="utf-8") as f:
        f.write(json.dumps(event, ensure_ascii=False, sort_keys=True) + "\n")
    return path


def iter_events(limit: int | None = None) -> Iterable[dict[str, Any]]:
    path = journal_path()
    if not path.exists():
        return []
    lines = path.read_text(encoding="utf-8").splitlines()
    if limit is not None:
        lines = lines[-limit:]
    events: list[dict[str, Any]] = []
    for line in lines:
        if not line.strip():
            continue
        try:
            events.append(json.loads(line))
        except json.JSONDecodeError:
            continue
    return events


def latest_event(event_type: str) -> dict[str, Any] | None:
    for event in reversed(list(iter_events())):
        if event.get("type") == event_type:
            return event
    return None

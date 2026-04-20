from __future__ import annotations

from dataclasses import asdict, dataclass, field
from datetime import datetime
from typing import Any, Literal

EventType = Literal["pulse", "thought", "feedback"]


@dataclass(slots=True)
class AssetMove:
    symbol: str
    label: str
    value: float | None = None
    change_percent: float | None = None
    unit: str = ""
    note: str = ""

    def pressure(self) -> str:
        if self.change_percent is None:
            return "unknown"
        if self.change_percent > 0.35:
            return "up"
        if self.change_percent < -0.35:
            return "down"
        return "flat"


@dataclass(slots=True)
class Pulse:
    timestamp: str
    session: str
    mood: str
    assets: list[AssetMove]
    drivers: list[str]
    tensions: list[str]
    question: str
    concept: str
    source_notes: list[str] = field(default_factory=list)

    def to_dict(self) -> dict[str, Any]:
        data = asdict(self)
        data["type"] = "pulse"
        return data


@dataclass(slots=True)
class Feedback:
    timestamp: str
    thought: str
    linked_pulse_timestamp: str | None
    claim: str
    good: list[str]
    check: list[str]
    counter_view: list[str]
    next_questions: list[str]
    concepts: list[str]

    def to_dict(self) -> dict[str, Any]:
        data = asdict(self)
        data["type"] = "feedback"
        return data


@dataclass(slots=True)
class Thought:
    timestamp: str
    text: str
    linked_pulse_timestamp: str | None

    def to_dict(self) -> dict[str, Any]:
        data = asdict(self)
        data["type"] = "thought"
        return data


def iso_now() -> str:
    return datetime.now().astimezone().isoformat(timespec="seconds")

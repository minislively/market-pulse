from __future__ import annotations

from .models import Feedback, Pulse


def _pct(value: float | None) -> str:
    if value is None:
        return "n/a"
    sign = "+" if value > 0 else ""
    return f"{sign}{value:.2f}%"


def render_pulse(pulse: Pulse, *, compact: bool = False) -> str:
    if compact:
        drivers = "; ".join(pulse.drivers[:2]) if pulse.drivers else "drivers unknown"
        return f"[mp] {pulse.mood} · {drivers} · Q: {pulse.question}"

    lines: list[str] = []
    lines.append(f"Market Pulse · {pulse.timestamp} · {pulse.session}")
    lines.append("")
    lines.append("Mood")
    lines.append(f"  {pulse.mood}")
    lines.append("")
    lines.append("Assets")
    for asset in pulse.assets:
        suffix = f" {asset.unit}" if asset.unit and asset.value is not None else ""
        value = "n/a" if asset.value is None else f"{asset.value:.2f}{suffix}"
        note = f" · {asset.note}" if asset.note else ""
        lines.append(f"  - {asset.label}: {value} ({_pct(asset.change_percent)}){note}")
    lines.append("")
    lines.append("Drivers")
    for idx, driver in enumerate(pulse.drivers, 1):
        lines.append(f"  {idx}. {driver}")
    lines.append("")
    lines.append("Tensions")
    for tension in pulse.tensions:
        lines.append(f"  - {tension}")
    lines.append("")
    lines.append("Question")
    lines.append(f"  {pulse.question}")
    lines.append("")
    lines.append("Concept")
    lines.append(f"  {pulse.concept}")
    if pulse.source_notes:
        lines.append("")
        lines.append("Source notes")
        for note in pulse.source_notes:
            lines.append(f"  - {note}")
    return "\n".join(lines)


def render_feedback(feedback: Feedback) -> str:
    lines: list[str] = []
    lines.append(f"Feedback · {feedback.timestamp}")
    lines.append("")
    lines.append("Claim")
    lines.append(f"  {feedback.claim}")
    lines.append("")
    lines.append("Good")
    for item in feedback.good:
        lines.append(f"  - {item}")
    lines.append("")
    lines.append("Check")
    for item in feedback.check:
        lines.append(f"  - {item}")
    lines.append("")
    lines.append("Counter-view")
    for item in feedback.counter_view:
        lines.append(f"  - {item}")
    lines.append("")
    lines.append("Next questions")
    for item in feedback.next_questions:
        lines.append(f"  - {item}")
    lines.append("")
    lines.append("Concepts")
    lines.append("  " + ", ".join(feedback.concepts))
    return "\n".join(lines)

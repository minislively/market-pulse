from __future__ import annotations

from collections import Counter
from typing import Any

from .feedback import detect_tags
from .journal import iter_events, journal_path


def render_review(limit: int = 80) -> str:
    events = list(iter_events(limit=limit))
    if not events:
        return "No market-pulse journal entries yet. Start with `mp now`, then `mp think \"...\"`."

    thoughts = [event for event in events if event.get("type") == "thought"]
    pulses = [event for event in events if event.get("type") == "pulse"]
    feedback = [event for event in events if event.get("type") == "feedback"]
    tag_counts: Counter[str] = Counter()
    concept_counts: Counter[str] = Counter()

    for thought in thoughts:
        tag_counts.update(detect_tags(str(thought.get("text", ""))))
    for item in feedback:
        concept_counts.update(str(concept) for concept in item.get("concepts", []))

    lines: list[str] = []
    lines.append("Market Pulse Review")
    lines.append("")
    lines.append(f"Journal: {journal_path()}")
    lines.append(f"Entries scanned: {len(events)} · pulses {len(pulses)} · thoughts {len(thoughts)} · feedback {len(feedback)}")
    lines.append("")
    lines.append("Repeated themes")
    if tag_counts:
        for tag, count in tag_counts.most_common(6):
            lines.append(f"  - {tag}: {count}")
    else:
        lines.append("  - Not enough tagged thoughts yet")
    lines.append("")
    lines.append("Concepts to revisit")
    if concept_counts:
        for concept, count in concept_counts.most_common(6):
            lines.append(f"  - {concept}: {count}")
    else:
        lines.append("  - Add a few `mp think` entries first")
    lines.append("")
    lines.append("Suggested drill")
    lines.append("  For the next 3 notes, explicitly separate:")
    lines.append("  1. market-wide signal")
    lines.append("  2. sector-specific signal")
    lines.append("  3. alternative explanation")
    return "\n".join(lines)

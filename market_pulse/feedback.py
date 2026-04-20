from __future__ import annotations

import re

from .journal import latest_event
from .models import Feedback, Thought, iso_now

KEYWORDS = {
    "rates": ["금리", "yield", "rate", "yields"],
    "semis": ["반도체", "semiconductor", "semis", "ai", "AI", "엔비디아", "nvidia"],
    "fx": ["달러", "환율", "원화", "usd", "dollar", "krw"],
    "oil": ["유가", "oil", "wti", "원유"],
    "korea": ["한국", "코스피", "kospi", "korea"],
    "crypto": ["코인", "비트", "btc", "crypto"],
}


def build_thought(text: str) -> Thought:
    latest_pulse = latest_event("pulse")
    return Thought(
        timestamp=iso_now(),
        text=text.strip(),
        linked_pulse_timestamp=latest_pulse.get("timestamp") if latest_pulse else None,
    )


def generate_feedback(text: str) -> Feedback:
    latest_pulse = latest_event("pulse")
    tags = detect_tags(text)
    claim = extract_claim(text, tags)
    good = build_good(tags)
    check = build_checks(tags)
    counter = build_counter_views(tags)
    next_questions = build_next_questions(tags)
    concepts = build_concepts(tags)
    return Feedback(
        timestamp=iso_now(),
        thought=text.strip(),
        linked_pulse_timestamp=latest_pulse.get("timestamp") if latest_pulse else None,
        claim=claim,
        good=good,
        check=check,
        counter_view=counter,
        next_questions=next_questions,
        concepts=concepts,
    )


def detect_tags(text: str) -> set[str]:
    lowered = text.lower()
    tags: set[str] = set()
    for tag, needles in KEYWORDS.items():
        if any(needle.lower() in lowered for needle in needles):
            tags.add(tag)
    return tags


def extract_claim(text: str, tags: set[str]) -> str:
    cleaned = re.sub(r"\s+", " ", text.strip())
    if not cleaned:
        return "No claim provided yet. Write one market interpretation to train against."
    if tags:
        return f"You are linking {', '.join(sorted(tags))} to a market interpretation: “{cleaned}”"
    return f"You are making a market interpretation that needs evidence: “{cleaned}”"


def build_good(tags: set[str]) -> list[str]:
    items = ["You wrote an explicit interpretation instead of only consuming market noise."]
    if "rates" in tags and "semis" in tags:
        items.append("You separated macro pressure from sector/growth resilience, which is a useful market lens.")
    if "fx" in tags and "korea" in tags:
        items.append("You connected FX pressure with Korea/EM risk, which is an important cross-market habit.")
    if len(tags) >= 2:
        items.append("You are already comparing more than one driver instead of forcing a single-cause story.")
    return items


def build_checks(tags: set[str]) -> list[str]:
    checks = ["Name the observable data that would confirm or reject this view."]
    if "semis" in tags:
        checks.append("Check whether semiconductor strength is broad or concentrated in a few mega-cap names.")
    if "rates" in tags:
        checks.append("Check whether yields moved before or after the equity reaction; timing matters.")
    if "fx" in tags:
        checks.append("Check whether USD strength is broad DXY strength or mostly a KRW/local move.")
    if "korea" in tags:
        checks.append("Separate KOSPI index direction from sector leadership and foreign flow if available.")
    if len(checks) == 1:
        checks.append("Avoid broad claims until at least two assets or events point in the same direction.")
    return checks


def build_counter_views(tags: set[str]) -> list[str]:
    counters = []
    if "semis" in tags:
        counters.append("Semis strength may be positioning, earnings expectations, or mega-cap concentration rather than broad growth optimism.")
    if "rates" in tags:
        counters.append("Rate pressure may be background noise if earnings revisions or liquidity are dominating the session.")
    if "fx" in tags:
        counters.append("FX pressure can matter less when local sector leadership or global risk appetite is strong enough.")
    if not counters:
        counters.append("The same price action may come from positioning, liquidity, news timing, or sector rotation; keep at least one alternative open.")
    return counters


def build_next_questions(tags: set[str]) -> list[str]:
    questions = []
    if "rates" in tags and "semis" in tags:
        questions.append("If yields rise further, do semis still outperform the broad market?")
    if "fx" in tags and "korea" in tags:
        questions.append("Does KRW weakness coincide with foreign selling, or are exporters offsetting the pressure?")
    if "oil" in tags:
        questions.append("Is oil moving enough to change inflation expectations, or is it only a sector input today?")
    if not questions:
        questions.append("What would you need to see by the close to say this interpretation was wrong?")
    return questions


def build_concepts(tags: set[str]) -> list[str]:
    concepts: list[str] = []
    mapping = {
        "rates": "rates vs growth",
        "semis": "sector leadership",
        "fx": "dollar liquidity",
        "oil": "inflation impulse",
        "korea": "EM/Korea risk transmission",
        "crypto": "high-beta risk appetite",
    }
    for tag in sorted(tags):
        concepts.append(mapping[tag])
    if not concepts:
        concepts.append("risk-on / risk-off")
    return concepts

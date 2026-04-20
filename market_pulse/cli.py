from __future__ import annotations

import argparse
import sys

from .feedback import build_thought, generate_feedback
from .journal import append_event
from .pulse import compose_pulse, fetch_yahoo_assets
from .render import render_feedback, render_pulse
from .review import render_review


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="mp", description="CLI-first market thinking journal")
    sub = parser.add_subparsers(dest="command")

    now = sub.add_parser("now", help="show the current market pulse")
    now.add_argument("--compact", action="store_true", help="print one compact line")
    now.add_argument("--no-save", action="store_true", help="do not write this pulse to the journal")

    think = sub.add_parser("think", help="write a market thought and get feedback")
    think.add_argument("text", nargs="+", help="your market interpretation")
    think.add_argument("--no-save", action="store_true", help="do not write thought/feedback to the journal")

    review = sub.add_parser("review", help="review recent thoughts and feedback")
    review.add_argument("--limit", type=int, default=80, help="number of recent journal events to scan")

    return parser


def cmd_now(args: argparse.Namespace) -> int:
    assets, notes = fetch_yahoo_assets()
    pulse = compose_pulse(assets, notes)
    if not args.no_save:
        append_event(pulse.to_dict())
    print(render_pulse(pulse, compact=args.compact))
    return 0


def cmd_think(args: argparse.Namespace) -> int:
    text = " ".join(args.text).strip()
    thought = build_thought(text)
    feedback = generate_feedback(text)
    if not args.no_save:
        append_event(thought.to_dict())
        append_event(feedback.to_dict())
    print(render_feedback(feedback))
    return 0


def cmd_review(args: argparse.Namespace) -> int:
    print(render_review(limit=args.limit))
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    if args.command is None:
        args = parser.parse_args(["now", *(argv or [])])
    if args.command == "now":
        return cmd_now(args)
    if args.command == "think":
        return cmd_think(args)
    if args.command == "review":
        return cmd_review(args)
    parser.print_help(sys.stderr)
    return 2

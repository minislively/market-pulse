import os
import tempfile
import unittest
from unittest.mock import patch

from market_pulse.journal import append_event, iter_events
from market_pulse.review import render_review


class JournalReviewTests(unittest.TestCase):
    def test_journal_round_trip(self):
        with tempfile.TemporaryDirectory() as tmp:
            with patch.dict(os.environ, {"MARKET_PULSE_HOME": tmp}):
                append_event({"type": "thought", "text": "달러 강세와 한국 시장"})
                events = list(iter_events())
                review = render_review()
        self.assertEqual(events, [{"type": "thought", "text": "달러 강세와 한국 시장"}])
        self.assertIn("fx", review)
        self.assertIn("korea", review)


if __name__ == "__main__":
    unittest.main()

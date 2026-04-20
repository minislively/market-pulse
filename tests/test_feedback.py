import os
import tempfile
import unittest
from unittest.mock import patch

from market_pulse.feedback import detect_tags, generate_feedback


class FeedbackTests(unittest.TestCase):
    def test_detect_tags_korean_market_terms(self):
        tags = detect_tags("금리가 부담인데 반도체가 버티고 달러도 강하다")
        self.assertTrue({"rates", "semis", "fx"}.issubset(tags))

    def test_feedback_includes_counter_view_for_semis_and_rates(self):
        with tempfile.TemporaryDirectory() as tmp:
            with patch.dict(os.environ, {"MARKET_PULSE_HOME": tmp}):
                feedback = generate_feedback("금리가 부담인데도 반도체가 버티는 것 같다")
        self.assertIn("rates", feedback.claim)
        self.assertTrue(any("Semis strength" in item for item in feedback.counter_view))
        self.assertTrue(any("yields" in item.lower() for item in feedback.check))


if __name__ == "__main__":
    unittest.main()

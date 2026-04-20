import unittest
from datetime import datetime
from zoneinfo import ZoneInfo

from market_pulse.models import AssetMove
from market_pulse.pulse import compose_pulse, session_for_now


class PulseTests(unittest.TestCase):
    def test_session_for_korea_open(self):
        now = datetime(2026, 4, 20, 10, 0, tzinfo=ZoneInfo("Asia/Seoul"))
        self.assertEqual(session_for_now(now), "Korea open")

    def test_compose_pulse_mentions_rates_tension(self):
        pulse = compose_pulse([
            AssetMove("^IXIC", "Nasdaq", 100, -0.8),
            AssetMove("^TNX", "US 10Y", 4.8, 1.1, "%"),
            AssetMove("DX-Y.NYB", "DXY", 105, 0.3),
        ])
        self.assertIn(pulse.mood, {"risk-off / macro pressure", "mixed / needs confirmation"})
        self.assertTrue(any("rates" in tension for tension in pulse.tensions))
        self.assertTrue(pulse.question)


if __name__ == "__main__":
    unittest.main()

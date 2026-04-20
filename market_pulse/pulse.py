from __future__ import annotations

import json
import math
import urllib.parse
import urllib.request
from datetime import datetime
from zoneinfo import ZoneInfo

from .models import AssetMove, Pulse, iso_now

DEFAULT_SYMBOLS: list[tuple[str, str, str]] = [
    ("^GSPC", "S&P 500", ""),
    ("^IXIC", "Nasdaq", ""),
    ("^KS11", "KOSPI", ""),
    ("KRW=X", "USD/KRW", "KRW"),
    ("DX-Y.NYB", "DXY", ""),
    ("^TNX", "US 10Y", "%"),
    ("CL=F", "WTI", "USD"),
    ("GC=F", "Gold", "USD"),
    ("BTC-USD", "BTC", "USD"),
]


def session_for_now(now: datetime | None = None) -> str:
    dt = now or datetime.now(ZoneInfo("Asia/Seoul"))
    hour = dt.hour
    if 6 <= hour < 9:
        return "Korea morning / US close handoff"
    if 9 <= hour < 12:
        return "Korea open"
    if 12 <= hour < 15:
        return "Asia midday"
    if 15 <= hour < 18:
        return "Korea close"
    if 18 <= hour < 22:
        return "US pre-open"
    return "US session / global watch"


def fetch_yahoo_assets(timeout: float = 4.0) -> tuple[list[AssetMove], list[str]]:
    assets: list[AssetMove] = []
    failures: list[str] = []
    for symbol, label, unit in DEFAULT_SYMBOLS:
        asset, error = fetch_yahoo_chart_asset(symbol, label, unit, timeout=timeout)
        assets.append(asset)
        if error:
            failures.append(error)
    notes = ["market quotes from Yahoo Finance chart endpoint"]
    if failures:
        notes.append(f"{len(failures)} quote(s) unavailable; kept the learning loop alive")
    return assets, notes


def fetch_yahoo_chart_asset(symbol: str, label: str, unit: str, timeout: float = 4.0) -> tuple[AssetMove, str | None]:
    encoded = urllib.parse.quote(symbol, safe="")
    url = f"https://query1.finance.yahoo.com/v8/finance/chart/{encoded}?range=5d&interval=1d"
    req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0 market-pulse/0.1"})
    try:
        with urllib.request.urlopen(req, timeout=timeout) as response:
            payload = json.loads(response.read().decode("utf-8"))
        result = payload.get("chart", {}).get("result", [])[0]
        meta = result.get("meta", {})
        quote = result.get("indicators", {}).get("quote", [{}])[0]
        closes = [value for value in quote.get("close", []) if _clean_number(value) is not None]
        value = _clean_number(meta.get("regularMarketPrice"))
        if value is None and closes:
            value = float(closes[-1])
        previous = _clean_number(meta.get("chartPreviousClose"))
        if previous is None and len(closes) >= 2:
            previous = float(closes[-2])
        change_percent = None
        if value is not None and previous not in (None, 0):
            change_percent = ((value - float(previous)) / float(previous)) * 100
        return AssetMove(symbol=symbol, label=label, value=value, change_percent=change_percent, unit=unit), None
    except Exception as exc:
        return AssetMove(symbol=symbol, label=label, value=None, change_percent=None, unit=unit, note="live data unavailable"), f"{symbol}: {exc.__class__.__name__}"


def _clean_number(value: object) -> float | None:
    if isinstance(value, (int, float)) and math.isfinite(float(value)):
        return float(value)
    return None


def fallback_assets() -> list[AssetMove]:
    return [
        AssetMove("^GSPC", "S&P 500", None, None, note="live data unavailable"),
        AssetMove("^IXIC", "Nasdaq", None, None, note="live data unavailable"),
        AssetMove("^KS11", "KOSPI", None, None, note="live data unavailable"),
        AssetMove("KRW=X", "USD/KRW", None, None, "KRW", "live data unavailable"),
        AssetMove("^TNX", "US 10Y", None, None, "%", "live data unavailable"),
    ]


def compose_pulse(assets: list[AssetMove], source_notes: list[str] | None = None) -> Pulse:
    lookup = {asset.symbol: asset for asset in assets}
    equities = [lookup.get("^GSPC"), lookup.get("^IXIC"), lookup.get("^KS11")]
    equity_changes = [a.change_percent for a in equities if a and a.change_percent is not None]
    avg_equity = sum(equity_changes) / len(equity_changes) if equity_changes else 0.0
    usd = lookup.get("DX-Y.NYB")
    us10y = lookup.get("^TNX")
    wti = lookup.get("CL=F")
    btc = lookup.get("BTC-USD")

    mood = infer_mood(avg_equity, usd.change_percent if usd else None, us10y.change_percent if us10y else None, btc.change_percent if btc else None)
    drivers = infer_drivers(assets)
    tensions = infer_tensions(assets)
    question = infer_question(tensions, mood)
    concept = infer_concept(tensions, drivers)
    return Pulse(
        timestamp=iso_now(),
        session=session_for_now(),
        mood=mood,
        assets=assets,
        drivers=drivers,
        tensions=tensions,
        question=question,
        concept=concept,
        source_notes=source_notes or [],
    )


def infer_mood(avg_equity: float, usd_pct: float | None, rates_pct: float | None, btc_pct: float | None) -> str:
    pressure = 0
    if avg_equity < -0.35:
        pressure -= 1
    elif avg_equity > 0.35:
        pressure += 1
    if usd_pct is not None and usd_pct > 0.25:
        pressure -= 1
    if rates_pct is not None and rates_pct > 0.5:
        pressure -= 1
    if btc_pct is not None and btc_pct > 1.0:
        pressure += 1
    if pressure >= 2:
        return "risk-on / growth-friendly"
    if pressure <= -2:
        return "risk-off / macro pressure"
    return "mixed / needs confirmation"


def infer_drivers(assets: list[AssetMove]) -> list[str]:
    drivers: list[str] = []
    for asset in assets:
        if asset.change_percent is None:
            continue
        if asset.symbol == "^TNX" and abs(asset.change_percent) > 0.5:
            direction = "rising" if asset.change_percent > 0 else "falling"
            drivers.append(f"US 10Y yield is {direction}, so rate pressure matters")
        elif asset.symbol in {"KRW=X", "DX-Y.NYB"} and abs(asset.change_percent) > 0.25:
            direction = "stronger" if asset.change_percent > 0 else "softer"
            drivers.append(f"Dollar/FX is {direction}, watch cross-market pressure")
        elif asset.symbol in {"^GSPC", "^IXIC", "^KS11"} and abs(asset.change_percent) > 0.6:
            direction = "higher" if asset.change_percent > 0 else "lower"
            drivers.append(f"{asset.label} is {direction}, check whether this is broad or sector-led")
        elif asset.symbol == "CL=F" and abs(asset.change_percent) > 1.0:
            direction = "higher" if asset.change_percent > 0 else "lower"
            drivers.append(f"Oil is {direction}, inflation and margin narratives may matter")
    if not drivers:
        drivers.append("No single asset is dominating; compare cross-asset confirmation")
        drivers.append("Use the next note to separate market-wide signal from sector-specific noise")
    return drivers[:4]


def infer_tensions(assets: list[AssetMove]) -> list[str]:
    lookup = {asset.symbol: asset for asset in assets}
    tensions: list[str] = []
    nasdaq = lookup.get("^IXIC")
    rates = lookup.get("^TNX")
    dollar = lookup.get("DX-Y.NYB") or lookup.get("KRW=X")
    kospi = lookup.get("^KS11")
    oil = lookup.get("CL=F")

    if rates and rates.change_percent is not None and nasdaq and nasdaq.change_percent is not None:
        tensions.append("rates pressure vs growth/tech resilience")
    if dollar and dollar.change_percent is not None and kospi and kospi.change_percent is not None:
        tensions.append("USD strength vs Korea/EM risk appetite")
    if oil and oil.change_percent is not None:
        tensions.append("oil/inflation pressure vs earnings optimism")
    if not tensions:
        tensions.append("macro signal vs sector-specific leadership")
    return tensions[:3]


def infer_question(tensions: list[str], mood: str) -> str:
    if any("rates" in tension for tension in tensions):
        return "Is the market trading rate pressure or earnings/growth hope?"
    if any("USD" in tension for tension in tensions):
        return "Is FX pressure driving risk appetite, or is it just background noise?"
    if "risk-off" in mood:
        return "Which asset confirms the risk-off signal, and which asset disagrees?"
    return "What is the strongest cross-asset confirmation, and what is the main contradiction?"


def infer_concept(tensions: list[str], drivers: list[str]) -> str:
    text = " ".join(tensions + drivers).lower()
    if "rates" in text or "yield" in text:
        return "rates vs growth"
    if "dollar" in text or "fx" in text or "usd" in text:
        return "dollar liquidity"
    if "oil" in text or "inflation" in text:
        return "inflation impulse"
    return "risk-on / risk-off"

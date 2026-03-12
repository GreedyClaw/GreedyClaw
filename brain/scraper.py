"""Web scraping for market data — Forex Factory, Investing.com, news sites."""

import httpx
from bs4 import BeautifulSoup
from datetime import datetime, timezone
from dataclasses import dataclass


@dataclass
class NewsItem:
    title: str
    source: str
    time: str
    impact: str = ""  # high, medium, low
    actual: str = ""
    forecast: str = ""
    previous: str = ""


@dataclass
class MarketContext:
    timestamp: str
    news: list[NewsItem]
    sentiment_summary: str


HEADERS = {
    "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"
}


async def fetch_forex_factory_calendar() -> list[NewsItem]:
    """Fetch today's economic events from Forex Factory."""
    items = []
    try:
        async with httpx.AsyncClient(timeout=15, headers=HEADERS) as client:
            resp = await client.get("https://www.forexfactory.com/calendar?day=today")
            if resp.status_code != 200:
                return items
            soup = BeautifulSoup(resp.text, "html.parser")
            rows = soup.select("tr.calendar__row")
            for row in rows[:20]:
                title_el = row.select_one(".calendar__event-title")
                impact_el = row.select_one(".calendar__impact span")
                actual_el = row.select_one(".calendar__actual")
                forecast_el = row.select_one(".calendar__forecast")
                previous_el = row.select_one(".calendar__previous")
                time_el = row.select_one(".calendar__time")
                if title_el:
                    impact_class = impact_el.get("class", [""]) if impact_el else [""]
                    impact = "high" if "high" in str(impact_class) else \
                             "medium" if "medium" in str(impact_class) else "low"
                    items.append(NewsItem(
                        title=title_el.get_text(strip=True),
                        source="forex_factory",
                        time=time_el.get_text(strip=True) if time_el else "",
                        impact=impact,
                        actual=actual_el.get_text(strip=True) if actual_el else "",
                        forecast=forecast_el.get_text(strip=True) if forecast_el else "",
                        previous=previous_el.get_text(strip=True) if previous_el else "",
                    ))
    except Exception as e:
        items.append(NewsItem(
            title=f"[Forex Factory unavailable: {e}]",
            source="forex_factory",
            time=datetime.now(timezone.utc).strftime("%H:%M"),
        ))
    return items


async def fetch_investing_com_news(symbol: str = "gold") -> list[NewsItem]:
    """Fetch recent news headlines from Investing.com."""
    items = []
    slug_map = {"XAUUSD": "gold", "EURUSD": "eur-usd", "GBPUSD": "gbp-usd", "BTCUSDT": "bitcoin"}
    slug = slug_map.get(symbol.upper(), symbol.lower())
    try:
        async with httpx.AsyncClient(timeout=15, headers=HEADERS) as client:
            resp = await client.get(f"https://www.investing.com/commodities/{slug}-news")
            if resp.status_code != 200:
                return items
            soup = BeautifulSoup(resp.text, "html.parser")
            articles = soup.select("article a[data-test='article-title-link']")
            for a in articles[:10]:
                items.append(NewsItem(
                    title=a.get_text(strip=True),
                    source="investing_com",
                    time=datetime.now(timezone.utc).strftime("%H:%M"),
                ))
    except Exception as e:
        items.append(NewsItem(
            title=f"[Investing.com unavailable: {e}]",
            source="investing_com",
            time=datetime.now(timezone.utc).strftime("%H:%M"),
        ))
    return items


async def fetch_generic_news(url: str, source_name: str) -> list[NewsItem]:
    """Fetch and extract text from any URL using trafilatura."""
    items = []
    try:
        import trafilatura
        async with httpx.AsyncClient(timeout=15, headers=HEADERS) as client:
            resp = await client.get(url)
            if resp.status_code == 200:
                text = trafilatura.extract(resp.text, include_comments=False)
                if text:
                    # Split into headline-sized chunks
                    for line in text.split("\n")[:10]:
                        line = line.strip()
                        if len(line) > 20:
                            items.append(NewsItem(
                                title=line[:200],
                                source=source_name,
                                time=datetime.now(timezone.utc).strftime("%H:%M"),
                            ))
    except Exception as e:
        items.append(NewsItem(
            title=f"[{source_name} unavailable: {e}]",
            source=source_name,
            time=datetime.now(timezone.utc).strftime("%H:%M"),
        ))
    return items


async def gather_market_context(symbols: list[str], sources: list[str]) -> MarketContext:
    """Gather all market data into a single context object."""
    all_news: list[NewsItem] = []

    if "forex_factory" in sources:
        all_news.extend(await fetch_forex_factory_calendar())

    if "investing_com" in sources:
        for sym in symbols:
            all_news.extend(await fetch_investing_com_news(sym))

    return MarketContext(
        timestamp=datetime.now(timezone.utc).isoformat(),
        news=all_news,
        sentiment_summary=_summarize_news(all_news),
    )


def _summarize_news(news: list[NewsItem]) -> str:
    if not news:
        return "No news data available."
    high_impact = [n for n in news if n.impact == "high"]
    lines = []
    if high_impact:
        lines.append(f"HIGH IMPACT events today: {len(high_impact)}")
        for n in high_impact[:5]:
            parts = [n.title]
            if n.actual:
                parts.append(f"actual={n.actual}")
            if n.forecast:
                parts.append(f"forecast={n.forecast}")
            lines.append(f"  - {' | '.join(parts)}")
    lines.append(f"Total news items: {len(news)}")
    return "\n".join(lines)

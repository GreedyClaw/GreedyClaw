"""Tests for brain/memory.py — SessionLog and DecisionLog."""

import json
import sys
import tempfile
from pathlib import Path

# Ensure brain package is importable
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from memory import SessionLog, DecisionLog


class TestSessionLog:
    """Test append-only JSONL session transcript."""

    def test_write_and_read_back(self):
        with tempfile.TemporaryDirectory() as tmp:
            log = SessionLog(Path(tmp))

            log.append("user", "What is the price of gold?")
            log.append("assistant", "Let me check the current price.")
            log.append("tool", "get_price result", tool_name="get_price")

            # Read back the JSONL file
            lines = log.path.read_text(encoding="utf-8").strip().split("\n")
            assert len(lines) == 3

            entry0 = json.loads(lines[0])
            assert entry0["role"] == "user"
            assert entry0["content"] == "What is the price of gold?"
            assert "ts" in entry0

            entry1 = json.loads(lines[1])
            assert entry1["role"] == "assistant"

            entry2 = json.loads(lines[2])
            assert entry2["role"] == "tool"
            assert entry2["tool_name"] == "get_price"

    def test_session_id_format(self):
        with tempfile.TemporaryDirectory() as tmp:
            log = SessionLog(Path(tmp))
            # Session ID should be YYYYMMDD_HHMMSS format
            assert len(log.session_id) == 15
            assert log.session_id[8] == "_"

    def test_empty_session(self):
        with tempfile.TemporaryDirectory() as tmp:
            log = SessionLog(Path(tmp))
            # File should not exist if nothing was written
            assert not log.path.exists()


class TestDecisionLog:
    """Test structured trading decision log."""

    def test_write_and_read_back(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "decisions.jsonl"
            log = DecisionLog(path)

            log.log_decision(
                symbol="XAUUSD",
                action="buy",
                confidence=85.0,
                reasoning="Gold breaking resistance with strong volume",
                sources=["https://tradingview.com", "https://forexfactory.com"],
                result={"filled": True, "price": 2650.50},
            )

            entries = log.recent(10)
            assert len(entries) == 1

            entry = entries[0]
            assert entry["symbol"] == "XAUUSD"
            assert entry["action"] == "buy"
            assert entry["confidence"] == 85.0
            assert "resistance" in entry["reasoning"]
            assert len(entry["sources"]) == 2
            assert entry["result"]["filled"] is True
            assert "ts" in entry

    def test_multiple_decisions(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "decisions.jsonl"
            log = DecisionLog(path)

            for i in range(5):
                log.log_decision(
                    symbol=f"SYM{i}",
                    action="hold",
                    confidence=50.0 + i * 10,
                    reasoning=f"Decision {i}",
                    sources=[],
                )

            # recent(3) should return only last 3
            entries = log.recent(3)
            assert len(entries) == 3
            assert entries[0]["symbol"] == "SYM2"
            assert entries[2]["symbol"] == "SYM4"

    def test_recent_on_nonexistent_file(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "nonexistent.jsonl"
            log = DecisionLog(path)
            entries = log.recent(10)
            assert entries == []

    def test_decision_without_result(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "decisions.jsonl"
            log = DecisionLog(path)

            log.log_decision(
                symbol="BTCUSDT",
                action="sell",
                confidence=70.0,
                reasoning="Bearish divergence on RSI",
                sources=["chart analysis"],
            )

            entries = log.recent(1)
            assert len(entries) == 1
            assert entries[0]["result"] is None

"""Session & decision persistence — JSONL append-only logs."""

import json
import time
from pathlib import Path
from datetime import datetime, timezone


class SessionLog:
    """Append-only JSONL session transcript."""

    def __init__(self, sessions_dir: Path):
        self.sessions_dir = sessions_dir
        self.session_id = datetime.now(timezone.utc).strftime("%Y%m%d_%H%M%S")
        self.path = sessions_dir / f"{self.session_id}.jsonl"

    def append(self, role: str, content: str, **extra):
        entry = {
            "ts": datetime.now(timezone.utc).isoformat(),
            "role": role,
            "content": content,
            **extra,
        }
        with open(self.path, "a", encoding="utf-8") as f:
            f.write(json.dumps(entry, ensure_ascii=False) + "\n")


class DecisionLog:
    """Structured log of every trading decision + reasoning."""

    def __init__(self, path: Path):
        self.path = path

    def log_decision(
        self,
        symbol: str,
        action: str,
        confidence: float,
        reasoning: str,
        sources: list[str],
        result: dict | None = None,
    ):
        entry = {
            "ts": datetime.now(timezone.utc).isoformat(),
            "symbol": symbol,
            "action": action,
            "confidence": confidence,
            "reasoning": reasoning,
            "sources": sources,
            "result": result,
        }
        with open(self.path, "a", encoding="utf-8") as f:
            f.write(json.dumps(entry, ensure_ascii=False) + "\n")

    def recent(self, n: int = 20) -> list[dict]:
        if not self.path.exists():
            return []
        lines = self.path.read_text(encoding="utf-8").strip().split("\n")
        entries = []
        for line in lines[-n:]:
            try:
                entries.append(json.loads(line))
            except json.JSONDecodeError:
                pass
        return entries

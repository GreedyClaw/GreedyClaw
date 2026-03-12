"""Brain configuration — loads from ~/.greedyclaw/brain.yaml or env vars."""

import os
import yaml
from pathlib import Path
from dataclasses import dataclass, field


def _config_dir() -> Path:
    return Path.home() / ".greedyclaw"


@dataclass
class BrainConfig:
    # LLM
    anthropic_api_key: str = ""
    model: str = "claude-sonnet-4-20250514"
    max_tokens: int = 4096

    # GreedyClaw gateway
    gateway_url: str = "http://127.0.0.1:7878"
    gateway_token: str = ""

    # Agent behavior
    loop_interval_minutes: int = 15
    market: str = "forex"  # forex, crypto, solana
    symbols: list[str] = field(default_factory=lambda: ["XAUUSD"])

    # Data sources
    sources: list[str] = field(default_factory=lambda: [
        "forex_factory",
        "investing_com",
        "reuters",
    ])

    # Persistence
    data_dir: Path = field(default_factory=lambda: _config_dir() / "brain")
    sessions_dir: Path = field(default_factory=lambda: _config_dir() / "brain" / "sessions")
    decisions_log: Path = field(default_factory=lambda: _config_dir() / "brain" / "decisions.jsonl")

    @classmethod
    def load(cls) -> "BrainConfig":
        cfg = cls()

        # Load from YAML if exists
        yaml_path = _config_dir() / "brain.yaml"
        if yaml_path.exists():
            with open(yaml_path) as f:
                data = yaml.safe_load(f) or {}
            for k, v in data.items():
                if hasattr(cfg, k):
                    setattr(cfg, k, v)

        # Env overrides
        cfg.anthropic_api_key = os.environ.get("ANTHROPIC_API_KEY", cfg.anthropic_api_key)
        cfg.model = os.environ.get("BRAIN_MODEL", cfg.model)
        cfg.gateway_url = os.environ.get("GREEDYCLAW_URL", cfg.gateway_url)
        cfg.gateway_token = os.environ.get("GREEDYCLAW_AUTH_TOKEN", cfg.gateway_token)
        cfg.market = os.environ.get("BRAIN_MARKET", cfg.market)

        if isinstance(cfg.symbols, str):
            cfg.symbols = [s.strip() for s in cfg.symbols.split(",")]

        # Ensure dirs
        cfg.data_dir = Path(cfg.data_dir)
        cfg.sessions_dir = Path(cfg.sessions_dir)
        cfg.decisions_log = Path(cfg.decisions_log)
        cfg.data_dir.mkdir(parents=True, exist_ok=True)
        cfg.sessions_dir.mkdir(parents=True, exist_ok=True)

        return cfg


DEFAULT_BRAIN_YAML = """\
# GreedyClaw Brain — Autonomous AI Trading Agent
# Docs: https://github.com/GreedyClaw/GreedyClaw

# LLM model (anthropic/claude-sonnet-4-20250514, anthropic/claude-opus-4, etc.)
model: claude-sonnet-4-20250514

# How often the agent wakes up to check the market (minutes)
loop_interval_minutes: 15

# Market focus: forex, crypto, solana
market: forex

# Symbols to watch
symbols:
  - XAUUSD

# Data sources for research
sources:
  - forex_factory
  - investing_com
  - reuters
"""

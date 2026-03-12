"""Tests for brain/tools.py — SSRF protection and tool schema validation."""

import sys
from pathlib import Path

# Ensure brain package is importable
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from tools import _is_ssrf_safe, TradingTools


class TestSSRFProtection:
    """Verify that private/internal IPs are blocked by _is_ssrf_safe."""

    def test_localhost_blocked(self):
        assert _is_ssrf_safe("http://localhost/admin") is False

    def test_127_0_0_1_blocked(self):
        assert _is_ssrf_safe("http://127.0.0.1/secret") is False

    def test_10_x_blocked(self):
        assert _is_ssrf_safe("http://10.0.0.1/internal") is False

    def test_172_16_blocked(self):
        assert _is_ssrf_safe("http://172.16.0.1/api") is False

    def test_192_168_blocked(self):
        assert _is_ssrf_safe("http://192.168.1.1/router") is False

    def test_metadata_blocked(self):
        """Cloud metadata endpoint (GCP/AWS) must be blocked."""
        assert _is_ssrf_safe("http://169.254.169.254/latest/meta-data/") is False

    def test_google_metadata_blocked(self):
        assert _is_ssrf_safe("http://metadata.google.internal/computeMetadata/v1/") is False

    def test_empty_url_blocked(self):
        assert _is_ssrf_safe("") is False

    def test_malformed_url_blocked(self):
        assert _is_ssrf_safe("not-a-url") is False

    def test_public_url_allowed(self):
        """A real public URL should pass SSRF check."""
        assert _is_ssrf_safe("https://www.google.com") is True

    def test_ipv6_loopback_blocked(self):
        assert _is_ssrf_safe("http://[::1]/admin") is False


class TestToolDefinitions:
    """Verify all tool schemas have required fields."""

    def test_all_tools_have_required_fields(self):
        schemas = TradingTools.schemas()
        assert len(schemas) > 0, "Should have at least one tool"

        for tool in schemas:
            assert "name" in tool, f"Tool missing 'name': {tool}"
            assert "description" in tool, f"Tool {tool.get('name', '?')} missing 'description'"
            assert "parameters" in tool, f"Tool {tool.get('name', '?')} missing 'parameters'"
            assert isinstance(tool["name"], str) and len(tool["name"]) > 0
            assert isinstance(tool["description"], str) and len(tool["description"]) > 0
            assert isinstance(tool["parameters"], dict)

    def test_trade_tool_has_required_params(self):
        schemas = TradingTools.schemas()
        trade = next(t for t in schemas if t["name"] == "trade")
        props = trade["parameters"]["properties"]
        assert "action" in props
        assert "symbol" in props
        assert "amount" in props
        assert trade["parameters"]["required"] == ["action", "symbol", "amount"]

    def test_tool_names_are_unique(self):
        schemas = TradingTools.schemas()
        names = [t["name"] for t in schemas]
        assert len(names) == len(set(names)), f"Duplicate tool names: {names}"

    def test_expected_tools_exist(self):
        schemas = TradingTools.schemas()
        names = {t["name"] for t in schemas}
        expected = {"trade", "get_price", "get_positions", "get_balance", "get_risk_status", "web_search", "fetch_url", "log_decision"}
        assert expected.issubset(names), f"Missing tools: {expected - names}"

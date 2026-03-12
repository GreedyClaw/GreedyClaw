/// GreedyClaw extension for OpenClaw.
/// Registers 8 trading tools that proxy to the local GreedyClaw REST API.
///
/// Setup:
///   1. Set env vars: GREEDYCLAW_AUTH_TOKEN, GREEDYCLAW_URL (default: http://127.0.0.1:7878)
///   2. Add to openclaw.json:
///      { "plugins": { "load": { "paths": ["path/to/openclaw-extension"] }, "entries": { "greedyclaw": { "enabled": true } } } }
///   3. Restart OpenClaw

import type { OpenClawPluginApi } from "openclaw/plugin-sdk";
import { ALL_TOOLS } from "./src/trading-tools.js";

const plugin = {
  id: "greedyclaw",
  name: "GreedyClaw",
  description: "AI-native trading execution gateway — Binance, PumpFun, PumpSwap",

  register(api: OpenClawPluginApi) {
    for (const tool of ALL_TOOLS) {
      api.registerTool(tool as any, { name: tool.name });
    }

    api.logger.info(
      `[GreedyClaw] Registered ${ALL_TOOLS.length} trading tools: ${ALL_TOOLS.map((t) => t.name).join(", ")}`
    );
  },
};

export default plugin;

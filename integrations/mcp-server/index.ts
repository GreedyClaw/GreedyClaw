#!/usr/bin/env npx tsx
/// GreedyClaw MCP Server — wraps the REST API as MCP tools.
/// Usage: npx tsx index.ts
/// Env: GREEDYCLAW_AUTH_TOKEN, GREEDYCLAW_URL (default http://127.0.0.1:7878)

import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { z } from "zod";

const BASE_URL = process.env.GREEDYCLAW_URL || "http://127.0.0.1:7878";
const TOKEN = process.env.GREEDYCLAW_AUTH_TOKEN || "";

async function gcFetch(
  path: string,
  method = "GET",
  body?: unknown
): Promise<string> {
  const resp = await fetch(`${BASE_URL}${path}`, {
    method,
    headers: {
      Authorization: `Bearer ${TOKEN}`,
      "Content-Type": "application/json",
    },
    body: body ? JSON.stringify(body) : undefined,
  });
  const data = await resp.json();
  return JSON.stringify(data, null, 2);
}

const server = new McpServer({
  name: "greedyclaw",
  version: "0.1.0",
});

// --- trade ---
server.tool(
  "trade",
  "Execute a trade (buy/sell) on the configured exchange. Returns fill details and risk snapshot.",
  {
    action: z.enum(["buy", "sell"]).describe("Trade direction"),
    symbol: z
      .string()
      .describe("Trading pair (BTCUSDT) or token mint address for Solana"),
    amount: z
      .number()
      .positive()
      .describe(
        "Quantity: base asset for Binance, SOL for Solana buy, tokens for Solana sell"
      ),
    order_type: z
      .enum(["market", "limit"])
      .default("market")
      .describe("Order type"),
    price: z
      .number()
      .optional()
      .describe("Limit price (required for limit orders)"),
  },
  async ({ action, symbol, amount, order_type, price }) => ({
    content: [
      {
        type: "text",
        text: await gcFetch("/trade", "POST", {
          action,
          symbol,
          amount,
          order_type,
          price,
        }),
      },
    ],
  })
);

// --- status ---
server.tool(
  "status",
  "Check gateway health, exchange connectivity, and risk state.",
  {},
  async () => ({
    content: [{ type: "text", text: await gcFetch("/status") }],
  })
);

// --- balance ---
server.tool(
  "balance",
  "Get account balances from the configured exchange.",
  {},
  async () => ({
    content: [{ type: "text", text: await gcFetch("/balance") }],
  })
);

// --- positions ---
server.tool(
  "positions",
  "Get open positions with entry price, current price, and unrealized PnL.",
  {},
  async () => ({
    content: [{ type: "text", text: await gcFetch("/positions") }],
  })
);

// --- price ---
server.tool(
  "price",
  "Get current price for a symbol or token.",
  {
    symbol: z.string().describe("Trading pair or token mint address"),
  },
  async ({ symbol }) => ({
    content: [
      {
        type: "text",
        text: await gcFetch(`/price/${encodeURIComponent(symbol)}`),
      },
    ],
  })
);

// --- trades ---
server.tool(
  "trades",
  "Get recent trade history (last 50 trades from audit log).",
  {},
  async () => ({
    content: [{ type: "text", text: await gcFetch("/trades") }],
  })
);

// --- stats ---
server.tool(
  "stats",
  "Get aggregated trading statistics: total trades, volume, commissions.",
  {},
  async () => ({
    content: [{ type: "text", text: await gcFetch("/trades/stats") }],
  })
);

// --- cancel ---
server.tool(
  "cancel",
  "Cancel an open order.",
  {
    symbol: z.string().describe("Trading pair (e.g. BTCUSDT)"),
    order_id: z.string().describe("Exchange order ID"),
  },
  async ({ symbol, order_id }) => ({
    content: [
      {
        type: "text",
        text: await gcFetch(
          `/order/${encodeURIComponent(symbol)}:${encodeURIComponent(order_id)}`,
          "DELETE"
        ),
      },
    ],
  })
);

// --- pnl ---
server.tool(
  "pnl",
  "Get PnL time series for equity curve visualization.",
  {},
  async () => ({
    content: [{ type: "text", text: await gcFetch("/trades/pnl") }],
  })
);

// --- scanner_start ---
server.tool(
  "scanner_start",
  "Start the PumpFun token scanner (Solana memecoin discovery).",
  {},
  async () => ({
    content: [{ type: "text", text: await gcFetch("/scanner/start", "POST") }],
  })
);

// --- scanner_stop ---
server.tool(
  "scanner_stop",
  "Stop the PumpFun token scanner.",
  {},
  async () => ({
    content: [{ type: "text", text: await gcFetch("/scanner/stop", "POST") }],
  })
);

// --- scanner_status ---
server.tool(
  "scanner_status",
  "Get scanner status: tracked tokens, metrics, top tokens by score.",
  {},
  async () => ({
    content: [{ type: "text", text: await gcFetch("/scanner/status") }],
  })
);

// --- scanner_tokens ---
server.tool(
  "scanner_tokens",
  "Get all tokens tracked by the scanner with scores and metrics.",
  {},
  async () => ({
    content: [{ type: "text", text: await gcFetch("/scanner/tokens") }],
  })
);

// Start
const transport = new StdioServerTransport();
await server.connect(transport);

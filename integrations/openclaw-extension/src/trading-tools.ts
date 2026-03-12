/// GreedyClaw trading tools for OpenClaw.
/// Each tool maps to a GreedyClaw REST API endpoint.

import { Type } from "@sinclair/typebox";

const GREEDYCLAW_URL = process.env.GREEDYCLAW_URL || "http://127.0.0.1:7878";
const GREEDYCLAW_TOKEN = process.env.GREEDYCLAW_AUTH_TOKEN || "";

async function gcFetch(path: string, options?: RequestInit) {
  const resp = await fetch(`${GREEDYCLAW_URL}${path}`, {
    ...options,
    headers: {
      Authorization: `Bearer ${GREEDYCLAW_TOKEN}`,
      "Content-Type": "application/json",
      ...options?.headers,
    },
  });
  const data = await resp.json();
  if (!resp.ok) {
    return {
      success: false,
      error: (data as any).error || resp.statusText,
      code: (data as any).code || "HTTP_ERROR",
      suggestion: (data as any).suggestion || "",
    };
  }
  return data;
}

function jsonResult(data: unknown) {
  return { type: "text" as const, text: JSON.stringify(data, null, 2) };
}

// --- Tool: Trade ---

const TradeSchema = Type.Object({
  action: Type.Enum({ buy: "buy", sell: "sell" }, {
    description: 'Trade direction: "buy" or "sell"',
  }),
  symbol: Type.String({
    description:
      "Trading pair (e.g. BTCUSDT for Binance) or token mint address for Solana exchanges",
  }),
  amount: Type.Number({
    description:
      "Quantity: base asset for Binance, SOL amount for Solana buy, token count for Solana sell",
    minimum: 0,
  }),
  order_type: Type.Optional(
    Type.Enum({ market: "market", limit: "limit" }, {
      description: 'Order type (default: "market")',
    })
  ),
  price: Type.Optional(
    Type.Number({ description: "Limit price (required for limit orders)" })
  ),
});

export const tradeTool = {
  name: "greedyclaw_trade",
  label: "GreedyClaw Trade",
  description:
    "Execute a trade (buy/sell) on the configured exchange (Binance, PumpFun, or PumpSwap). Returns fill details and risk snapshot.",
  parameters: TradeSchema,
  async execute(_toolCallId: string, args: unknown) {
    const params = args as {
      action: string;
      symbol: string;
      amount: number;
      order_type?: string;
      price?: number;
    };
    return jsonResult(
      await gcFetch("/trade", {
        method: "POST",
        body: JSON.stringify(params),
      })
    );
  },
};

// --- Tool: Status ---

export const statusTool = {
  name: "greedyclaw_status",
  label: "GreedyClaw Status",
  description:
    "Check GreedyClaw gateway health, exchange connectivity, and current risk state.",
  parameters: Type.Object({}),
  async execute() {
    return jsonResult(await gcFetch("/status"));
  },
};

// --- Tool: Balance ---

export const balanceTool = {
  name: "greedyclaw_balance",
  label: "GreedyClaw Balance",
  description: "Get account balances from the configured exchange.",
  parameters: Type.Object({}),
  async execute() {
    return jsonResult(await gcFetch("/balance"));
  },
};

// --- Tool: Positions ---

export const positionsTool = {
  name: "greedyclaw_positions",
  label: "GreedyClaw Positions",
  description:
    "Get all open positions with entry price, current price, and unrealized PnL.",
  parameters: Type.Object({}),
  async execute() {
    return jsonResult(await gcFetch("/positions"));
  },
};

// --- Tool: Price ---

const PriceSchema = Type.Object({
  symbol: Type.String({
    description: "Trading pair or token mint address",
  }),
});

export const priceTool = {
  name: "greedyclaw_price",
  label: "GreedyClaw Price",
  description: "Get current price for a symbol or token.",
  parameters: PriceSchema,
  async execute(_toolCallId: string, args: unknown) {
    const { symbol } = args as { symbol: string };
    return jsonResult(await gcFetch(`/price/${encodeURIComponent(symbol)}`));
  },
};

// --- Tool: Trades ---

export const tradesTool = {
  name: "greedyclaw_trades",
  label: "GreedyClaw Trade History",
  description: "Get recent trade history from the audit log (last 50 trades).",
  parameters: Type.Object({}),
  async execute() {
    return jsonResult(await gcFetch("/trades"));
  },
};

// --- Tool: Stats ---

export const statsTool = {
  name: "greedyclaw_stats",
  label: "GreedyClaw Stats",
  description:
    "Get aggregated trading statistics: total trades, buys, sells, volume, commissions.",
  parameters: Type.Object({}),
  async execute() {
    return jsonResult(await gcFetch("/trades/stats"));
  },
};

// --- Tool: Cancel ---

const CancelSchema = Type.Object({
  symbol: Type.String({ description: "Trading pair (e.g. BTCUSDT)" }),
  order_id: Type.String({ description: "Exchange order ID to cancel" }),
});

export const cancelTool = {
  name: "greedyclaw_cancel",
  label: "GreedyClaw Cancel Order",
  description: "Cancel an open order by symbol and order ID.",
  parameters: CancelSchema,
  async execute(_toolCallId: string, args: unknown) {
    const { symbol, order_id } = args as {
      symbol: string;
      order_id: string;
    };
    return jsonResult(
      await gcFetch(`/order/${encodeURIComponent(symbol)}:${encodeURIComponent(order_id)}`, {
        method: "DELETE",
      })
    );
  },
};

export const ALL_TOOLS = [
  tradeTool,
  statusTool,
  balanceTool,
  positionsTool,
  priceTool,
  tradesTool,
  statsTool,
  cancelTool,
];

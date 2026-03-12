/// Embedded web dashboard for GreedyClaw.
/// Served at GET /dashboard — single HTML page with Chart.js from CDN.

use axum::response::Html;

pub async fn serve_dashboard() -> Html<&'static str> {
    Html(DASHBOARD_HTML)
}

const DASHBOARD_HTML: &str = r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>GreedyClaw Dashboard</title>
<script src="https://cdn.jsdelivr.net/npm/chart.js@4.4.7/dist/chart.umd.min.js"></script>
<style>
:root {
  --bg: #0d1117;
  --bg2: #161b22;
  --bg3: #21262d;
  --border: #30363d;
  --text: #e6edf3;
  --text2: #8b949e;
  --green: #3fb950;
  --red: #f85149;
  --blue: #58a6ff;
  --yellow: #d29922;
  --purple: #bc8cff;
}
* { margin: 0; padding: 0; box-sizing: border-box; }
body {
  font-family: 'Segoe UI', -apple-system, sans-serif;
  background: var(--bg);
  color: var(--text);
  min-height: 100vh;
}
a { color: var(--blue); text-decoration: none; }

/* Header */
.header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 24px;
  background: var(--bg2);
  border-bottom: 1px solid var(--border);
}
.header .logo {
  font-size: 18px;
  font-weight: 700;
  letter-spacing: -0.5px;
}
.header .logo span { color: var(--green); }
.header-right {
  display: flex;
  align-items: center;
  gap: 12px;
}
.status-dot {
  width: 8px; height: 8px;
  border-radius: 50%;
  background: var(--text2);
  display: inline-block;
}
.status-dot.ok { background: var(--green); }
.status-dot.err { background: var(--red); }
.badge {
  font-size: 12px;
  padding: 2px 8px;
  border-radius: 12px;
  background: var(--bg3);
  border: 1px solid var(--border);
  color: var(--text2);
}

/* Auth bar */
.auth-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 24px;
  background: var(--bg2);
  border-bottom: 1px solid var(--border);
  font-size: 13px;
}
.auth-bar input {
  background: var(--bg);
  border: 1px solid var(--border);
  color: var(--text);
  padding: 4px 10px;
  border-radius: 6px;
  font-family: monospace;
  font-size: 13px;
  width: 280px;
}
.auth-bar button {
  background: var(--green);
  color: var(--bg);
  border: none;
  padding: 4px 14px;
  border-radius: 6px;
  cursor: pointer;
  font-weight: 600;
  font-size: 13px;
}
.auth-bar button:hover { opacity: 0.9; }
.auth-bar .auto-refresh {
  margin-left: auto;
  color: var(--text2);
  display: flex;
  align-items: center;
  gap: 6px;
}
.auth-bar .auto-refresh label { cursor: pointer; }

/* Layout */
.container {
  max-width: 1400px;
  margin: 0 auto;
  padding: 20px 24px;
}

/* Stat cards */
.stats-row {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
  gap: 12px;
  margin-bottom: 20px;
}
.stat-card {
  background: var(--bg2);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 16px;
}
.stat-card .label {
  font-size: 12px;
  color: var(--text2);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  margin-bottom: 6px;
}
.stat-card .value {
  font-size: 24px;
  font-weight: 700;
  font-variant-numeric: tabular-nums;
}
.stat-card .value.positive { color: var(--green); }
.stat-card .value.negative { color: var(--red); }
.stat-card .sub {
  font-size: 12px;
  color: var(--text2);
  margin-top: 4px;
}

/* Chart */
.chart-container {
  background: var(--bg2);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 16px;
  margin-bottom: 20px;
}
.chart-container h3 {
  font-size: 14px;
  color: var(--text2);
  margin-bottom: 12px;
  font-weight: 500;
}
.chart-container canvas {
  max-height: 300px;
}

/* Two columns */
.two-col {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 16px;
  margin-bottom: 20px;
}
@media (max-width: 900px) {
  .two-col { grid-template-columns: 1fr; }
}

/* Risk gauges */
.risk-panel {
  background: var(--bg2);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 16px;
}
.risk-panel h3 {
  font-size: 14px;
  color: var(--text2);
  margin-bottom: 12px;
  font-weight: 500;
}
.gauge {
  margin-bottom: 12px;
}
.gauge .gauge-label {
  display: flex;
  justify-content: space-between;
  font-size: 13px;
  margin-bottom: 4px;
}
.gauge-bar {
  height: 6px;
  background: var(--bg3);
  border-radius: 3px;
  overflow: hidden;
}
.gauge-fill {
  height: 100%;
  border-radius: 3px;
  transition: width 0.5s ease;
}

/* Positions table */
.panel {
  background: var(--bg2);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 16px;
}
.panel h3 {
  font-size: 14px;
  color: var(--text2);
  margin-bottom: 12px;
  font-weight: 500;
}
.panel .empty {
  color: var(--text2);
  font-size: 13px;
  font-style: italic;
  padding: 20px 0;
  text-align: center;
}

/* Tables */
table {
  width: 100%;
  border-collapse: collapse;
  font-size: 13px;
}
th {
  text-align: left;
  color: var(--text2);
  font-weight: 500;
  padding: 8px 10px;
  border-bottom: 1px solid var(--border);
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}
td {
  padding: 8px 10px;
  border-bottom: 1px solid var(--bg3);
  font-variant-numeric: tabular-nums;
}
tr:hover td { background: var(--bg3); }
.side-buy { color: var(--green); font-weight: 600; }
.side-sell { color: var(--red); font-weight: 600; }
.status-filled { color: var(--green); }
.status-rejected { color: var(--red); }

/* Trade history */
.trades-panel {
  background: var(--bg2);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 16px;
  margin-bottom: 20px;
}
.trades-panel h3 {
  font-size: 14px;
  color: var(--text2);
  margin-bottom: 12px;
  font-weight: 500;
}
.trades-panel .table-scroll {
  max-height: 400px;
  overflow-y: auto;
}
.trades-panel .table-scroll::-webkit-scrollbar { width: 6px; }
.trades-panel .table-scroll::-webkit-scrollbar-track { background: var(--bg); }
.trades-panel .table-scroll::-webkit-scrollbar-thumb { background: var(--border); border-radius: 3px; }

/* Loading / error */
.loading {
  text-align: center;
  color: var(--text2);
  padding: 40px;
  font-size: 14px;
}
.error-msg {
  color: var(--red);
  background: rgba(248,81,73,0.1);
  border: 1px solid rgba(248,81,73,0.3);
  padding: 8px 12px;
  border-radius: 6px;
  font-size: 13px;
  margin-bottom: 12px;
}
</style>
</head>
<body>

<div class="header">
  <div class="logo"><span>Greedy</span>Claw</div>
  <div class="header-right">
    <span class="status-dot" id="statusDot"></span>
    <span class="badge" id="exchangeBadge">—</span>
    <span class="badge" id="versionBadge">—</span>
  </div>
</div>

<div class="auth-bar">
  <label>Token:</label>
  <input type="password" id="tokenInput" placeholder="Bearer auth token" />
  <button onclick="connect()">Connect</button>
  <div class="auto-refresh">
    <input type="checkbox" id="autoRefresh" checked />
    <label for="autoRefresh">Auto-refresh 10s</label>
  </div>
</div>

<div id="errorBox" style="padding: 0 24px; margin-top: 12px;"></div>

<div class="container" id="main" style="display:none;">
  <!-- Stat cards -->
  <div class="stats-row">
    <div class="stat-card">
      <div class="label">Daily PnL</div>
      <div class="value" id="dailyPnl">$0.00</div>
      <div class="sub" id="dailyPnlSub">realized + floating</div>
    </div>
    <div class="stat-card">
      <div class="label">Total Trades</div>
      <div class="value" id="totalTrades">0</div>
      <div class="sub" id="totalTradesSub">0 today</div>
    </div>
    <div class="stat-card">
      <div class="label">Volume (USD)</div>
      <div class="value" id="totalVolume">$0</div>
      <div class="sub" id="totalCommission">fees: $0</div>
    </div>
    <div class="stat-card">
      <div class="label">Open Positions</div>
      <div class="value" id="openPositions">0</div>
      <div class="sub" id="openPositionsSub">max: 0</div>
    </div>
    <div class="stat-card">
      <div class="label">Symbols Traded</div>
      <div class="value" id="uniqueSymbols">0</div>
      <div class="sub">unique instruments</div>
    </div>
    <div class="stat-card">
      <div class="label">Buy / Sell</div>
      <div class="value" id="buySellRatio">0 / 0</div>
      <div class="sub" id="rejectedTrades">0 rejected</div>
    </div>
  </div>

  <!-- Equity curve -->
  <div class="chart-container">
    <h3>Equity Curve — Cumulative Realized PnL</h3>
    <canvas id="equityChart"></canvas>
  </div>

  <!-- Positions & Risk -->
  <div class="two-col">
    <div class="panel">
      <h3>Open Positions</h3>
      <div id="positionsTable"><div class="empty">No open positions</div></div>
    </div>
    <div class="risk-panel">
      <h3>Risk Limits</h3>
      <div class="gauge">
        <div class="gauge-label">
          <span>Daily Loss Used</span>
          <span id="dailyLossText">$0 / $0</span>
        </div>
        <div class="gauge-bar"><div class="gauge-fill" id="dailyLossBar" style="width:0;background:var(--green);"></div></div>
      </div>
      <div class="gauge">
        <div class="gauge-label">
          <span>Positions</span>
          <span id="positionsText">0 / 0</span>
        </div>
        <div class="gauge-bar"><div class="gauge-fill" id="positionsBar" style="width:0;background:var(--blue);"></div></div>
      </div>
      <div class="gauge">
        <div class="gauge-label">
          <span>Rate (trades/min)</span>
          <span id="rateText">0 / 0</span>
        </div>
        <div class="gauge-bar"><div class="gauge-fill" id="rateBar" style="width:0;background:var(--purple);"></div></div>
      </div>
    </div>
  </div>

  <!-- Trade history -->
  <div class="trades-panel">
    <h3>Trade History</h3>
    <div class="table-scroll" id="tradesTable">
      <div class="empty">No trades yet</div>
    </div>
  </div>
</div>

<script>
let token = localStorage.getItem('gc_token') || '';
let refreshTimer = null;
let equityChart = null;

document.getElementById('tokenInput').value = token;

if (token) connect();

function getHeaders() {
  return { 'Authorization': 'Bearer ' + token, 'Content-Type': 'application/json' };
}

async function apiFetch(path) {
  const resp = await fetch(path, { headers: getHeaders() });
  if (!resp.ok) throw new Error(`${resp.status} ${resp.statusText}`);
  return resp.json();
}

async function connect() {
  token = document.getElementById('tokenInput').value.trim();
  if (!token) return;
  localStorage.setItem('gc_token', token);
  document.getElementById('errorBox').innerHTML = '';

  try {
    await refresh();
    document.getElementById('main').style.display = 'block';
    startAutoRefresh();
  } catch(e) {
    showError('Connection failed: ' + e.message);
  }
}

function showError(msg) {
  document.getElementById('errorBox').innerHTML = '<div class="error-msg">' + escHtml(msg) + '</div>';
}

function escHtml(s) {
  const d = document.createElement('div');
  d.textContent = s;
  return d.innerHTML;
}

async function refresh() {
  const [status, stats, pnl, positions, trades] = await Promise.all([
    apiFetch('/status'),
    apiFetch('/trades/stats'),
    apiFetch('/trades/pnl'),
    apiFetch('/positions'),
    apiFetch('/trades'),
  ]);

  updateHeader(status);
  updateStats(stats, status.risk);
  updateRisk(status.risk);
  updateEquityChart(pnl.series);
  updatePositions(positions);
  updateTrades(trades.trades);
}

function updateHeader(s) {
  const dot = document.getElementById('statusDot');
  dot.className = 'status-dot ' + (s.status === 'ok' ? 'ok' : 'err');
  document.getElementById('exchangeBadge').textContent = s.exchange + (s.testnet ? ' (testnet)' : '');
  document.getElementById('versionBadge').textContent = 'v' + s.version;
}

function updateStats(stats, risk) {
  // Daily PnL
  const pnl = risk.total_daily_pnl;
  const el = document.getElementById('dailyPnl');
  el.textContent = '$' + pnl.toFixed(2);
  el.className = 'value ' + (pnl >= 0 ? 'positive' : 'negative');
  document.getElementById('dailyPnlSub').textContent =
    'realized $' + risk.realized_daily_pnl.toFixed(2) + ' + floating $' + risk.floating_pnl.toFixed(2);

  // Total trades
  document.getElementById('totalTrades').textContent = stats.total_trades;
  document.getElementById('totalTradesSub').textContent = stats.today_trades + ' today';

  // Volume
  document.getElementById('totalVolume').textContent = '$' + fmtNum(stats.total_volume_usd);
  document.getElementById('totalCommission').textContent = 'fees: $' + stats.total_commission.toFixed(4);

  // Positions
  document.getElementById('openPositions').textContent = risk.open_positions;
  document.getElementById('openPositionsSub').textContent = 'max: ' + risk.max_open_positions;

  // Symbols
  document.getElementById('uniqueSymbols').textContent = stats.unique_symbols;

  // Buy/Sell
  document.getElementById('buySellRatio').textContent = stats.buys + ' / ' + stats.sells;
  document.getElementById('rejectedTrades').textContent = stats.rejected + ' rejected';
}

function updateRisk(risk) {
  // Daily loss gauge
  const lossUsed = Math.abs(Math.min(risk.total_daily_pnl, 0));
  const lossMax = lossUsed + risk.remaining_daily_limit;
  const lossPct = lossMax > 0 ? (lossUsed / lossMax * 100) : 0;
  document.getElementById('dailyLossText').textContent = '$' + lossUsed.toFixed(2) + ' / $' + lossMax.toFixed(2);
  const lossBar = document.getElementById('dailyLossBar');
  lossBar.style.width = lossPct + '%';
  lossBar.style.background = lossPct > 80 ? 'var(--red)' : lossPct > 50 ? 'var(--yellow)' : 'var(--green)';

  // Positions gauge
  const posPct = risk.max_open_positions > 0 ? (risk.open_positions / risk.max_open_positions * 100) : 0;
  document.getElementById('positionsText').textContent = risk.open_positions + ' / ' + risk.max_open_positions;
  document.getElementById('positionsBar').style.width = posPct + '%';

  // Rate gauge
  const ratePct = risk.max_trades_per_minute > 0 ? (risk.trades_last_minute / risk.max_trades_per_minute * 100) : 0;
  document.getElementById('rateText').textContent = risk.trades_last_minute + ' / ' + risk.max_trades_per_minute;
  const rateBar = document.getElementById('rateBar');
  rateBar.style.width = ratePct + '%';
  rateBar.style.background = ratePct > 80 ? 'var(--red)' : 'var(--purple)';
}

function updateEquityChart(series) {
  const ctx = document.getElementById('equityChart');
  if (!series || series.length === 0) {
    if (equityChart) { equityChart.destroy(); equityChart = null; }
    return;
  }

  // Build cumulative PnL from realized_pnl field
  let cumPnl = 0;
  const labels = [];
  const data = [];

  for (const pt of series) {
    labels.push(pt.timestamp.replace('T', ' ').substring(0, 19));
    // realized_pnl is cumulative daily from risk engine
    data.push(pt.realized_pnl);
  }

  if (equityChart) {
    equityChart.data.labels = labels;
    equityChart.data.datasets[0].data = data;
    equityChart.update('none');
    return;
  }

  equityChart = new Chart(ctx, {
    type: 'line',
    data: {
      labels: labels,
      datasets: [{
        label: 'Realized PnL ($)',
        data: data,
        borderColor: '#3fb950',
        backgroundColor: 'rgba(63,185,80,0.1)',
        fill: true,
        tension: 0.3,
        pointRadius: 2,
        pointHoverRadius: 5,
        borderWidth: 2,
      }]
    },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      interaction: { intersect: false, mode: 'index' },
      plugins: {
        legend: { display: false },
        tooltip: {
          backgroundColor: '#161b22',
          borderColor: '#30363d',
          borderWidth: 1,
          titleColor: '#e6edf3',
          bodyColor: '#e6edf3',
        }
      },
      scales: {
        x: {
          ticks: { color: '#8b949e', maxTicksLimit: 10, font: { size: 11 } },
          grid: { color: '#21262d' },
        },
        y: {
          ticks: {
            color: '#8b949e',
            font: { size: 11 },
            callback: function(v) { return '$' + v.toFixed(2); }
          },
          grid: { color: '#21262d' },
        }
      }
    }
  });
}

function updatePositions(positions) {
  const container = document.getElementById('positionsTable');
  if (!positions || positions.length === 0) {
    container.innerHTML = '<div class="empty">No open positions</div>';
    return;
  }

  let html = '<table><thead><tr><th>Symbol</th><th>Qty</th><th>Entry</th><th>Current</th><th>PnL</th></tr></thead><tbody>';
  for (const p of positions) {
    const pnl = p.unrealized_pnl || 0;
    const pnlClass = pnl >= 0 ? 'positive' : 'negative';
    html += '<tr>'
      + '<td>' + escHtml(p.symbol) + '</td>'
      + '<td>' + p.quantity.toFixed(6) + '</td>'
      + '<td>$' + p.avg_entry_price.toFixed(4) + '</td>'
      + '<td>$' + (p.current_price || 0).toFixed(4) + '</td>'
      + '<td class="' + pnlClass + '">$' + pnl.toFixed(2) + '</td>'
      + '</tr>';
  }
  html += '</tbody></table>';
  container.innerHTML = html;
}

function updateTrades(trades) {
  const container = document.getElementById('tradesTable');
  if (!trades || trades.length === 0) {
    container.innerHTML = '<div class="empty">No trades yet</div>';
    return;
  }

  let html = '<table><thead><tr><th>Time</th><th>Symbol</th><th>Side</th><th>Qty</th><th>Price</th><th>Status</th><th>Order ID</th></tr></thead><tbody>';
  for (const t of trades) {
    const sideClass = t.side === 'buy' ? 'side-buy' : 'side-sell';
    const statusClass = (t.status || '').toLowerCase().includes('filled') ? 'status-filled'
      : (t.status || '').toLowerCase().includes('rejected') ? 'status-rejected' : '';
    const time = (t.timestamp || '').replace('T', ' ').substring(0, 19);
    html += '<tr>'
      + '<td>' + escHtml(time) + '</td>'
      + '<td>' + escHtml(t.symbol) + '</td>'
      + '<td class="' + sideClass + '">' + escHtml(t.side).toUpperCase() + '</td>'
      + '<td>' + (t.filled_qty || 0).toFixed(6) + '</td>'
      + '<td>$' + (t.avg_price || 0).toFixed(4) + '</td>'
      + '<td class="' + statusClass + '">' + escHtml(t.status || '') + '</td>'
      + '<td style="font-size:11px;color:var(--text2)">' + escHtml((t.exchange_order_id || '').substring(0, 16)) + '</td>'
      + '</tr>';
  }
  html += '</tbody></table>';
  container.innerHTML = html;
}

function fmtNum(n) {
  if (n >= 1e6) return (n / 1e6).toFixed(2) + 'M';
  if (n >= 1e3) return (n / 1e3).toFixed(1) + 'K';
  return n.toFixed(2);
}

function startAutoRefresh() {
  if (refreshTimer) clearInterval(refreshTimer);
  refreshTimer = setInterval(async () => {
    if (!document.getElementById('autoRefresh').checked) return;
    try { await refresh(); } catch(e) { /* silent */ }
  }, 10000);
}
</script>
</body>
</html>
"##;

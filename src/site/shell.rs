//! the page's shell: the one HTML document of cx.cyberia.my, with the parts
//! rendered next door dropped into it. the markup and the assembly live apart
//! so neither has to be read to change the other.

/// every part the shell interpolates, already rendered.
pub struct Parts<'a> {
    pub level: String,
    pub base_date: String,
    pub tabs: String,
    pub readout: String,
    pub chart_svg: &'a str,
    pub chart_labels: &'a str,
    pub bar: String,
    pub chips: String,
    pub legs: String,
    pub deaths: String,
    pub views: String,
    pub calc: String,
}

/// the whole page, from the doctype to the closing tag.
pub fn page(p: &Parts) -> String {
    format!(
        r##"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>CX {level} — century index · Cyberia</title>
<meta property="og:title" content="CX — century index">
<meta property="og:description" content="The unit of account for century-scale obligations. Eight world assets at fixed weights, rebalanced every day: whoever owes the index owes CX, and the weights never change.">
<link rel="preload" href="/fonts/play-400.woff2" as="font" type="font/woff2" crossorigin>
<link rel="preload" href="/fonts/play-700.woff2" as="font" type="font/woff2" crossorigin>
<style>
  @font-face{{font-family:'Play';font-style:normal;font-weight:400;font-display:swap;src:url(/fonts/play-400.woff2) format('woff2')}}
  @font-face{{font-family:'Play';font-style:normal;font-weight:700;font-display:swap;src:url(/fonts/play-700.woff2) format('woff2')}}
  :root{{
    --bg:#050505;--green:#79ff4f;--cyan:#00e5ff;--ink:#f2f2f2;--mute:#8a8a8a;--line:#1f1f1f;
    --crypto:#2f9e14;--fiat:#0093ad;--elements:#c98500;
  }}
  *{{box-sizing:border-box;margin:0;padding:0}}
  html,body{{background:var(--bg);color:var(--ink);font-family:'Play',sans-serif;-webkit-font-smoothing:antialiased}}
  body{{min-height:100vh;display:flex;flex-direction:column;align-items:center;padding:40px 20px 56px}}
  main{{width:100%;max-width:520px;display:flex;flex-direction:column;gap:28px}}
  .brand{{display:flex;align-items:center;gap:12px;font-size:17px;letter-spacing:.06em;color:var(--mute)}}
  .brand svg{{width:32px;height:32px}}
  h1{{font-size:34px;line-height:1.05;font-weight:700}}
  h1 span{{display:block;color:var(--green)}}
  h2{{font-size:15px;font-weight:400;color:var(--mute)}}
  section{{display:flex;flex-direction:column;gap:12px}}

  .hero{{border-top:2px solid var(--green);padding-top:14px;display:flex;flex-direction:column;gap:4px}}
  .hero b{{font-size:52px;line-height:1;font-weight:700}}
  .hero .unit{{font-size:14px;color:var(--mute);margin-top:6px}}
  .tabs{{display:flex;gap:6px;margin-top:14px}}
  .tabs button{{font-family:inherit;font-size:13px;letter-spacing:.04em;color:var(--mute);
    background:none;border:1px solid var(--line);border-radius:8px;padding:5px 12px;cursor:pointer;
    transition:color .15s,border-color .15s}}
  .tabs button:hover{{color:var(--ink)}}
  .tabs button.on{{color:var(--bg);background:var(--green);border-color:var(--green)}}

  .chartwrap{{position:relative;padding-bottom:24px}}
  .chart{{width:100%;height:200px;display:block;overflow:visible}}
  .labels{{position:absolute;left:0;right:0;top:0;height:200px;pointer-events:none}}
  .labels span{{position:absolute;font-size:13px;color:var(--mute);white-space:nowrap;
    font-variant-numeric:tabular-nums}}
  .labels .y{{right:0;transform:translateY(-50%)}}
  .labels .x{{top:206px;transform:translateX(-50%)}}
  .chart .line{{fill:none;stroke:var(--green);stroke-width:2;stroke-linejoin:round;stroke-linecap:round}}
  .chart .grid{{stroke:var(--line);stroke-width:1}}
  .chart .end{{fill:var(--green);stroke:var(--bg);stroke-width:2}}
  .chart .cross{{stroke:var(--mute);stroke-width:1}}
  .chart .dot{{fill:var(--green);stroke:var(--bg);stroke-width:2}}
  .readout{{font-size:13px;color:var(--mute);min-height:18px}}
  .readout b{{color:var(--ink);font-weight:400}}

  .bar{{display:flex;height:24px;gap:2px}}
  .bar .seg{{position:relative}}
  .bar .seg:first-child{{border-radius:4px 0 0 4px}}
  .bar .seg:last-child{{border-radius:0 4px 4px 0}}
  .bar .seg i{{position:absolute;inset:0;display:grid;place-items:center;font-style:normal;font-size:12px;color:#fff}}
  .seg.crypto{{background:var(--crypto)}}
  .seg.fiat{{background:var(--fiat)}}
  .seg.elements{{background:var(--elements)}}
  .chips{{display:flex;gap:16px;flex-wrap:wrap;font-size:13px;color:var(--mute)}}
  .chips b{{color:var(--ink);font-weight:400}}
  .chips .d,td .d{{display:inline-block;width:10px;height:10px;border-radius:3px;margin-right:6px;vertical-align:-1px}}
  .d.crypto{{background:var(--crypto)}}
  .d.fiat{{background:var(--fiat)}}
  .d.elements{{background:var(--elements)}}

  table{{width:100%;border-collapse:collapse;font-size:14px}}
  th{{font-weight:400;color:var(--mute);text-align:left;padding:6px 8px 6px 0;border-bottom:1px solid var(--line)}}
  th.n,td.n{{text-align:right;font-variant-numeric:tabular-nums;white-space:nowrap}}
  td{{padding:8px 8px 8px 0;border-bottom:1px solid var(--line)}}

  p{{font-size:15px;line-height:1.5}}
  p .mute,li .mute{{color:var(--mute)}}
  .formula{{border:1px solid var(--line);border-radius:12px;padding:14px 16px;font-size:15px;line-height:1.6;letter-spacing:.02em}}
  .formula .mute{{color:var(--mute)}}
  ul{{list-style:none;display:flex;flex-direction:column;gap:8px;font-size:15px;line-height:1.45}}
  ul li{{padding-left:16px;position:relative}}
  ul li::before{{content:"›";position:absolute;left:0;color:var(--green)}}
  .note{{font-size:13px;line-height:1.5;color:var(--mute)}}
  .note a{{color:var(--cyan);text-decoration:none}}
  footer{{margin-top:16px;display:flex;justify-content:space-between;align-items:center;color:var(--cyan);font-size:14px;letter-spacing:.08em}}
  footer a{{color:var(--mute);text-decoration:none;letter-spacing:0}}
</style>
</head>
<body>
<main>
  <div class="brand">
    <svg viewBox="0 0 100 100" aria-hidden="true">
      <circle cx="50" cy="12" r="9" fill="#79ff4f"/><circle cx="80" cy="27" r="9" fill="#00a2ff"/>
      <circle cx="87" cy="60" r="9" fill="#1a3fbf"/><circle cx="64" cy="86" r="9" fill="#9b4dff"/>
      <circle cx="30" cy="86" r="9" fill="#ff3b3b"/><circle cx="12" cy="60" r="9" fill="#ff8a1e"/>
      <circle cx="20" cy="27" r="9" fill="#ffe14d"/>
    </svg>
    Cyberia
  </div>
  <div class="player" data-cyberia-player style="display:flex;margin:16px 0 0"></div>

  <h1>CX<span>century index</span></h1>

  <div class="hero">
    <b>${level}</b>
    <div class="unit">for 1 CX — the basket that cost $1 on {base_date}</div>
    <div class="tabs" role="tablist">{tabs}</div>
  </div>

  <section>
    <div class="chartwrap">
      <svg class="chart" id="chart" viewBox="0 0 1000 320" preserveAspectRatio="none"
           role="img" aria-label="CX index level">{chart_svg}</svg>
      <div class="labels" id="labels">{chart_labels}</div>
    </div>
    <div class="readout" id="readout">{readout}</div>
  </section>

  <section>
    <h2>What it is</h2>
    <p>The unit of account for century-scale obligations. Eight world assets at fixed
    weights, rebalanced every day: whoever owes the index owes CX, and the weights never change.
    <span class="mute">A 25–80 year lease outlives every currency it could be written in —
    so the payment is a portfolio instead of a number, and the portfolio is the same one
    for everyone on every day.</span></p>
  </section>

  <section>
    <h2>Basket</h2>
    <div class="bar" role="img" aria-label="basket weights: BTC 20, ETH 15, CNY 15, USD 15, GOLD 15, CU 10, OIL 5, UX 5 percent">{bar}</div>
    <div class="chips">{chips}</div>
    <table>
      <thead><tr><th>leg</th><th class="n">fix (USD)</th><th class="n">one CX holds</th><th class="n">weight</th><th class="n">of peak</th></tr></thead>
      <tbody>{legs}</tbody>
    </table>
    <div class="note">The basket is rebalanced to its weights every day, so what one CX holds changes daily
    and the weights never do. Of peak is the death gauge: a leg whose 365-day average closes below 10% of its
    highest average since {base_date} leaves the basket, its weight going pro rata to the rest, never to return.
    {deaths} Every fix is a trailing 365-day average.</div>
  </section>

  <section>
    <h2>Definition</h2>
    <div class="formula">
      I(t) = I(t−1) · Σ w<sub>i</sub> · P<sub>i</sub>(t) / P<sub>i</sub>(t−1)
      &nbsp;&nbsp;&nbsp;
      R(t) = R<sub>0</sub> · I(t) / I(t<sub>0</sub>)
      <div class="mute">weights fixed for good, rebalanced daily · every price a trailing 365-day average of daily fixes · I = $1 on {base_date}</div>
    </div>
  </section>

  <section>
    <h2>Mechanics</h2>
    <ul>
      <li>Rent — a number of CX a year, for the life of the lease; measured in the index it never changes</li>
      <li>Reset — annual, on the contract anniversary; the TWAP window ends 30 days before payment, so the tenant knows the invoice a month ahead</li>
      <li>Settlement — the CX owed are converted at that day's fix into the payment currency, never capped <span class="mute">— dollars, rupiah or satoshi are what the rent is paid in, not what it is</span></li>
      <li>Nothing on top — no collar, no floor, no ruler: a clamp needs a unit outside the index, and that unit becomes a second bet over the basket</li>
      <li>Oracle — a daily on-chain fix; on divergence the annex computation from named public fixes prevails <span class="mute">— index disputes are arithmetic, never renegotiation</span></li>
    </ul>
  </section>

  <section>
    <h2>Pricer</h2>
    <p class="note">What the index is for. The product is a leasehold — the right to use a
    parcel for a term, never its title. Part of its value is taken as a premium at signing,
    the rest returned as rent, indexed for the life of the contract. One thing moves: the indexation, so the
    lease can be stress-tested across what the index might do. The deal and the estate's underwriting are
    stated as the fixed figures they are, shown so the price can be checked rather than taken
    on faith.</p>
    {calc}
  </section>

  <section>
    <h2>Sources</h2>
    <div class="note">
      This published series is a reconstruction from free public data, not the contractual
      annex fix: BTC and ETH from Coinbase daily closes, gold as
      <a href="https://paxos.com/paxg/">PAXG</a> daily closes from Yahoo Finance (one token per troy ounce; the
      <a href="https://prices.lbma.org.uk/">LBMA</a> PM fix is the fallback), CNY from the ECB reference rate,
      Brent and the IMF copper and uranium assessments from
      <a href="https://fred.stlouisfed.org/">FRED</a>. Copper and uranium publish monthly and
      carry forward between prints. A signed lease names its own fixes with fallbacks —
      see the protocol page in the graph.
      Machine-readable: <a href="/cx.json">cx.json</a>.
    </div>
  </section>

  <footer>
    <span>don't trust · verify</span>
    <a href="https://cyberia.my">cyberia.my</a>
  </footer>
</main>
<script>
(() => {{
  const views = {views};
  const svg = document.getElementById('chart');
  const labels = document.getElementById('labels');
  const readout = document.getElementById('readout');
  const rest = readout.textContent;
  let pts = [];

  const bind = (key) => {{
    const view = views[key];
    if (!view) return;
    svg.innerHTML = view.svg;
    labels.innerHTML = view.labels;
    pts = view.pts;
  }};

  const at = (evt) => {{
    if (!pts.length) return;
    const hover = svg.querySelector('.hover');
    const cross = svg.querySelector('.cross');
    const dot = svg.querySelector('.dot');
    if (!hover) return;
    const box = svg.getBoundingClientRect();
    const x = ((evt.touches ? evt.touches[0].clientX : evt.clientX) - box.left) / box.width * 1000;
    let best = 0;
    for (let i = 1; i < pts.length; i++) {{
      if (Math.abs(pts[i][0] - x) < Math.abs(pts[best][0] - x)) best = i;
    }}
    const p = pts[best];
    cross.setAttribute('x1', p[0]); cross.setAttribute('x2', p[0]);
    dot.setAttribute('cx', p[0]); dot.setAttribute('cy', p[1]);
    hover.style.display = '';
    readout.innerHTML = '<b>$' + p[2] + '</b> · ' + p[3];
  }};

  const clear = () => {{
    const hover = svg.querySelector('.hover');
    if (hover) hover.style.display = 'none';
    readout.textContent = rest;
  }};

  svg.addEventListener('mousemove', at);
  svg.addEventListener('touchmove', at, {{passive: true}});
  svg.addEventListener('mouseleave', clear);
  svg.addEventListener('touchend', clear);

  document.querySelectorAll('.tabs button').forEach((b) => {{
    b.addEventListener('click', () => {{
      document.querySelectorAll('.tabs button').forEach((o) => o.classList.remove('on'));
      b.classList.add('on');
      clear();
      bind(b.dataset.range);
    }});
  }});
}})();
</script>
<script src="https://cyberia.to/player.js" async data-download="/music/atlas.shrugged.set.mp3"></script>
</body>
</html>
"##,
        level = p.level,
        base_date = p.base_date,
        tabs = p.tabs,
        readout = p.readout,
        chart_svg = p.chart_svg,
        chart_labels = p.chart_labels,
        bar = p.bar,
        chips = p.chips,
        legs = p.legs,
        deaths = p.deaths,
        views = p.views,
        calc = p.calc,
    )
}

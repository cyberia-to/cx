//! the published page at cyberia.my/cx and the machine-readable fix beside it.

use crate::chart::Chart;
use crate::index::{Group, Index};
use crate::num::{fdiv, format_date, format_fixed, format_thousands, SCALE};
use crate::series::Daily;

fn group_class(g: Group) -> &'static str {
    match g {
        Group::Crypto => "crypto",
        Group::Fiat => "fiat",
        Group::Elements => "elements",
    }
}

/// percentage change between two levels, in fixed point.
fn change_pct(now: i128, then: i128) -> Option<i128> {
    let ratio = fdiv(now - then, then)?;
    Some(ratio * 100)
}

pub fn render(idx: &Index, twaps: &[(&str, Daily)], chart: &Chart) -> Result<String, String> {
    let (day, level) = idx.latest().ok_or("no index level")?;
    let shares = idx.current_shares(twaps);

    let year_ago = idx.level_before(365).and_then(|then| change_pct(level, then));
    let since_base = change_pct(level, crate::index::BASE_LEVEL);

    let mut legs_html = String::new();
    for (q, (leg, share)) in idx.quantities.iter().zip(shares.iter()) {
        let fix = twaps
            .iter()
            .find(|(t, _)| *t == leg.ticker)
            .and_then(|(_, d)| d.value_on(day))
            .unwrap_or(0);
        legs_html.push_str(&format!(
            "<tr><td><span class=\"d {cls}\"></span>{ticker}</td>\
             <td class=\"n\">{fix}</td><td class=\"n\">{qty}</td>\
             <td class=\"n\">{weight}%</td><td class=\"n\">{share}%</td></tr>",
            cls = group_class(leg.group),
            ticker = leg.ticker,
            fix = format_thousands(fix, leg.fix_places()),
            qty = format_thousands(q.quantity, leg.quantity_places()),
            weight = leg.weight_bp / 100,
            share = format_fixed(*share * SCALE / 100, 1),
        ));
    }

    // the bar shows the basket as defined — the weights a lease is written at.
    // today's drifted shares live in the table beside them, where the two can
    // be read against each other.
    let mut bar_html = String::new();
    for leg in crate::index::LEGS.iter() {
        let pct = leg.weight_bp / 100;
        bar_html.push_str(&format!(
            "<div class=\"seg {}\" style=\"flex:{}\" title=\"{} — {}%\"><i>{}</i></div>",
            group_class(leg.group),
            leg.weight_bp,
            leg.ticker,
            pct,
            pct
        ));
    }

    let mut group_totals = [(Group::Crypto, 0i128), (Group::Fiat, 0), (Group::Elements, 0)];
    for leg in crate::index::LEGS.iter() {
        for slot in group_totals.iter_mut() {
            if slot.0 == leg.group {
                slot.1 += leg.weight_bp;
            }
        }
    }
    let mut drift_totals = [(Group::Crypto, 0i128), (Group::Fiat, 0), (Group::Elements, 0)];
    for (leg, share) in shares.iter() {
        for slot in drift_totals.iter_mut() {
            if slot.0 == leg.group {
                slot.1 += *share;
            }
        }
    }
    let drift_html: String = drift_totals
        .iter()
        .map(|(g, share)| format!("{} {}%", g.label(), format_fixed(*share * SCALE / 100, 1)))
        .collect::<Vec<_>>()
        .join(" · ");
    let chips_html: String = group_totals
        .iter()
        .map(|(g, share)| {
            format!(
                "<span><span class=\"d {}\"></span>{} <b>{}%</b></span>",
                group_class(*g),
                g.label(),
                format_fixed(*share * SCALE / 100, 1)
            )
        })
        .collect();

    let delta_html = match year_ago {
        Some(v) => format!(
            "<span class=\"delta {}\">{}{}% · 12 months</span>",
            if v >= 0 { "up" } else { "down" },
            if v >= 0 { "+" } else { "" },
            format_fixed(v, 1)
        ),
        None => String::new(),
    };

    let since_html = match since_base {
        Some(v) => format!(
            "{}{}% since {}",
            if v >= 0 { "+" } else { "" },
            format_fixed(v, 1),
            format_date(idx.base_day)
        ),
        None => String::new(),
    };

    let page = format!(
        r##"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>CX {level} — century index · Cyberia</title>
<meta property="og:title" content="CX — century index">
<meta property="og:description" content="The unit of account for century-scale obligations. A fixed basket of eight world assets: whoever owes the index owes quantities, and the quantities never change.">
<link href="https://fonts.googleapis.com/css2?family=Play:wght@400;700&display=swap" rel="stylesheet">
<style>
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
  .hero .row{{display:flex;gap:14px;align-items:baseline;flex-wrap:wrap;font-size:14px;color:var(--mute)}}
  .delta.up{{color:var(--green)}}
  .delta.down{{color:#e06060}}

  .chart{{width:100%;height:200px;overflow:visible}}
  .chart .line{{fill:none;stroke:var(--green);stroke-width:2;stroke-linejoin:round;stroke-linecap:round}}
  .chart .grid{{stroke:var(--line);stroke-width:1}}
  .chart .tick{{fill:var(--mute);font-size:11px;font-family:'Play',sans-serif}}
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

  <h1>CX<span>century index</span></h1>

  <div class="hero">
    <b>{level}</b>
    <div class="row"><span>fix of {date}</span>{delta}<span>{since}</span></div>
  </div>

  <section>
    {chart_svg}
    <div class="readout" id="readout">base 100 on {base_date} · logarithmic · hover the line for a day</div>
  </section>

  <section>
    <h2>What it is</h2>
    <p>The unit of account for century-scale obligations. A fixed basket of eight world
    assets: whoever owes the index owes quantities, and the quantities never change.
    <span class="mute">A 25–80 year lease outlives every currency it could be written in —
    so the payment is a portfolio instead of a number.</span></p>
  </section>

  <section>
    <h2>Basket</h2>
    <div class="bar" role="img" aria-label="basket weights: BTC 20, ETH 15, CNY 15, USD 15, GOLD 15, CU 10, OIL 5, UX 5 percent">{bar}</div>
    <div class="chips">{chips}</div>
    <table>
      <thead><tr><th>leg</th><th class="n">fix (USD)</th><th class="n">quantity</th><th class="n">weight</th><th class="n">share now</th></tr></thead>
      <tbody>{legs}</tbody>
    </table>
    <div class="note">Weight is what a lease is written at; share is what the leg has become.
    Quantities were fixed on {base_date} and never change, so the shares drift with prices —
    over this decade the basket ran to {drift}. A lease signed today starts at the weights, not the drift.
    Every fix is a trailing 365-day average.</div>
  </section>

  <section>
    <h2>Definition</h2>
    <div class="formula">
      q<sub>i</sub> = w<sub>i</sub> · R<sub>0</sub> / P<sub>i</sub>(t<sub>0</sub>)
      &nbsp;&nbsp;&nbsp;
      R(t) = Σ q<sub>i</sub> · P<sub>i</sub>(t)
      <div class="mute">quantities fixed at signing · every price a trailing 365-day average of daily fixes</div>
    </div>
  </section>

  <section>
    <h2>Mechanics</h2>
    <ul>
      <li>Reset — annual, on the contract anniversary; the TWAP window ends 30 days before payment, so the tenant knows the invoice a month ahead</li>
      <li>Ruler — bitcoin: the basket is priced in sats over the BTC/USD 365-day TWAP</li>
      <li>Collar — +35% / −15% per year in sats; wide enough to deliver the full index path, halving the descent toward the floor</li>
      <li>Dual floor — never fewer satoshi than year 0, never fewer year-0 dollars; whichever binds</li>
      <li>Oracle — a daily on-chain fix; on divergence the annex computation from named public fixes prevails <span class="mute">— index disputes are arithmetic, never renegotiation</span></li>
    </ul>
  </section>

  <section>
    <h2>Sources</h2>
    <div class="note">
      This published series is a reconstruction from free public data, not the contractual
      annex fix: BTC and ETH from Coinbase daily closes, gold from the
      <a href="https://prices.lbma.org.uk/">LBMA</a> PM fix, CNY from the ECB reference rate,
      Brent and the IMF copper and uranium assessments from
      <a href="https://fred.stlouisfed.org/">FRED</a>. Copper and uranium publish monthly and
      carry forward between prints. A signed lease names its own fixes with fallbacks —
      see the protocol page in the graph.
      Machine-readable: <a href="/cx/cx.json">cx.json</a>.
    </div>
  </section>

  <footer>
    <span>don't trust · verify</span>
    <a href="https://cyberia.my">cyberia.my</a>
  </footer>
</main>
<script>
(() => {{
  const pts = {points};
  const svg = document.querySelector('.chart');
  const hover = svg && svg.querySelector('.hover');
  const cross = hover && hover.querySelector('.cross');
  const dot = hover && hover.querySelector('.dot');
  const readout = document.getElementById('readout');
  if (!svg || !hover || !pts.length) return;
  const rest = readout.textContent;
  const firstDay = {first_day}, stepDays = {step_days};
  const iso = (d) => new Date(d * 86400000).toISOString().slice(0, 10);
  const at = (evt) => {{
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
    readout.innerHTML = '<b>' + p[2] + '</b> · ' + iso(firstDay + best * stepDays);
  }};
  svg.addEventListener('mousemove', at);
  svg.addEventListener('touchmove', at, {{passive: true}});
  const clear = () => {{ hover.style.display = 'none'; readout.textContent = rest; }};
  svg.addEventListener('mouseleave', clear);
  svg.addEventListener('touchend', clear);
}})();
</script>
</body>
</html>
"##,
        level = format_thousands(level, 2),
        date = format_date(day),
        base_date = format_date(idx.base_day),
        delta = delta_html,
        since = since_html,
        chart_svg = chart.svg,
        bar = bar_html,
        chips = chips_html,
        legs = legs_html,
        drift = drift_html,
        points = chart.points_json,
        first_day = chart.first_day,
        step_days = chart.step_days,
    );

    Ok(page)
}

/// the same fix as data: the level series, the quantities and the provenance.
pub fn render_json(
    idx: &Index,
    twaps: &[(&str, Daily)],
    coverage: &[(&str, String, usize)],
) -> String {
    let (day, level) = match idx.latest() {
        Some(v) => v,
        None => (0, 0),
    };
    let mut out = String::from("{\n");
    out.push_str(&format!("  \"index\": \"CX\",\n"));
    out.push_str(&format!("  \"base_date\": \"{}\",\n", format_date(idx.base_day)));
    out.push_str("  \"base_level\": 100,\n");
    out.push_str(&format!("  \"date\": \"{}\",\n", format_date(day)));
    out.push_str(&format!("  \"level\": {},\n", format_fixed(level, 4)));
    out.push_str("  \"twap_days\": 365,\n");

    out.push_str("  \"legs\": [\n");
    let shares = idx.current_shares(twaps);
    for (i, (q, (leg, share))) in idx.quantities.iter().zip(shares.iter()).enumerate() {
        let fix = twaps
            .iter()
            .find(|(t, _)| *t == leg.ticker)
            .and_then(|(_, d)| d.value_on(day))
            .unwrap_or(0);
        let cov = coverage
            .iter()
            .find(|(t, _, _)| *t == leg.ticker)
            .map(|(_, span, n)| format!("{span} ({n} observations)"))
            .unwrap_or_default();
        out.push_str(&format!(
            "    {{\"ticker\": \"{}\", \"group\": \"{}\", \"weight_bp\": {}, \"unit\": \"{}\", \
             \"fix\": {}, \"quantity\": {}, \"share_bp\": {}, \"source\": \"{}\", \"coverage\": \"{}\"}}{}\n",
            leg.ticker,
            leg.group.label(),
            leg.weight_bp,
            leg.unit,
            format_fixed(fix, 6),
            format_fixed(q.quantity, 8),
            share,
            leg.source,
            cov,
            if i + 1 < idx.quantities.len() { "," } else { "" }
        ));
    }
    out.push_str("  ],\n");

    out.push_str("  \"history\": [\n");
    let mut day_cursor = idx.level.start;
    let mut first = true;
    while day_cursor <= idx.level.end() {
        if let Some(v) = idx.level.value_on(day_cursor) {
            if !first {
                out.push_str(",\n");
            }
            out.push_str(&format!(
                "    [\"{}\", {}]",
                format_date(day_cursor),
                format_fixed(v, 4)
            ));
            first = false;
        }
        day_cursor += 7;
    }
    out.push_str("\n  ]\n}\n");
    out
}

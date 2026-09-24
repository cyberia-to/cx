//! the pages this tool writes into the cx subgraph.
//!
//! the graph holds the record: what the index is, what it read on each month
//! since the base date, and where every number came from.

use crate::index::{Anniversary, Binding, Index};
use crate::num::{civil_from_days, fdiv, format_date, format_fixed, format_thousands, SCALE};
use crate::series::Daily;

fn change_pct(now: i128, then: i128) -> Option<i128> {
    Some(fdiv(now - then, then)? * 100)
}

fn signed(v: i128, places: u32) -> String {
    format!("{}{}", if v >= 0 { "+" } else { "" }, format_fixed(v, places))
}

/// the last published level of every month in the series.
fn monthly_closes(level: &Daily) -> Vec<(i64, i128)> {
    let mut out: Vec<(i64, i128)> = Vec::new();
    let mut day = level.start;
    while day <= level.end() {
        if let Some(v) = level.value_on(day) {
            let (y, m, _) = civil_from_days(day);
            match out.last_mut() {
                Some((prev_day, prev_v)) => {
                    let (py, pm, _) = civil_from_days(*prev_day);
                    if py == y && pm == m {
                        *prev_day = day;
                        *prev_v = v;
                    } else {
                        out.push((day, v));
                    }
                }
                None => out.push((day, v)),
            }
        }
        day += 1;
    }
    out
}

pub fn render_history(
    idx: &Index,
    twaps: &[(&str, Daily)],
    coverage: &[(&str, String, usize)],
    path: &[Anniversary],
    annual_rate: i128,
) -> Result<String, String> {
    let (last_day, last_level) = idx.latest().ok_or("no index level")?;
    let months = monthly_closes(&idx.level);


    let mut body = String::new();
    body.push_str(
        "---\ntags: cyberia, protocol, cybernomics, cx\nalias: cx history, century index history, \
         index history, cx series\ncrystal-type: pattern\ncrystal-domain: cyberia\n---\n",
    );
    body.push_str("# cx history\n\n");
    body.push_str(&format!(
        "the published level of the [[century-index]] from its base date to {}. \
         one CX is the basket that cost $1 on {}; today it costs ${}. every price enters as a trailing \
         365-day average, so the series moves at the pace of a year, not a day — \
         which is what a century-scale obligation asks of it.\n\n",
        format_date(last_day),
        format_date(idx.base_day),
        format_thousands(last_level, 2)
    ));

    body.push_str("## reading\n\n");
    if let Some(first) = idx.level.value_on(idx.level.start) {
        if let Some(total) = change_pct(last_level, first) {
            body.push_str(&format!(
                "- since {} the basket returned {}% in dollars\n",
                format_date(idx.base_day),
                signed(total, 1)
            ));
        }
    }
    if let Some(then) = idx.level_before(365) {
        if let Some(y) = change_pct(last_level, then) {
            body.push_str(&format!("- the last twelve months: {}%\n", signed(y, 1)));
        }
    }
    let high = idx.level.values.iter().max().copied().unwrap_or(0);
    let low = idx.level.values.iter().min().copied().unwrap_or(0);
    body.push_str(&format!(
        "- the range since the base date: {} low, {} high\n\n",
        format_thousands(low, 2),
        format_thousands(high, 2)
    ));

    body.push_str("## quantities\n\n");
    body.push_str(&format!(
        "fixed on {}, unchanged since — the obligation is these quantities, \
         not the number they priced at.\n\n",
        format_date(idx.base_day)
    ));
    body.push_str("| leg | quantity | unit | fix at base | weight |\n");
    body.push_str("|---|---|---|---|---|\n");
    for q in &idx.quantities {
        body.push_str(&format!(
            "| {} | {} | {} | {} | {}% |\n",
            q.leg.ticker,
            format_thousands(q.quantity, q.leg.quantity_places(q.quantity)),
            q.leg.unit,
            format_thousands(q.base_fix, q.leg.fix_places()),
            format_fixed(q.leg.weight_bp * SCALE / 100, 0)
        ));
    }
    body.push('\n');

    body.push_str("## monthly level\n\n");
    body.push_str("| month | level | month | level |\n|---|---|---|---|\n");
    let half = months.len().div_ceil(2);
    for i in 0..half {
        let left = months.get(i);
        let right = months.get(i + half);
        let cell = |p: Option<&(i64, i128)>| -> (String, String) {
            match p {
                Some((d, v)) => {
                    let (y, m, _) = civil_from_days(*d);
                    (format!("{y}-{m:02}"), format_thousands(*v, 2))
                }
                None => ("—".into(), "—".into()),
            }
        };
        let (lm, lv) = cell(left);
        let (rm, rv) = cell(right);
        body.push_str(&format!("| {lm} | {lv} | {rm} | {rv} |\n"));
    }
    body.push('\n');

    body.push_str("## sources\n\n");
    body.push_str(
        "this series is a reconstruction from free public data — evidence and orientation, \
         never the contractual fix. a signed annex names its own sources with fallbacks, \
         per the cessation waterfall in [[century-index]].\n\n",
    );
    body.push_str("| leg | source | observed range |\n|---|---|---|\n");
    for q in &idx.quantities {
        let cov = coverage
            .iter()
            .find(|(t, _, _)| *t == q.leg.ticker)
            .map(|(_, span, n)| format!("{span} · {n} observations"))
            .unwrap_or_else(|| "—".to_string());
        body.push_str(&format!("| {} | {} | {} |\n", q.leg.ticker, q.leg.source, cov));
    }
    body.push_str(
        "\ncopper and uranium publish monthly and carry forward between prints, as a closed \
         market does. the uranium assessment is the weakest fix in the basket, which is why \
         it carries the smallest weight.\n\n",
    );

    body.push_str("## drift\n\n");
    let shares = idx.current_shares(twaps);
    let mut crypto_now = 0i128;
    for (leg, share) in shares.iter() {
        if leg.group == crate::index::Group::Crypto {
            crypto_now += *share;
        }
    }
    let crypto_base = 3500i128;
    let moved = crypto_now - crypto_base;
    let direction = if moved >= 0 { "gained" } else { "given up" };
    body.push_str(&format!(
        "quantities never change, so shares do. the basket was written at 35% crypto, \
         30% fiat, 35% elements; since {} bitcoin and ether have {} {} points of share \
         against the rest, leaving the basket at {}% crypto today. this is the arithmetic \
         of fixed quantities, not a flaw in the fixes — but over a 25-80 year lease it is \
         the property that decides what the obligation actually tracks: the basket rides \
         with the world, as the quantities were written.\n\n",
        format_date(idx.base_day),
        direction,
        format_fixed(moved.abs() * SCALE / 100, 1),
        format_fixed(crypto_now * SCALE / 100, 1)
    ));

    if let (Some(first), Some(last)) = (path.first(), path.last()) {
        body.push_str("## what a lease actually paid\n\n");
        body.push_str(&format!(
            "the machinery of §2 run from the base date: the basket priced in bitcoin, each \
             annual step collared at +35%/−15%, the dual floor holding underneath. a lease \
             signed on {} at one CX of rent invoiced ${} in its first year and ${} in its \
             last — {}% a year, compounded.\n\n",
            format_date(idx.base_day),
            format_fixed(first.invoice, 2),
            format_fixed(last.invoice, 2),
            signed(annual_rate, 1)
        ));
        body.push_str("| year | basket in btc | rent owed | invoice | set by |\n|---|---|---|---|---|\n");
        for a in path {
            body.push_str(&format!(
                "| {} | {} | {} | ${} | {} |\n",
                format_date(a.day),
                format_fixed(a.uncollared, 8),
                format_fixed(a.rent_btc, 8),
                format_thousands(a.invoice, 2),
                a.binding.label()
            ));
        }

        // the reading is counted from the path, never asserted — the same
        // sentence has to stay true whichever rule the years actually hit
        let years = &path[1..];
        let count = |b: Binding| years.iter().filter(|a| a.binding == b).count();
        let (up, down, sat, fiat, basket) = (
            count(Binding::CollarUp),
            count(Binding::CollarDown),
            count(Binding::SatFloor),
            count(Binding::FiatFloor),
            count(Binding::Basket),
        );
        let plural = |n: usize| if n == 1 { "year" } else { "years" };
        body.push_str(&format!(
            "\nof the {} anniversaries after signing: the basket set the rent outright in {} {}, \
             the collar capped a rise in {} and held a fall in {}, the dollar floor bound in {} \
             and the satoshi floor in {}. ",
            years.len(),
            basket,
            plural(basket),
            up,
            down,
            fiat,
            sat
        ));
        if fiat + sat > 0 {
            body.push_str(
                "where a floor bound, the tenant owed year-zero money regardless of the basket — \
                 the dollar leg when bitcoin had fallen, the satoshi leg when it had outrun the \
                 basket: the protocol's bitcoin-standard reading. ",
            );
        }
        if down > 0 {
            body.push_str(
                "where the collar held a fall, the rent in sats came down 15% a year while the \
                 basket in sats fell faster; the dollar invoice still moved with bitcoin. ",
            );
        }
        if up > 0 {
            body.push_str(
                "where the collar capped a rise, the tenant kept the increase above 35% a year. ",
            );
        }
        body.push_str("\n\n");
    }

    body.push_str("## today\n\n");
    body.push_str("| leg | fix (usd) | share |\n|---|---|---|\n");
    for (leg, share) in shares.iter() {
        let fix = twaps
            .iter()
            .find(|(t, _)| *t == leg.ticker)
            .and_then(|(_, d)| d.value_on(last_day))
            .unwrap_or(0);
        body.push_str(&format!(
            "| {} | {} | {}% |\n",
            leg.ticker,
            format_thousands(fix, leg.fix_places()),
            format_fixed(*share * SCALE / 100, 1)
        ));
    }
    body.push_str(&format!(
        "\nthe fix is published daily at [cyberia.my/cx](https://cyberia.my/cx) and rebuilt \
         from source by [[cx]]. generated {}.\n",
        format_date(last_day)
    ));

    Ok(body)
}

pub fn render_readme(idx: &Index, twaps: &[(&str, Daily)]) -> Result<String, String> {
    let (last_day, last_level) = idx.latest().ok_or("no index level")?;
    let shares = idx.current_shares(twaps);

    let mut body = String::new();
    body.push_str(
        "---\ntags: cyberia, protocol, cybernomics, cx\nalias: cx, cx index, century index fix, \
         index publication\ncrystal-type: entity\ncrystal-domain: cyberia\n---\n",
    );
    body.push_str("# cx\n\n");
    body.push_str(&format!(
        "the published fix of the [[century-index]] — ticker CX. one CX is the basket that \
         cost one dollar on the base date, so the level is a price: ${} on {}, against $1 \
         on {}. the protocol page defines the instrument; this subgraph carries what it \
         actually read.\n\n",
        format_thousands(last_level, 2),
        format_date(last_day),
        format_date(idx.base_day)
    ));

    body.push_str("## pages\n\n");
    body.push_str("- [[cx history]] — the monthly level since the base date, the quantities, the sources\n");
    body.push_str("- [[century-index]] — the protocol: basket, collar, floor, contract theses\n\n");

    body.push_str("## how the fix is built\n\n");
    body.push_str(
        "1. pull each leg from its public source\n\
         2. carry every fix onto a dense daily calendar — a closed market holds its last print\n\
         3. average each leg over the trailing 365 days\n\
         4. hold the quantities fixed from the base date and sum them at today's averages\n\n",
    );
    body.push_str(
        "the arithmetic is integer fixed-point end to end, so the same day's data yields the \
         same level on any machine — the index is reproducible by anyone holding the sources, \
         which is the whole point of an obligation nobody has to be trusted about.\n\n",
    );

    body.push_str("## the basket today\n\n");
    body.push_str("| leg | group | share | weight at base |\n|---|---|---|---|\n");
    for (leg, share) in shares.iter() {
        body.push_str(&format!(
            "| {} | {} | {}% | {}% |\n",
            leg.ticker,
            leg.group.label(),
            format_fixed(*share * SCALE / 100, 1),
            format_fixed(leg.weight_bp * SCALE / 100, 0)
        ));
    }
    body.push_str(
        "\nshares drift as prices move; the quantities behind them never do. measured in \
         itself the index is constant, so the obligation carries no numéraire.\n",
    );

    Ok(body)
}

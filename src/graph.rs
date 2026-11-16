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

    let holdings = idx.holdings(twaps);
    body.push_str("## holdings\n\n");
    body.push_str(&format!(
        "what one CX holds on {} — the weights are fixed and the basket is rebalanced to \
         them every day, so the holdings change daily and the weights never do. of peak is \
         the death gauge: today's trailing average against the highest since the base date.\n\n",
        format_date(last_day)
    ));
    body.push_str("| leg | one CX holds | unit | fix | weight | of peak |\n");
    body.push_str("|---|---|---|---|---|---|\n");
    for h in &holdings {
        let fix = twaps
            .iter()
            .find(|(t, _)| *t == h.leg.ticker)
            .and_then(|(_, d)| d.value_on(last_day))
            .unwrap_or(0);
        let of_peak = if h.alive {
            format!("{}%", format_fixed(h.of_peak_bp * SCALE / 100, 0))
        } else {
            "left the basket".to_string()
        };
        body.push_str(&format!(
            "| {} | {} | {} | {} | {}% | {} |\n",
            h.leg.ticker,
            format_thousands(h.quantity, h.leg.quantity_places(h.quantity)),
            h.leg.unit,
            format_thousands(fix, h.leg.fix_places()),
            format_fixed(h.weight_bp * SCALE / 100, 0),
            of_peak
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
    for leg in crate::index::LEGS.iter() {
        let cov = coverage
            .iter()
            .find(|(t, _, _)| *t == leg.ticker)
            .map(|(_, span, n)| format!("{span} · {n} observations"))
            .unwrap_or_else(|| "—".to_string());
        body.push_str(&format!("| {} | {} | {} |\n", leg.ticker, leg.source, cov));
    }
    body.push_str(
        "\ncopper and uranium publish monthly and carry forward between prints, as a closed \
         market does. the uranium assessment is the weakest fix in the basket, which is why \
         it carries the smallest weight.\n\n",
    );

    body.push_str("## rebalancing\n\n");
    body.push_str(
        "each day the index earns the weighted return of its legs at trailing-average \
         prices — `I(t) = I(t−1) · Σ wᵢ · P̄ᵢ(t)/P̄ᵢ(t−1)`. a leg that ran is sold back to \
         its weight, a leg that fell is bought back to it, so CX is the same portfolio for \
         everyone on every day and its composition owes nothing to the base date. buying \
         what falls has a price: a leg going to zero would take the index with it, at the \
         power of its weight. the death rule bounds that — a leg leaves the basket the day \
         its trailing average closes below a tenth of its highest since the base date, its \
         weight goes pro rata to the rest, and it never returns. the most a leg can cost is \
         1 − 0.1^w: 37% for bitcoin, 29% for a 15% leg, 11% for a 5% leg.\n\n",
    );
    if idx.deaths.is_empty() {
        body.push_str("no leg has left the basket.\n\n");
    } else {
        for d in &idx.deaths {
            body.push_str(&format!(
                "- {} left the basket on {}: its trailing average closed at {} against a peak of {}\n",
                d.leg.ticker,
                format_date(d.day),
                format_thousands(d.fix, d.leg.fix_places()),
                format_thousands(d.peak, d.leg.fix_places())
            ));
        }
        body.push('\n');
    }

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

    body.push_str(&format!(
        "the fix is published daily at [cyberia.my/cx](https://cyberia.my/cx) and rebuilt \
         from source by [[cx]]. generated {}.\n",
        format_date(last_day)
    ));

    Ok(body)
}

pub fn render_readme(idx: &Index, twaps: &[(&str, Daily)]) -> Result<String, String> {
    let (last_day, last_level) = idx.latest().ok_or("no index level")?;
    let holdings = idx.holdings(twaps);

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
    body.push_str("- [[cx history]] — the monthly level since the base date, today's holdings, the sources\n");
    body.push_str("- [[century-index]] — the protocol: basket, collar, floor, contract theses\n\n");

    body.push_str("## how the fix is built\n\n");
    body.push_str(
        "1. pull each leg from its public source\n\
         2. carry every fix onto a dense daily calendar — a closed market holds its last print\n\
         3. average each leg over the trailing 365 days\n\
         4. chain the weighted daily returns: each day the index earns Σ wᵢ · P̄ᵢ(t)/P̄ᵢ(t−1), \
         the basket rebalanced to its weights every day\n\
         5. read the death rule: a leg whose average closes below a tenth of its peak since the \
         base date leaves the basket, weight pro rata to the rest, never to return\n\n",
    );
    body.push_str(
        "the arithmetic is integer fixed-point end to end, so the same day's data yields the \
         same level on any machine — the index is reproducible by anyone holding the sources, \
         which is the whole point of an obligation nobody has to be trusted about.\n\n",
    );

    body.push_str("## the basket\n\n");
    body.push_str("| leg | group | weight | of peak |\n|---|---|---|---|\n");
    for h in holdings.iter() {
        let of_peak = if h.alive {
            format!("{}%", format_fixed(h.of_peak_bp * SCALE / 100, 0))
        } else {
            "left the basket".to_string()
        };
        body.push_str(&format!(
            "| {} | {} | {}% | {} |\n",
            h.leg.ticker,
            h.leg.group.label(),
            format_fixed(h.weight_bp * SCALE / 100, 0),
            of_peak
        ));
    }
    body.push_str(
        "\nthe weights hold every day by construction. measured in itself the index is \
         constant, so the obligation carries no numéraire; a lease owes a number of CX.\n",
    );

    Ok(body)
}

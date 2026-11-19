//! the machine-readable fix beside the page: the same numbers as data.

use crate::index::Index;
use crate::num::{format_date, format_fixed};
use crate::series::Daily;

/// the same fix as data: the level series, today's holdings and the provenance.
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
    out.push_str("  \"base_level\": 1,\n");
    out.push_str("  \"unit\": \"USD per CX\",\n");
    out.push_str(&format!("  \"date\": \"{}\",\n", format_date(day)));
    out.push_str(&format!("  \"level\": {},\n", format_fixed(level, 4)));
    out.push_str("  \"twap_days\": 365,\n");
    out.push_str("  \"rebalance\": \"daily\",\n");
    out.push_str(&format!(
        "  \"death_rule\": \"a leg leaves the basket the day its trailing average closes below {}% of its peak trailing average since the base date; weight pro rata to the rest; never returns\",\n",
        crate::index::DEATH_BP / 100
    ));

    out.push_str("  \"legs\": [\n");
    let holdings = idx.holdings(twaps);
    for (i, h) in holdings.iter().enumerate() {
        let fix = twaps
            .iter()
            .find(|(t, _)| *t == h.leg.ticker)
            .and_then(|(_, d)| d.value_on(day))
            .unwrap_or(0);
        let cov = coverage
            .iter()
            .find(|(t, _, _)| *t == h.leg.ticker)
            .map(|(_, span, n)| format!("{span} ({n} observations)"))
            .unwrap_or_default();
        out.push_str(&format!(
            "    {{\"ticker\": \"{}\", \"group\": \"{}\", \"weight_bp\": {}, \"live_weight_bp\": {}, \"unit\": \"{}\", \
             \"fix\": {}, \"holds\": {}, \"of_peak_bp\": {}, \"alive\": {}, \"source\": \"{}\", \"coverage\": \"{}\"}}{}\n",
            h.leg.ticker,
            h.leg.group.label(),
            h.leg.weight_bp,
            h.weight_bp,
            h.leg.unit,
            format_fixed(fix, 6),
            format_fixed(h.quantity, 8),
            h.of_peak_bp,
            h.alive,
            h.leg.source,
            cov,
            if i + 1 < holdings.len() { "," } else { "" }
        ));
    }
    out.push_str("  ],\n");

    out.push_str("  \"deaths\": [");
    for (i, d) in idx.deaths.iter().enumerate() {
        out.push_str(&format!(
            "{}{{\"ticker\": \"{}\", \"date\": \"{}\", \"peak\": {}, \"fix\": {}}}",
            if i > 0 { ", " } else { "" },
            d.leg.ticker,
            format_date(d.day),
            format_fixed(d.peak, 6),
            format_fixed(d.fix, 6)
        ));
    }
    out.push_str("],\n");

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fix_publishes_the_live_weights_and_the_deaths() {
        let (idx, twaps) = crate::testdata::one_death();
        let json = render_json(&idx, &twaps, &[]);
        // the written weight and the live one, which a death pulled apart
        assert!(json.contains(
            "\"ticker\": \"BTC\", \"group\": \"crypto\", \"weight_bp\": 2000, \"live_weight_bp\": 2105,"
        ));
        assert!(json.contains(
            "\"ticker\": \"UX\", \"group\": \"elements\", \"weight_bp\": 500, \"live_weight_bp\": 0,"
        ));
        // the death gauge, and the flag that says the leg is gone
        assert!(json.contains("\"of_peak_bp\": 900, \"alive\": false,"));
        assert!(json.contains(
            "\"deaths\": [{\"ticker\": \"UX\", \"date\": \"1970-01-03\", \"peak\": 100.000000, \"fix\": 9.000000}]"
        ));
        // the history samples the level weekly from the base date
        assert!(json.contains("[\"1970-01-01\", 1.0000]"));
    }
}

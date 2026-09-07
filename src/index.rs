//! the basket and the index level.
//!
//! `q_i = w_i · BASE / P_i(t0)` fixes the quantities once, at the base date.
//! from then on `I(t) = Σ q_i · P_i(t)` — whoever owes the index owes
//! quantities, and the quantities never change.

use crate::num::{fdiv, fmul, SCALE};
use crate::series::Daily;

/// the index reads 100 on its base date.
pub const BASE_LEVEL: i128 = 100 * SCALE;

/// the trailing window of §3: every price enters as a 365-day average.
pub const TWAP_WINDOW: usize = 365;

/// which layer of the basket a leg belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Group {
    Crypto,
    Fiat,
    Elements,
}

impl Group {
    pub fn label(&self) -> &'static str {
        match self {
            Group::Crypto => "crypto",
            Group::Fiat => "fiat",
            Group::Elements => "elements",
        }
    }
}

/// one leg of the basket: what it is, how much of the index it carries, and
/// the unit its quantity is denominated in.
#[derive(Debug, Clone, Copy)]
pub struct Leg {
    pub ticker: &'static str,
    pub group: Group,
    /// weight in basis points of the index — integers, so the weights sum
    /// exactly to 10_000 and no rounding drift enters the quantities.
    pub weight_bp: i128,
    pub unit: &'static str,
    pub source: &'static str,
}

/// the eight legs, in publication order.
pub const LEGS: [Leg; 8] = [
    Leg {
        ticker: "BTC",
        group: Group::Crypto,
        weight_bp: 2000,
        unit: "BTC",
        source: "Coinbase BTC-USD daily close",
    },
    Leg {
        ticker: "ETH",
        group: Group::Crypto,
        weight_bp: 1500,
        unit: "ETH",
        source: "Coinbase ETH-USD daily close",
    },
    Leg {
        ticker: "CNY",
        group: Group::Fiat,
        weight_bp: 1500,
        unit: "CNY",
        source: "ECB reference rate via Frankfurter",
    },
    Leg {
        ticker: "USD",
        group: Group::Fiat,
        weight_bp: 1500,
        unit: "USD",
        source: "quote currency, fixed at 1",
    },
    Leg {
        ticker: "GOLD",
        group: Group::Elements,
        weight_bp: 1500,
        unit: "troy oz",
        source: "LBMA gold PM fix (USD)",
    },
    Leg {
        ticker: "CU",
        group: Group::Elements,
        weight_bp: 1000,
        unit: "tonne",
        source: "IMF global copper price via FRED (monthly)",
    },
    Leg {
        ticker: "OIL",
        group: Group::Elements,
        weight_bp: 500,
        unit: "bbl",
        source: "Brent Europe spot via FRED (daily)",
    },
    Leg {
        ticker: "UX",
        group: Group::Elements,
        weight_bp: 500,
        unit: "lb U3O8",
        source: "IMF uranium price via FRED (monthly)",
    },
];

impl Leg {
    /// decimals when printing this leg's fix — a yuan needs more than a barrel.
    pub fn fix_places(&self) -> u32 {
        match self.ticker {
            "CNY" => 6,
            "USD" => 0,
            _ => 2,
        }
    }

    /// decimals when printing this leg's quantity.
    pub fn quantity_places(&self) -> u32 {
        match self.ticker {
            "BTC" | "ETH" => 6,
            "CNY" | "USD" => 2,
            _ => 6,
        }
    }
}

/// a leg's fixed quantity, derived once from the base-date fix.
#[derive(Debug, Clone)]
pub struct Quantity {
    pub leg: Leg,
    pub base_fix: i128,
    pub quantity: i128,
}

/// the computed index: quantities, the daily level series, and the base date.
#[derive(Debug, Clone)]
pub struct Index {
    pub base_day: i64,
    pub quantities: Vec<Quantity>,
    pub level: Daily,
}

/// why an index could not be built.
#[derive(Debug)]
pub enum IndexError {
    WeightsDoNotSum(i128),
    MissingLeg(&'static str),
    BaseFixUnavailable(&'static str),
    NonPositiveFix(&'static str),
    NoOverlap,
}

impl std::fmt::Display for IndexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IndexError::WeightsDoNotSum(bp) => {
                write!(f, "weights sum to {bp} bp, expected 10000")
            }
            IndexError::MissingLeg(t) => write!(f, "no series supplied for leg {t}"),
            IndexError::BaseFixUnavailable(t) => {
                write!(f, "leg {t} has no fix on the base date")
            }
            IndexError::NonPositiveFix(t) => write!(f, "leg {t} carries a non-positive fix"),
            IndexError::NoOverlap => write!(f, "legs share no common date range"),
        }
    }
}

/// build the index from one TWAP series per leg, keyed by ticker.
///
/// `series` must hold every leg in [`LEGS`]; the index runs over the range
/// all of them cover, starting at `base_day`.
pub fn build(series: &[(&str, Daily)], base_day: i64) -> Result<Index, IndexError> {
    let total_bp: i128 = LEGS.iter().map(|l| l.weight_bp).sum();
    if total_bp != 10_000 {
        return Err(IndexError::WeightsDoNotSum(total_bp));
    }

    let mut quantities = Vec::with_capacity(LEGS.len());
    let mut start = base_day;
    let mut end = i64::MAX;

    for leg in LEGS.iter() {
        let daily = series
            .iter()
            .find(|(t, _)| *t == leg.ticker)
            .map(|(_, d)| d)
            .ok_or(IndexError::MissingLeg(leg.ticker))?;

        let base_fix = daily
            .value_on(base_day)
            .ok_or(IndexError::BaseFixUnavailable(leg.ticker))?;
        if base_fix <= 0 {
            return Err(IndexError::NonPositiveFix(leg.ticker));
        }

        // q = w · BASE / P(t0), with the weight applied in basis points so the
        // division happens once per leg rather than twice.
        let weighted = BASE_LEVEL * leg.weight_bp / 10_000;
        let quantity = fdiv(weighted, base_fix).ok_or(IndexError::NonPositiveFix(leg.ticker))?;

        start = start.max(daily.start);
        end = end.min(daily.end());
        quantities.push(Quantity {
            leg: *leg,
            base_fix,
            quantity,
        });
    }

    if end < start {
        return Err(IndexError::NoOverlap);
    }

    let mut values = Vec::with_capacity((end - start + 1) as usize);
    for day in start..=end {
        let mut level: i128 = 0;
        for q in &quantities {
            let daily = series
                .iter()
                .find(|(t, _)| *t == q.leg.ticker)
                .map(|(_, d)| d)
                .ok_or(IndexError::MissingLeg(q.leg.ticker))?;
            let fix = daily
                .value_on(day)
                .ok_or(IndexError::BaseFixUnavailable(q.leg.ticker))?;
            level += fmul(q.quantity, fix);
        }
        values.push(level);
    }

    Ok(Index {
        base_day,
        quantities,
        level: Daily { start, values },
    })
}

impl Index {
    /// the latest published level.
    pub fn latest(&self) -> Option<(i64, i128)> {
        self.level
            .values
            .last()
            .map(|v| (self.level.end(), *v))
    }

    /// the level `days` calendar days before the end, for a change figure.
    pub fn level_before(&self, days: i64) -> Option<i128> {
        self.level.value_on(self.level.end() - days)
    }

    /// each leg's share of the current level, in basis points — the weights
    /// drift from their starting values as prices move.
    pub fn current_shares(&self, series: &[(&str, Daily)]) -> Vec<(Leg, i128)> {
        let day = self.level.end();
        let mut out = Vec::with_capacity(self.quantities.len());
        let mut total = 0i128;
        for q in &self.quantities {
            let fix = series
                .iter()
                .find(|(t, _)| *t == q.leg.ticker)
                .and_then(|(_, d)| d.value_on(day))
                .unwrap_or(0);
            let value = fmul(q.quantity, fix);
            total += value;
            out.push((q.leg, value));
        }
        if total > 0 {
            for entry in out.iter_mut() {
                entry.1 = entry.1 * 10_000 / total;
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::num::parse_fixed;

    fn fixed(s: &str) -> i128 {
        parse_fixed(s).expect("valid number")
    }

    fn flat(value: &str, len: usize) -> Daily {
        Daily {
            start: 0,
            values: vec![fixed(value); len],
        }
    }

    fn all_flat(len: usize) -> Vec<(&'static str, Daily)> {
        LEGS.iter()
            .map(|l| (l.ticker, flat("100", len)))
            .collect()
    }

    #[test]
    fn weights_sum_to_ten_thousand_basis_points() {
        let total: i128 = LEGS.iter().map(|l| l.weight_bp).sum();
        assert_eq!(total, 10_000);
    }

    #[test]
    fn index_reads_base_level_on_the_base_date() {
        let series = all_flat(3);
        let index = build(&series, 0).expect("builds");
        assert_eq!(index.level.values[0], BASE_LEVEL);
        // prices unchanged: the level stays put
        assert_eq!(index.level.values[2], BASE_LEVEL);
    }

    #[test]
    fn doubling_every_price_doubles_the_level() {
        let mut series = all_flat(2);
        for (_, daily) in series.iter_mut() {
            daily.values[1] = fixed("200");
        }
        let index = build(&series, 0).expect("builds");
        assert_eq!(index.level.values[1], BASE_LEVEL * 2);
    }

    #[test]
    fn a_leg_going_to_zero_costs_at_most_its_weight() {
        // T2: an asset falling — even to zero — triggers nothing; the sleeve
        // rides down. BTC carries 2000bp, so the level floors at 80.
        let mut series = all_flat(2);
        for (ticker, daily) in series.iter_mut() {
            if *ticker == "BTC" {
                daily.values[1] = 1; // one unit above zero: still a price
            }
        }
        let index = build(&series, 0).expect("builds");
        let level = index.level.values[1];
        assert!(level < BASE_LEVEL * 81 / 100, "level was {level}");
        assert!(level > BASE_LEVEL * 79 / 100, "level was {level}");
    }

    #[test]
    fn rejects_a_missing_or_unusable_leg() {
        let short: Vec<(&str, Daily)> = all_flat(2).into_iter().take(7).collect();
        assert!(matches!(
            build(&short, 0),
            Err(IndexError::MissingLeg("UX"))
        ));

        let mut zeroed = all_flat(2);
        zeroed[0].1.values[0] = 0;
        assert!(matches!(
            build(&zeroed, 0),
            Err(IndexError::NonPositiveFix("BTC"))
        ));
    }

    #[test]
    fn shares_start_at_the_declared_weights() {
        let series = all_flat(2);
        let index = build(&series, 0).expect("builds");
        let shares = index.current_shares(&series);
        for (leg, share) in shares {
            assert_eq!(share, leg.weight_bp, "leg {}", leg.ticker);
        }
    }
}

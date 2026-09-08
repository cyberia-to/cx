//! the basket and the index level.
//!
//! `q_i = w_i · BASE / P_i(t0)` fixes the quantities once, at the base date.
//! from then on `I(t) = Σ q_i · P_i(t)` — whoever owes the index owes
//! quantities, and the quantities never change.

use crate::num::{fdiv, fmul, SCALE};
use crate::series::Daily;

/// one CX is what the basket cost on the base date: a dollar. the level is
/// therefore a price — "1 CX = $69.03" — rather than an abstract index number,
/// so a lease written at 100 CX reads straight off it.
pub const BASE_LEVEL: i128 = SCALE;

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

    /// decimals when printing a quantity: enough for four significant digits,
    /// whether the leg holds a yuan or a fraction of a tonne.
    pub fn quantity_places(&self, value: i128) -> u32 {
        let v = value.abs();
        match v {
            _ if v >= SCALE => 3,
            _ if v >= SCALE / 100 => 5,
            _ if v >= SCALE / 10_000 => 7,
            _ => 9,
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

/// the collar of §2: a year's rent may rise at most 35% and fall at most 15%
/// against the year before, measured in the bitcoin numéraire.
pub const COLLAR_UP_BP: i128 = 13_500;
pub const COLLAR_DOWN_BP: i128 = 8_500;

/// one anniversary of a lease written on the base date.
#[derive(Debug, Clone, Copy)]
pub struct Anniversary {
    pub day: i64,
    /// the basket priced in bitcoin, before the collar
    pub uncollared: i128,
    /// the rent actually owed, collared and floored
    pub rent_btc: i128,
    /// what that rent invoices in dollars
    pub invoice: i128,
}

impl Index {
    /// walk the lease machinery of §2 across the published decade: price the
    /// basket in bitcoin, collar each annual step, hold the dual floor, and
    /// convert back to dollars at the same fix.
    ///
    /// `btc` is the BTC/USD trailing average that serves as the ruler; `F` is
    /// the year-zero rent in dollars, which for a one-dollar CX is [`BASE_LEVEL`].
    pub fn collared_path(&self, btc: &Daily) -> Vec<Anniversary> {
        let mut out: Vec<Anniversary> = Vec::new();
        let mut year = 0i64;
        let mut prev_rent: Option<i128> = None;
        let mut s0 = 0i128;

        loop {
            let (y, m, d) = crate::num::civil_from_days(self.base_day);
            let day = crate::num::days_from_civil(y + year, m, d);
            if day > self.level.end() {
                break;
            }
            let (Some(level), Some(x)) = (self.level.value_on(day), btc.value_on(day)) else {
                break;
            };
            let Some(s) = fdiv(level, x) else { break };

            let mut rent = match prev_rent {
                None => {
                    s0 = s;
                    s
                }
                Some(prev) => {
                    let hi = prev * COLLAR_UP_BP / 10_000;
                    let lo = prev * COLLAR_DOWN_BP / 10_000;
                    s.clamp(lo, hi)
                }
            };

            // dual floor: never fewer satoshi than year zero, never fewer
            // year-zero dollars
            if let Some(fiat_leg) = fdiv(BASE_LEVEL, x) {
                rent = rent.max(s0.max(fiat_leg));
            }

            out.push(Anniversary {
                day,
                uncollared: s,
                rent_btc: rent,
                invoice: fmul(rent, x),
            });
            prev_rent = Some(rent);
            year += 1;
        }
        out
    }
}

#[cfg(test)]
mod collar_tests {
    use super::*;
    use crate::num::{days_from_civil, parse_fixed};

    fn fixed(s: &str) -> i128 {
        parse_fixed(s).expect("valid number")
    }

    /// eleven anniversaries of daily values, starting at 2016-01-01.
    fn daily(len: usize, f: impl Fn(usize) -> i128) -> Daily {
        Daily {
            start: days_from_civil(2016, 1, 1),
            values: (0..len).map(f).collect(),
        }
    }

    fn index_with(level: Daily) -> Index {
        Index {
            base_day: days_from_civil(2016, 1, 1),
            quantities: Vec::new(),
            level,
        }
    }

    #[test]
    fn a_runaway_basket_is_capped_at_the_collar() {
        // the basket triples every year while the ruler stands still: the rent
        // may still only climb 35% a year
        let idx = index_with(daily(1500, |i| {
            fixed("1") * (1 + (i as i128) / 365 * 2)
        }));
        let btc = daily(1500, |_| fixed("1000"));
        let path = idx.collared_path(&btc);
        assert!(path.len() >= 4);
        for pair in path.windows(2) {
            let ratio = pair[1].rent_btc * 10_000 / pair[0].rent_btc;
            assert!(ratio <= COLLAR_UP_BP + 1, "rose {ratio} bp in one year");
        }
    }

    #[test]
    fn a_collapsing_basket_is_held_by_the_floor() {
        // the basket falls away, but the sat floor holds the rent at year zero
        let idx = index_with(daily(1500, |i| {
            (fixed("1") - (i as i128) * fixed("0.0005")).max(fixed("0.001"))
        }));
        let btc = daily(1500, |_| fixed("1000"));
        let path = idx.collared_path(&btc);
        let first = path.first().expect("a first year").rent_btc;
        for a in &path {
            assert!(a.rent_btc >= first, "rent fell below the year-zero floor");
        }
    }

    #[test]
    fn a_flat_world_never_moves_the_invoice() {
        let idx = index_with(daily(1500, |_| fixed("1")));
        let btc = daily(1500, |_| fixed("1000"));
        let path = idx.collared_path(&btc);
        let first = path.first().expect("a first year").invoice;
        for a in &path {
            assert_eq!(a.invoice, first);
        }
    }

    #[test]
    fn the_path_lands_on_anniversaries() {
        let idx = index_with(daily(1500, |_| fixed("1")));
        let btc = daily(1500, |_| fixed("1000"));
        let path = idx.collared_path(&btc);
        assert_eq!(path[0].day, days_from_civil(2016, 1, 1));
        assert_eq!(path[1].day, days_from_civil(2017, 1, 1));
    }
}

impl Index {
    /// the annual rate the collared path actually delivered, in fixed-point
    /// percent. the geometric root is floating point — this is a figure for a
    /// slider default and a sentence of copy, never an input to the index.
    pub fn collared_annual_rate(&self, btc: &Daily) -> Option<i128> {
        let path = self.collared_path(btc);
        let (first, last) = (path.first()?.invoice, path.last()?.invoice);
        if first <= 0 || path.len() < 2 {
            return None;
        }
        let years = (path.len() - 1) as f64;
        let ratio = last as f64 / first as f64;
        let rate = ratio.powf(1.0 / years) - 1.0;
        Some((rate * 100.0 * SCALE as f64) as i128)
    }
}

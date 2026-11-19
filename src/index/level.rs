//! the level: the daily chain of weighted returns, bounded by the death rule.
//!
//! `I(t) = I(t−1) · Σ wᵢ · P̄ᵢ(t)/P̄ᵢ(t−1)`, the sum taken over the living legs
//! and divided by their own weight — never by rounded pro-rata weights, whose
//! truncation would bleed the level daily.

use super::basket::{Death, Holding, DEATH_BP, LEGS};
use crate::num::{fdiv, fmul, SCALE};
use crate::series::Daily;

/// one CX is what the basket cost on the base date: a dollar. the level is
/// therefore a price — "1 CX = $1.31" — rather than an abstract index number,
/// so a lease written at 100 CX reads straight off it.
pub const BASE_LEVEL: i128 = SCALE;

/// the computed index: the daily level series, the base date, the peak
/// trailing average of every leg and the deaths the rule has recorded.
#[derive(Debug, Clone)]
pub struct Index {
    pub base_day: i64,
    pub level: Daily,
    /// the highest trailing average of each leg since the base date, in
    /// publication order
    pub peaks: Vec<i128>,
    pub deaths: Vec<Death>,
}

/// why an index could not be built.
#[derive(Debug)]
pub enum IndexError {
    WeightsDoNotSum(i128),
    MissingLeg(&'static str),
    BaseFixUnavailable(&'static str),
    NonPositiveFix(&'static str),
    NoOverlap,
    EveryLegDead,
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
            IndexError::EveryLegDead => write!(f, "every leg has left the basket"),
        }
    }
}

/// the live weights after the dead legs' weight is spread pro rata over the
/// living — a dead leg reads zero. each share truncates to the basis point, so
/// the sum lands at or just below 10_000, short by up to one point per living
/// leg: 9_996 after a 15% leg dies, 9_995 after a 5% one. the daily factor
/// never reads these — [`build`] divides by the living weight itself — so the
/// shortfall is in the published figures and nowhere else.
pub fn live_weights(alive: &[bool]) -> Vec<i128> {
    let total: i128 = LEGS
        .iter()
        .zip(alive)
        .filter(|(_, a)| **a)
        .map(|(l, _)| l.weight_bp)
        .sum();
    LEGS.iter()
        .zip(alive)
        .map(|(l, a)| if *a && total > 0 { l.weight_bp * 10_000 / total } else { 0 })
        .collect()
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

    let mut legs: Vec<&Daily> = Vec::with_capacity(LEGS.len());
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
        start = start.max(daily.start);
        end = end.min(daily.end());
        legs.push(daily);
    }
    if end < start {
        return Err(IndexError::NoOverlap);
    }

    let fix_on = |i: usize, day: i64| -> Result<i128, IndexError> {
        let v = legs[i]
            .value_on(day)
            .ok_or(IndexError::BaseFixUnavailable(LEGS[i].ticker))?;
        if v <= 0 {
            return Err(IndexError::NonPositiveFix(LEGS[i].ticker));
        }
        Ok(v)
    };

    let mut alive = vec![true; LEGS.len()];
    let mut peaks: Vec<i128> = (0..LEGS.len())
        .map(|i| fix_on(i, start))
        .collect::<Result<_, _>>()?;
    let mut deaths: Vec<Death> = Vec::new();

    // the level is carried at twice the scale so that a day's truncation
    // cannot accumulate into the published digits over a century of steps
    let mut level_hi: i128 = BASE_LEVEL * SCALE;
    let mut values = Vec::with_capacity((end - start + 1) as usize);
    values.push(BASE_LEVEL);

    for day in (start + 1)..=end {
        // the day's factor: the weighted mean of the living legs' daily
        // ratios, divided by the living weight itself — never by rounded
        // pro-rata weights, whose truncation would bleed the level daily
        let mut weighted: i128 = 0;
        let mut living_bp: i128 = 0;
        for i in 0..LEGS.len() {
            if !alive[i] {
                continue;
            }
            let ratio = fdiv(fix_on(i, day)?, fix_on(i, day - 1)?)
                .ok_or(IndexError::NonPositiveFix(LEGS[i].ticker))?;
            weighted += LEGS[i].weight_bp * ratio;
            living_bp += LEGS[i].weight_bp;
        }
        if living_bp == 0 {
            return Err(IndexError::EveryLegDead);
        }
        let factor = weighted / living_bp;
        level_hi = level_hi * factor / SCALE;
        values.push(level_hi / SCALE);

        // the death rule, read after the day's return is taken: the fall that
        // crossed the line is paid, and from tomorrow the leg is gone
        for i in 0..LEGS.len() {
            if !alive[i] {
                continue;
            }
            let fix = fix_on(i, day)?;
            if fix > peaks[i] {
                peaks[i] = fix;
            }
            if fix * 10_000 < peaks[i] * DEATH_BP {
                alive[i] = false;
                deaths.push(Death {
                    leg: LEGS[i],
                    day,
                    peak: peaks[i],
                    fix,
                });
            }
        }
    }

    Ok(Index {
        base_day,
        level: Daily { start, values },
        peaks,
        deaths,
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

    /// which legs are alive on the latest day.
    pub fn alive(&self) -> Vec<bool> {
        LEGS.iter()
            .map(|l| !self.deaths.iter().any(|d| d.leg.ticker == l.ticker))
            .collect()
    }

    /// every leg as the index holds it on its latest day: live weight, the
    /// quantity one CX holds, and how far the trailing average sits from its peak.
    pub fn holdings(&self, series: &[(&str, Daily)]) -> Vec<Holding> {
        let Some((day, level)) = self.latest() else {
            return Vec::new();
        };
        let alive = self.alive();
        let weights = live_weights(&alive);
        LEGS.iter()
            .enumerate()
            .map(|(i, leg)| {
                let fix = series
                    .iter()
                    .find(|(t, _)| *t == leg.ticker)
                    .and_then(|(_, d)| d.value_on(day))
                    .unwrap_or(0);
                let quantity = fdiv(fmul(level, weights[i] * SCALE / 10_000), fix).unwrap_or(0);
                let peak = self.peaks.get(i).copied().unwrap_or(0);
                let of_peak_bp = if peak > 0 { fix * 10_000 / peak } else { 0 };
                Holding {
                    leg: *leg,
                    weight_bp: weights[i],
                    quantity,
                    of_peak_bp,
                    alive: alive[i],
                }
            })
            .collect()
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

    fn set(series: &mut [(&str, Daily)], ticker: &str, day: usize, value: i128) {
        for (t, d) in series.iter_mut() {
            if *t == ticker {
                d.values[day] = value;
            }
        }
    }

    #[test]
    fn index_reads_base_level_on_the_base_date() {
        let series = all_flat(3);
        let index = build(&series, 0).expect("builds");
        assert_eq!(index.level.values[0], BASE_LEVEL);
        // prices unchanged: the level stays put, to the last digit
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
    fn a_day_earns_the_weighted_return_of_the_legs() {
        // BTC alone doubles: the index gains its 20% weight
        let mut series = all_flat(2);
        set(&mut series, "BTC", 1, fixed("200"));
        let index = build(&series, 0).expect("builds");
        assert_eq!(index.level.values[1], BASE_LEVEL * 12 / 10);
    }

    #[test]
    fn the_basket_is_rebalanced_every_day() {
        // BTC doubles, then halves back to where it started. a buy-and-hold
        // basket would be back at $1; the rebalanced one sold half the gain
        // at the top: (1 + 0.2)(1 − 0.1) = 1.08
        let mut series = all_flat(3);
        set(&mut series, "BTC", 1, fixed("200"));
        set(&mut series, "BTC", 2, fixed("100"));
        let index = build(&series, 0).expect("builds");
        assert_eq!(index.level.values[2], BASE_LEVEL * 108 / 100);
    }

    #[test]
    fn holdings_sit_at_the_weights_whatever_prices_did() {
        let mut series = all_flat(3);
        set(&mut series, "GOLD", 1, fixed("300"));
        set(&mut series, "GOLD", 2, fixed("250"));
        set(&mut series, "ETH", 2, fixed("40"));
        let index = build(&series, 0).expect("builds");
        let level = index.level.values[2];
        for h in index.holdings(&series) {
            assert_eq!(h.weight_bp, h.leg.weight_bp, "leg {}", h.leg.ticker);
            // q · P = w · I, to within a fixed-point digit
            let fix = series
                .iter()
                .find(|(t, _)| *t == h.leg.ticker)
                .map(|(_, d)| d.values[2])
                .expect("leg present");
            let value = fmul(h.quantity, fix);
            let target = level * h.leg.weight_bp / 10_000;
            // the quantity is truncated to a digit, which the price scales up
            let tolerance = fix / SCALE + 2;
            assert!((value - target).abs() <= tolerance, "leg {} holds {value} vs {target}", h.leg.ticker);
        }
    }

    #[test]
    fn a_leg_below_a_tenth_of_its_peak_leaves_and_never_returns() {
        // UX runs to 200, then collapses to 19 — below a tenth of its peak.
        // the day it crosses is paid; afterwards its moves no longer count.
        let mut series = all_flat(5);
        set(&mut series, "UX", 1, fixed("200"));
        set(&mut series, "UX", 2, fixed("19"));
        set(&mut series, "UX", 3, fixed("1900"));
        set(&mut series, "UX", 4, fixed("1"));
        let index = build(&series, 0).expect("builds");
        assert_eq!(index.deaths.len(), 1);
        assert_eq!(index.deaths[0].leg.ticker, "UX");
        assert_eq!(index.deaths[0].day, 2);
        assert_eq!(index.deaths[0].peak, fixed("200"));
        // the recovery and the second collapse leave the level untouched
        assert_eq!(index.level.values[3], index.level.values[2]);
        assert_eq!(index.level.values[4], index.level.values[2]);
        // the dead leg's weight went pro rata to the living seven
        let h = index.holdings(&series);
        let ux = h.iter().find(|h| h.leg.ticker == "UX").expect("UX");
        assert!(!ux.alive);
        assert_eq!(ux.weight_bp, 0);
        let btc = h.iter().find(|h| h.leg.ticker == "BTC").expect("BTC");
        assert_eq!(btc.weight_bp, 2000 * 10_000 / 9_500);
        // the shares truncate to the basis point, so the published weights
        // fall short of 100% and never run over it: 9,995 once this 5% leg
        // is gone, at most one point short per living leg
        let live: i128 = h.iter().map(|h| h.weight_bp).sum();
        assert_eq!(live, 9_995, "live weights sum to {live}");
    }

    #[test]
    fn a_leg_at_exactly_a_tenth_of_its_peak_lives() {
        let mut series = all_flat(2);
        set(&mut series, "OIL", 1, fixed("10"));
        let index = build(&series, 0).expect("builds");
        assert!(index.deaths.is_empty());
        let h = index.holdings(&series);
        let oil = h.iter().find(|h| h.leg.ticker == "OIL").expect("OIL");
        assert_eq!(oil.of_peak_bp, 1_000);
    }

    #[test]
    fn a_dying_leg_costs_at_most_a_tenth_to_the_power_of_its_weight() {
        // BTC loses 1% a day for a year. without the death rule the rebalanced
        // index would follow it down without bound; with it the loss stops at
        // 1 − 0.1^0.2 ≈ 37%, taken in small steps as the leg falls to a tenth.
        let days = 365;
        let mut series = all_flat(days);
        let mut p = fixed("100");
        for d in 1..days {
            p = p * 99 / 100;
            set(&mut series, "BTC", d, p);
        }
        let index = build(&series, 0).expect("builds");
        assert_eq!(index.deaths.len(), 1);
        let level = index.level.values[days - 1];
        assert!(level > BASE_LEVEL * 62 / 100, "level was {level}");
        assert!(level < BASE_LEVEL * 64 / 100, "level was {level}");
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
    fn an_index_whose_every_leg_has_died_refuses_to_go_on() {
        // every leg closes below a tenth of its peak on the same day: that
        // fall is paid, and the day after has no basket left to price
        let mut series = all_flat(3);
        for (_, daily) in series.iter_mut() {
            daily.values[1] = fixed("9");
            daily.values[2] = fixed("9");
        }
        assert!(matches!(build(&series, 0), Err(IndexError::EveryLegDead)));
    }

    #[test]
    fn a_century_of_flat_days_does_not_drift() {
        let series = all_flat(36_600);
        let index = build(&series, 0).expect("builds");
        assert_eq!(index.level.values[36_599], BASE_LEVEL);
    }
}

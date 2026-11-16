//! the basket and the index level.
//!
//! the index is its weights. each day CX earns the weighted return of its
//! legs at trailing-average prices — `I(t) = I(t−1) · Σ wᵢ · P̄ᵢ(t)/P̄ᵢ(t−1)` —
//! so the basket is rebalanced to its weights every day and holds the same
//! portfolio for everyone on every day, whatever the base date.
//!
//! daily rebalancing buys what falls, so a leg going to zero would take the
//! index with it, geometrically at the power of its weight. the death rule
//! bounds that: a leg leaves the basket the day its trailing average closes
//! below a tenth of its highest trailing average since the base date, its
//! weight goes pro rata to the rest, and it never returns.

use crate::num::{fdiv, fmul, SCALE};
use crate::series::Daily;

/// one CX is what the basket cost on the base date: a dollar. the level is
/// therefore a price — "1 CX = $1.41" — rather than an abstract index number,
/// so a lease written at 100 CX reads straight off it.
pub const BASE_LEVEL: i128 = SCALE;

/// the trailing window of §3: every price enters as a 365-day average.
pub const TWAP_WINDOW: usize = 365;

/// the death rule of §2: a leg whose trailing average closes below this share
/// of its peak trailing average leaves the basket. in basis points.
pub const DEATH_BP: i128 = 1_000;

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
/// the unit its holding is denominated in.
#[derive(Debug, Clone, Copy)]
pub struct Leg {
    pub ticker: &'static str,
    pub group: Group,
    /// weight in basis points of the index — integers, so the weights sum
    /// exactly to 10_000 and no rounding drift enters the daily factor.
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
        source: "PAXG-USD daily close, Yahoo Finance (one token = one troy ounce; LBMA PM fix as fallback)",
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

    /// decimals when printing a holding: enough for four significant digits,
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

/// a leg that left the basket under the death rule.
#[derive(Debug, Clone)]
pub struct Death {
    pub leg: Leg,
    /// the day the trailing average closed below a tenth of its peak
    pub day: i64,
    pub peak: i128,
    pub fix: i128,
}

/// one leg as the index holds it on its latest day.
#[derive(Debug, Clone)]
pub struct Holding {
    pub leg: Leg,
    /// the leg's weight after any deaths redistributed, in basis points;
    /// zero for a dead leg
    pub weight_bp: i128,
    /// how much of the leg one CX holds today — `w · I(t) / P̄(t)`
    pub quantity: i128,
    /// today's trailing average as a share of the peak, in basis points —
    /// the gauge the death rule reads
    pub of_peak_bp: i128,
    pub alive: bool,
}

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
/// living — a dead leg reads zero. the sum is 10_000 while any leg lives.
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
    fn weights_sum_to_ten_thousand_basis_points() {
        let total: i128 = LEGS.iter().map(|l| l.weight_bp).sum();
        assert_eq!(total, 10_000);
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
        let live: i128 = h.iter().map(|h| h.weight_bp).sum();
        assert!((live - 10_000).abs() < LEGS.len() as i128, "live weights sum to {live}");
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
    fn a_century_of_flat_days_does_not_drift() {
        let series = all_flat(36_600);
        let index = build(&series, 0).expect("builds");
        assert_eq!(index.level.values[36_599], BASE_LEVEL);
    }
}

/// the collar of §2: a year's rent may rise at most 35% and fall at most 15%
/// against the year before, measured in the bitcoin numéraire.
pub const COLLAR_UP_BP: i128 = 13_500;
pub const COLLAR_DOWN_BP: i128 = 8_500;

/// what decided a year's rent under §2.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Binding {
    /// the basket's own price passed through
    Basket,
    /// the collar capped the rise at +35%
    CollarUp,
    /// the collar held the fall at −15%
    CollarDown,
    /// the satoshi floor: no fewer sats than year zero
    SatFloor,
    /// the dollar floor: no fewer year-zero dollars
    FiatFloor,
}

impl Binding {
    pub fn label(&self) -> &'static str {
        match self {
            Binding::Basket => "basket",
            Binding::CollarUp => "collar, rise capped",
            Binding::CollarDown => "collar, fall held",
            Binding::SatFloor => "satoshi floor",
            Binding::FiatFloor => "dollar floor",
        }
    }
}

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
    /// which rule of §2 set the rent this year
    pub binding: Binding,
}

impl Index {
    /// walk the lease machinery of §2 from the base date: price the basket in
    /// bitcoin, collar each annual step, hold the dual floor, and convert back
    /// to dollars at the same fix. year zero is the definition R(0) = S₀ and
    /// takes no floor.
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

            let (mut rent, mut binding) = match prev_rent {
                None => {
                    s0 = s;
                    (s, Binding::Basket)
                }
                Some(prev) => {
                    let hi = prev * COLLAR_UP_BP / 10_000;
                    let lo = prev * COLLAR_DOWN_BP / 10_000;
                    if s > hi {
                        (hi, Binding::CollarUp)
                    } else if s < lo {
                        (lo, Binding::CollarDown)
                    } else {
                        (s, Binding::Basket)
                    }
                }
            };

            // dual floor: never fewer satoshi than year zero, never fewer
            // year-zero dollars. the two legs are compared as sats at this
            // year's fix, and whichever is higher is the floor that binds.
            if prev_rent.is_some() {
                if let Some(fiat_leg) = fdiv(BASE_LEVEL, x) {
                    let floor = s0.max(fiat_leg);
                    if floor > rent {
                        rent = floor;
                        binding = if fiat_leg > s0 {
                            Binding::FiatFloor
                        } else {
                            Binding::SatFloor
                        };
                    }
                }
            }

            out.push(Anniversary {
                day,
                uncollared: s,
                rent_btc: rent,
                invoice: fmul(rent, x),
                binding,
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
            level,
            peaks: Vec::new(),
            deaths: Vec::new(),
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
        assert_eq!(path[0].binding, Binding::Basket);
        assert!(path[1..].iter().all(|a| a.binding == Binding::CollarUp));
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
        // a flat ruler makes both floor legs equal, and the sat leg is named
        assert!(path[1..].iter().all(|a| a.binding == Binding::SatFloor));
    }

    #[test]
    fn a_flat_world_never_moves_the_invoice() {
        let idx = index_with(daily(1500, |_| fixed("1")));
        let btc = daily(1500, |_| fixed("1000"));
        let path = idx.collared_path(&btc);
        let first = path.first().expect("a first year").invoice;
        for a in &path {
            assert_eq!(a.invoice, first);
            assert_eq!(a.binding, Binding::Basket);
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

//! the eight legs and what the basket is made of.
//!
//! the weights are the index: integers in basis points, summing to 10_000, so
//! no rounding drift enters the daily factor. the trailing window and the
//! death rule's threshold sit here too — they define the basket, not the
//! arithmetic that walks it.

use crate::num::SCALE;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weights_sum_to_ten_thousand_basis_points() {
        let total: i128 = LEGS.iter().map(|l| l.weight_bp).sum();
        assert_eq!(total, 10_000);
    }
}

//! the rent path: what a lease written on the base date invoices.
//!
//! rent is a number of CX a year; on each anniversary the index is what that
//! CX invoiced, and nothing else sets it. the audit of 2026-11-16 retired the
//! bitcoin ruler, the collar and the floor that used to sit in between.

use super::level::Index;
use crate::num::SCALE;

/// one anniversary of a lease written on the base date: the index on that
/// day is what one CX of rent invoiced, and nothing else sets it.
#[derive(Debug, Clone, Copy)]
pub struct Anniversary {
    pub day: i64,
    /// the index level — the invoice per CX of rent, in dollars
    pub invoice: i128,
}

impl Index {
    /// the rent path of §2 from the base date: one CX a year, read off the
    /// index on each anniversary. no ruler, no collar, no floor — the audit
    /// of 2026-11-16 retired them.
    pub fn rent_path(&self) -> Vec<Anniversary> {
        let (y, m, d) = crate::num::civil_from_days(self.base_day);
        let mut out = Vec::new();
        let mut year = 0i64;
        loop {
            let day = crate::num::days_from_civil(y + year, m, d);
            let Some(invoice) = self.level.value_on(day) else { break };
            if day > self.level.end() {
                break;
            }
            out.push(Anniversary { day, invoice });
            year += 1;
        }
        out
    }

    /// the annual rate the index delivered between the first and the last
    /// anniversary, in fixed-point percent. the geometric root is floating
    /// point — this is a figure for a slider default and a sentence of copy,
    /// never an input to the index.
    pub fn annual_rate(&self) -> Option<i128> {
        let path = self.rent_path();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::num::{days_from_civil, parse_fixed};
    use crate::series::Daily;

    fn fixed(s: &str) -> i128 {
        parse_fixed(s).expect("valid number")
    }

    fn index_with(len: usize, f: impl Fn(usize) -> i128) -> Index {
        Index {
            base_day: days_from_civil(2016, 1, 1),
            level: Daily {
                start: days_from_civil(2016, 1, 1),
                values: (0..len).map(f).collect(),
            },
            peaks: Vec::new(),
            deaths: Vec::new(),
        }
    }

    #[test]
    fn the_path_lands_on_anniversaries_and_reads_the_index() {
        let idx = index_with(1500, |i| fixed("1") + i as i128 * SCALE / 1000);
        let path = idx.rent_path();
        assert_eq!(path.len(), 5);
        assert_eq!(path[0].day, days_from_civil(2016, 1, 1));
        assert_eq!(path[1].day, days_from_civil(2017, 1, 1));
        // 2016 is a leap year: 366 days on
        assert_eq!(path[1].invoice, fixed("1") + 366 * SCALE / 1000);
    }

    #[test]
    fn a_flat_world_never_moves_the_invoice_and_rates_at_zero() {
        let idx = index_with(1500, |_| fixed("1"));
        let path = idx.rent_path();
        assert!(path.iter().all(|a| a.invoice == fixed("1")));
        assert_eq!(idx.annual_rate(), Some(0));
    }

    #[test]
    fn a_doubling_over_four_years_rates_at_nineteen_percent() {
        let idx = index_with(1500, |i| if i >= 1461 { fixed("2") } else { fixed("1") });
        let rate = idx.annual_rate().expect("a rate");
        // 2^(1/4) − 1 = 18.92%
        assert!(rate > 18 * SCALE && rate < 19 * SCALE, "rate was {rate}");
    }
}

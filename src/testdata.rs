//! the fixture the publication tests share: one built index with a death in it.

use crate::index::{build, Index, LEGS};
use crate::num::parse_fixed;
use crate::series::Daily;

/// a flat world of $100 fixes over 400 days in which UX collapses to $9 on the
/// third day — below a tenth of its peak — and leaves the basket for good. the
/// level pays that fall once, $1 → $0.9545, and never moves again.
pub fn one_death() -> (Index, Vec<(&'static str, Daily)>) {
    let flat = parse_fixed("100").expect("a number");
    let collapsed = parse_fixed("9").expect("a number");
    let mut series: Vec<(&'static str, Daily)> = LEGS
        .iter()
        .map(|l| {
            (
                l.ticker,
                Daily {
                    start: 0,
                    values: vec![flat; 400],
                },
            )
        })
        .collect();
    for (ticker, daily) in series.iter_mut() {
        if *ticker == "UX" {
            for v in daily.values[2..].iter_mut() {
                *v = collapsed;
            }
        }
    }
    let index = build(&series, 0).expect("the index builds");
    (index, series)
}

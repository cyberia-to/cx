//! daily price series: sparse observations in, a dense carried-forward
//! calendar out, then the trailing 365-day average the index consumes.

use crate::num::{format_date, SCALE};

/// a sparse series of observations, one value per date (days since epoch).
/// observations arrive unsorted from the feeds and are normalised on insert.
#[derive(Debug, Default, Clone)]
pub struct Series {
    points: Vec<(i64, i128)>,
}

impl Series {
    pub fn new() -> Self {
        Self { points: Vec::new() }
    }

    pub fn push(&mut self, day: i64, value: i128) {
        self.points.push((day, value));
    }

    pub fn len(&self) -> usize {
        self.points.len()
    }

    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    /// sort by date and drop duplicate dates, keeping the last observation —
    /// feeds occasionally restate a day, and the later value is the revision.
    pub fn normalise(&mut self) {
        self.points.sort_by_key(|(d, _)| *d);
        let mut deduped: Vec<(i64, i128)> = Vec::with_capacity(self.points.len());
        for (day, value) in self.points.drain(..) {
            match deduped.last_mut() {
                Some((last_day, last_value)) if *last_day == day => *last_value = value,
                _ => deduped.push((day, value)),
            }
        }
        self.points = deduped;
    }

    pub fn first_day(&self) -> Option<i64> {
        self.points.first().map(|(d, _)| *d)
    }

    pub fn last(&self) -> Option<(i64, i128)> {
        self.points.last().copied()
    }

    pub fn point(&self, i: usize) -> Option<(i64, i128)> {
        self.points.get(i).copied()
    }

    /// the observed range, for the provenance manifest.
    pub fn span_text(&self) -> String {
        match (self.points.first(), self.points.last()) {
            (Some((a, _)), Some((b, _))) => {
                format!("{} … {}", format_date(*a), format_date(*b))
            }
            _ => "—".to_string(),
        }
    }

    /// expand into one value per calendar day over `[from, to]`, carrying the
    /// last observation forward across weekends, holidays and — for the
    /// monthly IMF legs — the rest of the month. §3 of the protocol: a closed
    /// market carries its last fix forward.
    ///
    /// returns `None` when no observation exists at or before `from`, since
    /// carrying a *later* price backwards would invent history.
    pub fn carry_forward(&self, from: i64, to: i64) -> Option<Vec<i128>> {
        if to < from || self.points.is_empty() {
            return None;
        }
        let mut idx = 0usize;
        let mut current: Option<i128> = None;
        while idx < self.points.len() && self.points[idx].0 <= from {
            current = Some(self.points[idx].1);
            idx += 1;
        }
        let mut current = current?;
        let mut out = Vec::with_capacity((to - from + 1) as usize);
        for day in from..=to {
            while idx < self.points.len() && self.points[idx].0 <= day {
                current = self.points[idx].1;
                idx += 1;
            }
            out.push(current);
        }
        Some(out)
    }
}

/// a dense daily calendar of fixes with its start date.
#[derive(Debug, Clone)]
pub struct Daily {
    pub start: i64,
    pub values: Vec<i128>,
}

impl Daily {
    pub fn value_on(&self, day: i64) -> Option<i128> {
        if day < self.start {
            return None;
        }
        self.values.get((day - self.start) as usize).copied()
    }

    pub fn end(&self) -> i64 {
        self.start + self.values.len() as i64 - 1
    }

    /// trailing arithmetic mean over `window` calendar days ending on each day,
    /// the "annual TWAP" of §3.
    ///
    /// a leg whose market is younger than the window averages over what it has,
    /// once at least `min_days` of fixes exist — ETH had a four-month-old USD
    /// market at the base date of the published decade, and a shorter honest
    /// window beats a synthetic cross invented to fill the gap. output starts
    /// at the first day carrying `min_days` of history.
    pub fn trailing_mean_min(&self, window: usize, min_days: usize) -> Option<Daily> {
        if window == 0 || min_days == 0 || self.values.len() < min_days {
            return None;
        }
        let mut out = Vec::with_capacity(self.values.len() - min_days + 1);
        let mut sum: i128 = 0;
        for (i, v) in self.values.iter().enumerate() {
            sum += v;
            if i >= window {
                sum -= self.values[i - window];
            }
            let covered = (i + 1).min(window);
            if covered >= min_days {
                out.push(sum / covered as i128);
            }
        }
        if out.is_empty() {
            return None;
        }
        Some(Daily {
            start: self.start + min_days as i64 - 1,
            values: out,
        })
    }

}

/// sanity gate: a fix of zero or below cannot be a price, and would poison
/// every quantity derived from it.
pub fn all_positive(daily: &Daily) -> bool {
    daily.values.iter().all(|v| *v > 0)
}

/// the largest single-day jump in a series, as a fixed-point ratio.
/// feeds do glitch; a 10x overnight move is a data error, not a market.
pub fn max_daily_ratio(daily: &Daily) -> i128 {
    let mut worst = SCALE;
    for pair in daily.values.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        if a <= 0 || b <= 0 {
            continue;
        }
        let ratio = if b > a { b * SCALE / a } else { a * SCALE / b };
        if ratio > worst {
            worst = ratio;
        }
    }
    worst
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::num::{parse_date, parse_fixed};

    fn day(iso: &str) -> i64 {
        parse_date(iso).expect("valid date")
    }

    fn fixed(s: &str) -> i128 {
        parse_fixed(s).expect("valid number")
    }

    #[test]
    fn normalise_sorts_and_keeps_last_restatement() {
        let mut s = Series::new();
        s.push(day("2026-01-03"), fixed("3"));
        s.push(day("2026-01-01"), fixed("1"));
        s.push(day("2026-01-01"), fixed("9"));
        s.normalise();
        assert_eq!(s.len(), 2);
        assert_eq!(s.point(0), Some((day("2026-01-01"), fixed("9"))));
        assert_eq!(s.last(), Some((day("2026-01-03"), fixed("3"))));
    }

    #[test]
    fn carry_forward_fills_gaps_and_refuses_to_invent_history() {
        let mut s = Series::new();
        s.push(day("2026-01-01"), fixed("10"));
        s.push(day("2026-01-04"), fixed("20"));
        s.normalise();

        let dense = s
            .carry_forward(day("2026-01-01"), day("2026-01-05"))
            .expect("covered");
        assert_eq!(
            dense,
            vec![fixed("10"), fixed("10"), fixed("10"), fixed("20"), fixed("20")]
        );

        // nothing observed at or before the start: refuse rather than backfill
        assert!(s
            .carry_forward(day("2025-12-30"), day("2026-01-02"))
            .is_none());
    }

    #[test]
    fn trailing_mean_averages_the_window_and_shifts_the_start() {
        let daily = Daily {
            start: day("2026-01-01"),
            values: vec![fixed("1"), fixed("2"), fixed("3"), fixed("4")],
        };
        let mean = daily.trailing_mean_min(2, 2).expect("window fits");
        assert_eq!(mean.start, day("2026-01-02"));
        assert_eq!(mean.values, vec![fixed("1.5"), fixed("2.5"), fixed("3.5")]);
        assert!(daily.trailing_mean_min(9, 9).is_none());
    }

    #[test]
    fn a_young_series_averages_over_what_it_has() {
        let daily = Daily {
            start: day("2026-01-01"),
            values: vec![fixed("1"), fixed("2"), fixed("3")],
        };
        // window of 365, but only three days exist and two are enough to start
        let mean = daily.trailing_mean_min(365, 2).expect("min window met");
        assert_eq!(mean.start, day("2026-01-02"));
        assert_eq!(mean.values, vec![fixed("1.5"), fixed("2")]);
    }

    #[test]
    fn the_window_stops_growing_once_full() {
        let daily = Daily {
            start: day("2026-01-01"),
            values: vec![fixed("10"), fixed("20"), fixed("30"), fixed("40")],
        };
        let mean = daily.trailing_mean_min(2, 1).expect("builds");
        // day 3 averages only days 3 and 4, not the whole history
        assert_eq!(mean.values.last().copied(), Some(fixed("35")));
    }

    #[test]
    fn guards_catch_bad_fixes() {
        let good = Daily {
            start: 0,
            values: vec![fixed("10"), fixed("11")],
        };
        assert!(all_positive(&good));
        assert_eq!(max_daily_ratio(&good), fixed("1.1"));

        let bad = Daily {
            start: 0,
            values: vec![fixed("10"), fixed("0")],
        };
        assert!(!all_positive(&bad));
    }
}

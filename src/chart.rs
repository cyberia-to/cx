//! the index chart — one view per range, geometry computed here.
//!
//! each view carries its own scale, grid and path, so the page swaps a string
//! rather than recomputing anything in the browser. one series, so no legend:
//! the heading names what is plotted.

use crate::num::{civil_from_days, format_date, format_fixed, format_thousands, SCALE};
use crate::series::Daily;

/// plot area in viewBox units; the SVG scales to its container from there.
const W: i64 = 1000;
const H: i64 = 320;
const PAD_L: i64 = 8;
const PAD_R: i64 = 72;
const PAD_T: i64 = 16;
const PAD_B: i64 = 30;

/// above this ratio a linear axis buries the early years, and the log axis
/// earns its keep.
const LOG_ABOVE: i128 = 4;

pub struct View {
    pub key: &'static str,
    pub label: &'static str,
    /// inner SVG: grid, ticks, path, end marker, hover group
    pub svg: String,
    /// `[[x, y, "level", "YYYY-MM-DD"], …]` for the hover layer
    pub points_json: String,
    pub logarithmic: bool,
}

/// the ranges offered, in tab order. `None` means the whole series.
const RANGES: [(&str, &str, Option<i64>); 4] = [
    ("7d", "7D", Some(7)),
    ("1m", "1M", Some(30)),
    ("1y", "1Y", Some(365)),
    ("all", "ALL", None),
];

/// the 1-2-5 ladder for a log axis.
fn log_ticks(min_v: i128, max_v: i128) -> Vec<i128> {
    let mut ticks = Vec::new();
    let mut decade = 1i128;
    while decade * SCALE * 10 <= min_v {
        decade *= 10;
    }
    'outer: loop {
        for m in [1i128, 2, 5] {
            let tick = m * decade * SCALE;
            if tick > max_v * 2 {
                break 'outer;
            }
            if tick * 2 >= min_v {
                ticks.push(tick);
            }
        }
        decade *= 10;
        if decade > 1_000_000 {
            break;
        }
    }
    ticks
}

/// four to six round gridlines spanning a linear range.
fn linear_ticks(min_v: i128, max_v: i128) -> Vec<i128> {
    let span = (max_v - min_v).max(SCALE / 100);
    let mut step = SCALE / 100;
    let ladder = [1i128, 2, 5];
    'find: loop {
        for m in ladder {
            let candidate = step * m;
            if candidate > 0 && span / candidate <= 5 {
                step = candidate;
                break 'find;
            }
        }
        step *= 10;
        if step > SCALE * 100_000 {
            break;
        }
    }
    let lo = min_v.div_euclid(step) * step;
    let hi = (max_v + step - 1).div_euclid(step) * step;
    let mut ticks = Vec::new();
    let mut t = lo;
    while t <= hi {
        ticks.push(t);
        t += step;
    }
    ticks
}

/// fractional position on a log axis. the index path is integer fixed-point end
/// to end; this is chart geometry, rounded to whole viewBox units.
fn log_pos(v: i128, lo: i128, hi: i128) -> f64 {
    let f = |x: i128| (x as f64 / SCALE as f64).max(1e-9).ln();
    let (lo_l, hi_l, v_l) = (f(lo), f(hi), f(v));
    if (hi_l - lo_l).abs() < f64::EPSILON {
        return 0.0;
    }
    ((v_l - lo_l) / (hi_l - lo_l)).clamp(0.0, 1.0)
}

/// sample a range of the series, thinning so a view stays small to embed.
fn sample(level: &Daily, days: Option<i64>) -> Vec<(i64, i128)> {
    let end = level.end();
    let start = match days {
        Some(d) => (end - d + 1).max(level.start),
        None => level.start,
    };
    let span = end - start + 1;
    let stride = match span {
        0..=45 => 1,
        46..=200 => 2,
        201..=800 => 4,
        _ => 7,
    };
    let mut out = Vec::new();
    let mut day = start;
    while day <= end {
        if let Some(v) = level.value_on(day) {
            out.push((day, v));
        }
        day += stride;
    }
    if let Some(v) = level.value_on(end) {
        if out.last().map(|(d, _)| *d) != Some(end) {
            out.push((end, v));
        }
    }
    out
}

/// x-axis labels: dates for short ranges, months for a year, years for the lot.
fn x_labels(pts: &[(i64, i128)], days: Option<i64>) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    match days {
        Some(d) if d <= 30 => {
            let step = (pts.len() / 4).max(1);
            for (i, (day, _)) in pts.iter().enumerate() {
                if i % step == 0 {
                    let (_, m, dd) = civil_from_days(*day);
                    out.push((i, format!("{m:02}-{dd:02}")));
                }
            }
        }
        Some(_) => {
            let mut last = i64::MIN;
            for (i, (day, _)) in pts.iter().enumerate() {
                let (y, m, _) = civil_from_days(*day);
                let key = y * 12 + m;
                if key != last && (m % 2 == 1) {
                    last = key;
                    out.push((i, format!("{y}-{m:02}")));
                }
            }
        }
        None => {
            let mut last = i64::MIN;
            for (i, (day, _)) in pts.iter().enumerate() {
                let (y, m, _) = civil_from_days(*day);
                if y != last && m <= 2 {
                    last = y;
                    out.push((i, y.to_string()));
                }
            }
        }
    }
    out
}

fn render_view(level: &Daily, key: &'static str, label: &'static str, days: Option<i64>) -> View {
    let pts = sample(level, days);
    let n = pts.len().max(2) as i64;

    let min_v = pts.iter().map(|(_, v)| *v).min().unwrap_or(SCALE).max(1);
    let max_v = pts.iter().map(|(_, v)| *v).max().unwrap_or(SCALE).max(min_v);
    let logarithmic = max_v / min_v.max(1) >= LOG_ABOVE;

    let ticks = if logarithmic {
        log_ticks(min_v, max_v)
    } else {
        linear_ticks(min_v, max_v)
    };
    let y_lo = ticks.iter().min().copied().unwrap_or(min_v).min(min_v);
    let y_hi = ticks.iter().max().copied().unwrap_or(max_v).max(max_v);

    let plot_w = W - PAD_L - PAD_R;
    let plot_h = H - PAD_T - PAD_B;

    let x_of = |i: i64| -> i64 { PAD_L + i * plot_w / (n - 1).max(1) };
    let y_of = |v: i128| -> i64 {
        let t = if logarithmic {
            log_pos(v, y_lo, y_hi)
        } else {
            let span = (y_hi - y_lo).max(1);
            (v - y_lo) as f64 / span as f64
        };
        PAD_T + plot_h - (t * plot_h as f64).round() as i64
    };

    // a log ladder lands on whole numbers; a linear step decides its own places
    let places = if logarithmic {
        0
    } else {
        let step = ticks.windows(2).map(|w| w[1] - w[0]).min().unwrap_or(SCALE);
        match step {
            s if s >= SCALE => 0,
            s if s >= SCALE / 10 => 1,
            _ => 2,
        }
    };

    let mut svg = String::new();
    for tick in &ticks {
        let y = y_of(*tick);
        if y < PAD_T - 2 || y > PAD_T + plot_h + 2 {
            continue;
        }
        svg.push_str(&format!(
            "<line class=\"grid\" x1=\"{PAD_L}\" y1=\"{y}\" x2=\"{}\" y2=\"{y}\"/>",
            PAD_L + plot_w
        ));
        svg.push_str(&format!(
            "<text class=\"tick\" x=\"{}\" y=\"{}\">{}</text>",
            PAD_L + plot_w + 8,
            y + 4,
            format_thousands(*tick, places)
        ));
    }

    for (i, text) in x_labels(&pts, days) {
        svg.push_str(&format!(
            "<text class=\"tick\" x=\"{}\" y=\"{}\" text-anchor=\"middle\">{text}</text>",
            x_of(i as i64),
            H - 8
        ));
    }

    let mut path = String::with_capacity(pts.len() * 12);
    for (i, (_, v)) in pts.iter().enumerate() {
        path.push_str(if i == 0 { "M" } else { "L" });
        path.push_str(&format!("{} {}", x_of(i as i64), y_of(*v)));
        if i + 1 < pts.len() {
            path.push(' ');
        }
    }
    svg.push_str(&format!("<path class=\"line\" d=\"{path}\"/>"));

    if let Some((_, last)) = pts.last() {
        svg.push_str(&format!(
            "<circle class=\"end\" cx=\"{}\" cy=\"{}\" r=\"5\"/>",
            x_of(n - 1),
            y_of(*last)
        ));
    }

    svg.push_str(&format!(
        "<g class=\"hover\" style=\"display:none\"><line class=\"cross\" y1=\"{PAD_T}\" y2=\"{}\"/>\
         <circle class=\"dot\" r=\"5\"/></g>",
        PAD_T + plot_h
    ));

    let mut points_json = String::from("[");
    for (i, (day, v)) in pts.iter().enumerate() {
        if i > 0 {
            points_json.push(',');
        }
        points_json.push_str(&format!(
            "[{},{},\"{}\",\"{}\"]",
            x_of(i as i64),
            y_of(*v),
            format_fixed(*v, 2),
            format_date(*day)
        ));
    }
    points_json.push(']');

    View {
        key,
        label,
        svg,
        points_json,
        logarithmic,
    }
}

/// every range the page offers, in tab order.
pub fn render_views(level: &Daily) -> Vec<View> {
    RANGES
        .iter()
        .map(|(key, label, days)| render_view(level, key, label, *days))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::num::parse_fixed;

    fn series(len: usize, f: impl Fn(usize) -> i128) -> Daily {
        Daily {
            start: 0,
            values: (0..len).map(f).collect(),
        }
    }

    #[test]
    fn every_range_renders_a_path() {
        let level = series(4000, |i| parse_fixed("100").expect("n") + i as i128 * SCALE);
        let views = render_views(&level);
        assert_eq!(views.len(), 4);
        for v in &views {
            assert!(v.svg.contains("<path class=\"line\""), "{}", v.key);
            assert!(v.points_json.starts_with("[["), "{}", v.key);
            assert!(v.svg.contains("class=\"hover\""), "{}", v.key);
        }
        assert_eq!(views.iter().map(|v| v.label).collect::<Vec<_>>(), ["7D", "1M", "1Y", "ALL"]);
    }

    #[test]
    fn a_short_flat_range_stays_linear_and_the_decade_goes_log() {
        // a week of a slow-moving average barely moves: linear
        let flat = series(400, |i| parse_fixed("100").expect("n") + (i % 3) as i128 * SCALE / 100);
        let views = render_views(&flat);
        assert!(!views[0].logarithmic, "7d should be linear");

        // a 60x climb over the whole series: log
        let steep = series(4000, |i| parse_fixed("100").expect("n") * (1 + i as i128 / 60));
        let views = render_views(&steep);
        assert!(views[3].logarithmic, "all should be logarithmic");
    }

    #[test]
    fn short_ranges_sample_every_day() {
        let level = series(400, |i| parse_fixed("100").expect("n") + i as i128 * SCALE);
        let week = sample(&level, Some(7));
        assert_eq!(week.len(), 7);
        let month = sample(&level, Some(30));
        assert_eq!(month.len(), 30);
    }

    #[test]
    fn a_series_shorter_than_the_range_is_not_padded() {
        let level = series(10, |i| parse_fixed("100").expect("n") + i as i128 * SCALE);
        let year = sample(&level, Some(365));
        assert_eq!(year.len(), 10);
    }

    #[test]
    fn linear_ticks_bracket_the_data() {
        let ticks = linear_ticks(parse_fixed("6900").expect("n"), parse_fixed("6910").expect("n"));
        assert!(ticks.first().copied().unwrap_or(0) <= parse_fixed("6900").expect("n"));
        assert!(ticks.last().copied().unwrap_or(0) >= parse_fixed("6910").expect("n"));
        assert!(ticks.len() >= 2 && ticks.len() <= 8, "got {} ticks", ticks.len());
    }
}

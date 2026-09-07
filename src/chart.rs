//! the index chart as static SVG — integer geometry, no runtime dependency.
//!
//! one series, so no legend: the heading names what is plotted. the endpoint
//! carries a direct label; the axis ticks carry the rest.

use crate::num::{civil_from_days, format_date, format_thousands, SCALE};
use crate::series::Daily;

/// plot area in viewBox units. the SVG scales to its container from there.
const W: i64 = 1000;
const H: i64 = 320;
const PAD_L: i64 = 8;
const PAD_R: i64 = 66;
const PAD_T: i64 = 16;
const PAD_B: i64 = 30;

pub struct Chart {
    pub svg: String,
    /// `[[dayIndex, level], …]` for the hover layer, matching the plotted points.
    pub points_json: String,
    pub first_day: i64,
    pub step_days: i64,
}

/// the 1-2-5 ladder, one rung per readable gridline on a log axis.
fn log_ticks(min_v: i128, max_v: i128) -> Vec<i128> {
    let mut ticks = Vec::new();
    let mut decade = 1i128;
    // start a decade below the minimum so the floor tick sits under the series
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
    if ticks.is_empty() {
        ticks.push(min_v.max(SCALE));
    }
    ticks
}

/// position on a log axis, in thousandths of the plot height.
///
/// the index path is integer fixed-point end to end; this is chart geometry,
/// rounded to whole viewBox units, so the rendered SVG is stable regardless.
fn log_pos(v: i128, lo: i128, hi: i128) -> f64 {
    let f = |x: i128| (x as f64 / SCALE as f64).max(1e-9).ln();
    let (lo_l, hi_l, v_l) = (f(lo), f(hi), f(v));
    if (hi_l - lo_l).abs() < f64::EPSILON {
        return 0.0;
    }
    ((v_l - lo_l) / (hi_l - lo_l)).clamp(0.0, 1.0)
}

/// sample every `step` days so the plotted series stays small enough to embed.
fn sample(level: &Daily, step: i64) -> Vec<(i64, i128)> {
    let mut out = Vec::new();
    let mut day = level.start;
    while day <= level.end() {
        if let Some(v) = level.value_on(day) {
            out.push((day, v));
        }
        day += step;
    }
    // always finish on the latest fix, whatever the stride lands on
    if let Some(last) = level.values.last() {
        let end = level.end();
        if out.last().map(|(d, _)| *d) != Some(end) {
            out.push((end, *last));
        }
    }
    out
}

pub fn render(level: &Daily, step_days: i64) -> Chart {
    let pts = sample(level, step_days);
    let n = pts.len().max(2) as i64;

    let min_v = pts.iter().map(|(_, v)| *v).min().unwrap_or(SCALE).max(SCALE);
    let max_v = pts.iter().map(|(_, v)| *v).max().unwrap_or(SCALE).max(min_v);
    let ticks = log_ticks(min_v, max_v);
    // the axis spans the outermost ticks, so the series sits inside the grid
    let y_lo = ticks.iter().min().copied().unwrap_or(min_v).min(min_v);
    let y_hi = ticks.iter().max().copied().unwrap_or(max_v).max(max_v);

    let plot_w = W - PAD_L - PAD_R;
    let plot_h = H - PAD_T - PAD_B;

    let x_of = |i: i64| -> i64 { PAD_L + i * plot_w / (n - 1).max(1) };
    let y_of = |v: i128| -> i64 {
        let t = log_pos(v, y_lo, y_hi);
        PAD_T + plot_h - (t * plot_h as f64).round() as i64
    };

    let mut svg = String::new();
    svg.push_str(&format!(
        "<svg class=\"chart\" viewBox=\"0 0 {W} {H}\" role=\"img\" \
         aria-label=\"CX index level, {} to {}\" preserveAspectRatio=\"none\">",
        format_date(level.start),
        format_date(level.end())
    ));

    // gridlines and y ticks
    for tick in &ticks {
        let y = y_of(*tick);
        svg.push_str(&format!(
            "<line class=\"grid\" x1=\"{PAD_L}\" y1=\"{y}\" x2=\"{}\" y2=\"{y}\"/>",
            PAD_L + plot_w
        ));
        svg.push_str(&format!(
            "<text class=\"tick\" x=\"{}\" y=\"{}\">{}</text>",
            PAD_L + plot_w + 8,
            y + 4,
            format_thousands(*tick, 0)
        ));
    }

    // year boundaries on the x axis
    let mut last_year = i64::MIN;
    for (i, (day, _)) in pts.iter().enumerate() {
        let (y, m, _) = civil_from_days(*day);
        if y != last_year && m <= 2 {
            last_year = y;
            let x = x_of(i as i64);
            svg.push_str(&format!(
                "<text class=\"tick\" x=\"{x}\" y=\"{}\" text-anchor=\"middle\">{y}</text>",
                H - 8
            ));
        }
    }

    // the series itself
    let mut path = String::with_capacity(pts.len() * 12);
    for (i, (_, v)) in pts.iter().enumerate() {
        path.push_str(if i == 0 { "M" } else { "L" });
        path.push_str(&format!("{} {}", x_of(i as i64), y_of(*v)));
        if i + 1 < pts.len() {
            path.push(' ');
        }
    }
    svg.push_str(&format!("<path class=\"line\" d=\"{path}\"/>"));

    // endpoint marker with a surface ring, and its direct label
    if let Some((_, last)) = pts.last() {
        let x = x_of(n - 1);
        let y = y_of(*last);
        svg.push_str(&format!("<circle class=\"end\" cx=\"{x}\" cy=\"{y}\" r=\"5\"/>"));
    }

    svg.push_str("<g class=\"hover\" style=\"display:none\">");
    svg.push_str("<line class=\"cross\" y1=\"0\" y2=\"");
    svg.push_str(&format!("{}\"/>", PAD_T + plot_h));
    svg.push_str("<circle class=\"dot\" r=\"5\"/></g>");
    svg.push_str("</svg>");

    let mut points_json = String::from("[");
    for (i, (_, v)) in pts.iter().enumerate() {
        if i > 0 {
            points_json.push(',');
        }
        points_json.push_str(&format!(
            "[{},{},{}]",
            x_of(i as i64),
            y_of(*v),
            crate::num::format_fixed(*v, 2)
        ));
    }
    points_json.push(']');

    Chart {
        svg,
        points_json,
        first_day: level.start,
        step_days,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::num::parse_fixed;

    fn ramp(len: usize) -> Daily {
        Daily {
            start: 0,
            values: (0..len)
                .map(|i| parse_fixed("100").expect("n") + i as i128 * SCALE)
                .collect(),
        }
    }

    #[test]
    fn renders_a_path_and_matching_points() {
        let chart = render(&ramp(400), 7);
        assert!(chart.svg.starts_with("<svg"));
        assert!(chart.svg.contains("<path class=\"line\""));
        assert!(chart.points_json.starts_with("[["));
        // the sampled series always closes on the final fix
        assert!(chart.svg.contains("circle class=\"end\""));
    }

    #[test]
    fn log_ticks_ladder_covers_the_range() {
        let ticks = log_ticks(100 * SCALE, 7000 * SCALE);
        assert!(ticks.first().copied().unwrap_or(0) <= 100 * SCALE);
        assert!(ticks.last().copied().unwrap_or(0) >= 5000 * SCALE);
        // the ladder stays on 1-2-5 rungs
        for t in ticks {
            let mantissa = {
                let mut m = t / SCALE;
                while m % 10 == 0 && m > 9 {
                    m /= 10;
                }
                m
            };
            assert!(matches!(mantissa, 1 | 2 | 5), "tick {t} off the ladder");
        }
    }

    #[test]
    fn log_position_is_monotonic_and_bounded() {
        let (lo, hi) = (100 * SCALE, 10_000 * SCALE);
        assert!((log_pos(lo, lo, hi) - 0.0).abs() < 1e-9);
        assert!((log_pos(hi, lo, hi) - 1.0).abs() < 1e-9);
        // a decade up from the floor sits halfway across two decades
        assert!((log_pos(1000 * SCALE, lo, hi) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn handles_a_flat_series_without_dividing_by_zero() {
        let flat = Daily {
            start: 0,
            values: vec![parse_fixed("100").expect("n"); 10],
        };
        let chart = render(&flat, 7);
        assert!(chart.svg.contains("<path"));
    }
}

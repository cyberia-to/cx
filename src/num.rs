//! fixed-point decimal and civil-date arithmetic — integers only.
//!
//! every price, weight and index level in this crate is an `i128` scaled by
//! `SCALE` (1e-9). no floating point enters the index path, so the same input
//! yields the same output on any machine.

/// nine decimal places. uranium trades near $70/lb, bitcoin near $100k —
/// both fit with room to spare, and `i128` leaves headroom for the sums.
pub const SCALE: i128 = 1_000_000_000;

/// parse a decimal string ("13542.820869565219", "-0.5", "8") into fixed point.
/// digits past the ninth are truncated, never rounded — truncation is
/// reproducible, rounding invites a tie-breaking rule nobody agrees on.
pub fn parse_fixed(s: &str) -> Option<i128> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let (neg, body) = match s.as_bytes()[0] {
        b'-' => (true, &s[1..]),
        b'+' => (false, &s[1..]),
        _ => (false, s),
    };
    if body.is_empty() {
        return None;
    }
    let mut int_part: i128 = 0;
    let mut frac_part: i128 = 0;
    let mut frac_digits = 0u32;
    let mut seen_dot = false;
    let mut seen_digit = false;
    for b in body.bytes() {
        match b {
            b'0'..=b'9' => {
                seen_digit = true;
                let d = (b - b'0') as i128;
                if seen_dot {
                    if frac_digits < 9 {
                        frac_part = frac_part * 10 + d;
                        frac_digits += 1;
                    }
                } else {
                    int_part = int_part.checked_mul(10)?.checked_add(d)?;
                }
            }
            b'.' if !seen_dot => seen_dot = true,
            // exponent notation never appears in these feeds; reject it loudly
            // rather than silently mis-parsing.
            _ => return None,
        }
    }
    if !seen_digit {
        return None;
    }
    while frac_digits < 9 {
        frac_part *= 10;
        frac_digits += 1;
    }
    let v = int_part.checked_mul(SCALE)?.checked_add(frac_part)?;
    Some(if neg { -v } else { v })
}

/// multiply two fixed-point values.
pub fn fmul(a: i128, b: i128) -> i128 {
    a * b / SCALE
}

/// divide two fixed-point values; `None` when the divisor is zero.
pub fn fdiv(a: i128, b: i128) -> Option<i128> {
    if b == 0 {
        return None;
    }
    Some(a * SCALE / b)
}

/// render fixed point with `places` decimals (truncating, as parsed).
pub fn format_fixed(v: i128, places: u32) -> String {
    let neg = v < 0;
    let v = v.abs();
    let int_part = v / SCALE;
    let frac_part = v % SCALE;
    let mut out = String::new();
    if neg {
        out.push('-');
    }
    out.push_str(&int_part.to_string());
    if places > 0 {
        let mut frac = frac_part;
        // drop the digits below the requested precision
        for _ in 0..(9 - places.min(9)) {
            frac /= 10;
        }
        out.push('.');
        let s = frac.to_string();
        let width = places.min(9) as usize;
        for _ in 0..width.saturating_sub(s.len()) {
            out.push('0');
        }
        out.push_str(&s);
    }
    out
}

/// thousands-separated integer part, for display.
pub fn format_thousands(v: i128, places: u32) -> String {
    let raw = format_fixed(v, places);
    let (int_str, rest) = match raw.split_once('.') {
        Some((i, f)) => (i.to_string(), format!(".{f}")),
        None => (raw.clone(), String::new()),
    };
    let (sign, digits) = match int_str.strip_prefix('-') {
        Some(d) => ("-", d.to_string()),
        None => ("", int_str),
    };
    let mut grouped = String::new();
    let bytes = digits.as_bytes();
    for (i, b) in bytes.iter().enumerate() {
        if i > 0 && (bytes.len() - i) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(*b as char);
    }
    format!("{sign}{grouped}{rest}")
}

// ---------------------------------------------------------------------------
// civil dates as days since 1970-01-01, after Howard Hinnant's algorithms.
// integer-only, valid across the whole range this crate touches.
// ---------------------------------------------------------------------------

/// days since the unix epoch for a proleptic-gregorian y-m-d.
pub fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// inverse of [`days_from_civil`].
pub fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// parse `YYYY-MM-DD` into days since epoch.
pub fn parse_date(s: &str) -> Option<i64> {
    let b = s.as_bytes();
    if b.len() < 10 || b[4] != b'-' || b[7] != b'-' {
        return None;
    }
    let num = |from: usize, to: usize| -> Option<i64> { s[from..to].parse::<i64>().ok() };
    let y = num(0, 4)?;
    let m = num(5, 7)?;
    let d = num(8, 10)?;
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    Some(days_from_civil(y, m, d))
}

/// render days-since-epoch as `YYYY-MM-DD`.
pub fn format_date(days: i64) -> String {
    let (y, m, d) = civil_from_days(days);
    format!("{y:04}-{m:02}-{d:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_decimals_to_fixed_point() {
        assert_eq!(parse_fixed("1"), Some(SCALE));
        assert_eq!(parse_fixed("0.5"), Some(SCALE / 2));
        assert_eq!(parse_fixed("-2.25"), Some(-(SCALE * 9 / 4)));
        assert_eq!(parse_fixed("13542.820869565219"), Some(13_542_820_869_565));
        assert_eq!(parse_fixed(""), None);
        assert_eq!(parse_fixed("1e5"), None);
    }

    #[test]
    fn formats_round_trip() {
        let v = parse_fixed("62626.5").expect("parses");
        assert_eq!(format_fixed(v, 2), "62626.50");
        assert_eq!(format_thousands(v, 2), "62,626.50");
        assert_eq!(format_fixed(parse_fixed("100").expect("parses"), 0), "100");
    }

    #[test]
    fn multiplies_and_divides_in_fixed_point() {
        let two = parse_fixed("2").expect("parses");
        let three = parse_fixed("3").expect("parses");
        assert_eq!(fmul(two, three), parse_fixed("6").expect("parses"));
        assert_eq!(fdiv(three, two), parse_fixed("1.5"));
        assert_eq!(fdiv(three, 0), None);
    }

    #[test]
    fn dates_round_trip_across_leap_years() {
        for iso in ["1970-01-01", "2016-02-29", "2026-09-07", "2000-03-01"] {
            let days = parse_date(iso).expect("parses");
            assert_eq!(format_date(days), iso);
        }
        assert_eq!(parse_date("2026-13-01"), None);
        assert_eq!(
            parse_date("2026-09-08").expect("parses") - parse_date("2026-09-07").expect("parses"),
            1
        );
    }
}

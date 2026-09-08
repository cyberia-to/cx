//! century index — build the daily fix and everything published from it.
//!
//! the pipeline: pull public fixes → carry them onto a dense calendar →
//! trailing 365-day average → quantities fixed at the base date → index level.
//! the arithmetic is integer fixed-point throughout, so the same day's data
//! yields the same index on any machine.

mod calculator;
mod chart;
mod fetch;
mod graph;
mod index;
mod num;
mod series;
mod site;

use std::path::PathBuf;
use std::process::ExitCode;

use fetch::Fetcher;
use num::{days_from_civil, format_date, parse_date, civil_from_days, SCALE};
use series::{all_positive, max_daily_ratio, Daily, Series};

/// how far back the published history reaches.
const HISTORY_YEARS: i64 = 10;

/// a glitch guard: no leg in this basket moves 5x in a day. copper and uranium
/// are monthly, gold and oil settle daily — none of them gap like that, so a
/// jump this size is a bad parse or a bad feed, and the run should stop.
const MAX_DAILY_JUMP: i128 = 5 * SCALE;

/// the monthly legs print once a cycle; past this, a quiet source is news.
const STALE_AFTER_DAYS: i64 = 45;

/// the shortest average the index will publish. a leg younger than the full
/// window averages over what it has — below a quarter of fixes it is noise.
const MIN_TWAP_DAYS: usize = 90;

struct Args {
    out_dir: PathBuf,
    cache_dir: PathBuf,
    today: Option<String>,
    offline: bool,
}

fn parse_args() -> Args {
    let mut args = Args {
        out_dir: PathBuf::from("out"),
        cache_dir: PathBuf::from("out/cache"),
        today: None,
        offline: false,
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--out" => {
                if let Some(v) = it.next() {
                    args.out_dir = PathBuf::from(v);
                }
            }
            "--cache" => {
                if let Some(v) = it.next() {
                    args.cache_dir = PathBuf::from(v);
                }
            }
            "--today" => args.today = it.next(),
            "--offline" => args.offline = true,
            _ => {}
        }
    }
    args
}

/// today in UTC, from the system clock.
fn today_utc() -> i64 {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    secs.div_euclid(86_400)
}

/// fetch one Coinbase product across a date range, 290 days at a time —
/// the endpoint caps a response at 300 candles.
fn coinbase_series(f: &Fetcher, product: &str, from: i64, to: i64) -> Result<Series, String> {
    let mut all = Series::new();
    let mut cursor = from;
    let mut slice = 0;
    while cursor <= to {
        let end = (cursor + 289).min(to);
        let url = fetch::coinbase_url(product, &format_date(cursor), &format_date(end));
        let key = format!("coinbase-{product}-{slice}-{}", format_date(cursor));
        let body = f.get(&key, &url).map_err(|e| e.to_string())?;
        let part = fetch::parse_coinbase_candles(&body).map_err(|e| e.to_string())?;
        for i in 0..part.len() {
            if let Some(point) = part.point(i) {
                all.push(point.0, point.1);
            }
        }
        cursor = end + 1;
        slice += 1;
    }
    all.normalise();
    if all.is_empty() {
        return Err(format!("{product}: no candles across the range"));
    }
    Ok(all)
}

/// pull every leg's raw observations.
fn gather(f: &Fetcher, raw_from: i64, today: i64) -> Result<Vec<(&'static str, Series)>, String> {
    let mut out: Vec<(&'static str, Series)> = Vec::new();

    out.push(("BTC", coinbase_series(f, "BTC-USD", raw_from, today)?));
    out.push(("ETH", coinbase_series(f, "ETH-USD", raw_from, today)?));

    let cny_url = fetch::frankfurter_url(&format_date(raw_from), &format_date(today));
    let cny_body = f.get("frankfurter-cny", &cny_url).map_err(|e| e.to_string())?;
    out.push((
        "CNY",
        fetch::parse_frankfurter_cny(&cny_body).map_err(|e| e.to_string())?,
    ));

    // the quote currency: one dollar is one dollar, every day.
    let mut usd = Series::new();
    usd.push(raw_from, SCALE);
    out.push(("USD", usd));

    let gold_body = f
        .get("lbma-gold-pm", fetch::LBMA_GOLD_URL)
        .map_err(|e| e.to_string())?;
    out.push((
        "GOLD",
        fetch::parse_lbma(&gold_body).map_err(|e| e.to_string())?,
    ));

    for (ticker, id) in [
        ("CU", "PCOPPUSDM"),
        ("OIL", "DCOILBRENTEU"),
        ("UX", "PURANUSDM"),
    ] {
        let body = f
            .get(&format!("fred-{id}"), &fetch::fred_url(id))
            .map_err(|e| e.to_string())?;
        out.push((
            ticker,
            fetch::parse_fred_csv(&body).map_err(|e| e.to_string())?,
        ));
    }

    Ok(out)
}

fn run() -> Result<(), String> {
    let args = parse_args();

    let today = match &args.today {
        Some(s) => parse_date(s).ok_or_else(|| format!("bad --today {s}"))?,
        None => today_utc(),
    };
    let (ty, tm, td) = civil_from_days(today);
    let base_day = days_from_civil(ty - HISTORY_YEARS, tm, td);
    // the base date needs a full trailing window behind it
    let raw_from = base_day - index::TWAP_WINDOW as i64;

    eprintln!(
        "cx: base {} · today {} · raw from {}",
        format_date(base_day),
        format_date(today),
        format_date(raw_from)
    );

    let f = Fetcher::new(&args.cache_dir, args.offline);
    let raw = gather(&f, raw_from, today)?;

    // dense daily calendars, then the trailing 365-day average
    let mut twaps: Vec<(&'static str, Daily)> = Vec::with_capacity(raw.len());
    let mut coverage: Vec<(&'static str, String, usize)> = Vec::new();
    for (ticker, s) in &raw {
        // a leg whose source went quiet is still carried forward, per §3 — but
        // silence past a print cycle is worth saying out loud rather than
        // burying under a fresh-looking number.
        if let Some((last_day, _)) = s.last() {
            let stale = today - last_day;
            if stale > STALE_AFTER_DAYS && *ticker != "USD" {
                eprintln!(
                    "cx: warning — {ticker} last printed {} ({stale} days ago), carried forward",
                    format_date(last_day)
                );
            }
        }
        // a leg that starts later than the raw window begins at its own first
        // print rather than being back-filled with a price nobody quoted.
        let leg_start = s
            .first_day()
            .ok_or_else(|| format!("{ticker}: no observations at all"))?
            .max(raw_from);
        let values = s
            .carry_forward(leg_start, today)
            .ok_or_else(|| format!("{ticker}: no observation at or before {}", format_date(leg_start)))?;
        let dense = Daily {
            start: leg_start,
            values,
        };
        if !all_positive(&dense) {
            return Err(format!("{ticker}: a fix was zero or negative"));
        }
        let jump = max_daily_ratio(&dense);
        if jump > MAX_DAILY_JUMP {
            return Err(format!(
                "{ticker}: a single day moved {}x — refusing the feed",
                num::format_fixed(jump, 2)
            ));
        }
        let twap = dense
            .trailing_mean_min(index::TWAP_WINDOW, MIN_TWAP_DAYS)
            .ok_or_else(|| format!("{ticker}: fewer than {MIN_TWAP_DAYS} days of fixes"))?;
        if twap.start > base_day {
            return Err(format!(
                "{ticker}: its average only begins {}, after the base date {}",
                format_date(twap.start),
                format_date(base_day)
            ));
        }
        coverage.push((ticker, s.span_text(), s.len()));
        twaps.push((ticker, twap));
    }

    let idx = index::build(&twaps, base_day).map_err(|e| e.to_string())?;
    let (last_day, last_level) = idx
        .latest()
        .ok_or_else(|| "index produced no levels".to_string())?;
    eprintln!(
        "cx: {} levels · latest {} = {}",
        idx.level.values.len(),
        format_date(last_day),
        num::format_fixed(last_level, 2)
    );

    std::fs::create_dir_all(&args.out_dir).map_err(|e| e.to_string())?;

    // one view per range: 7d, 1m, 1y, all
    let views = chart::render_views(&idx.level);

    let html = site::render(&idx, &twaps, &views)?;
    let json = site::render_json(&idx, &twaps, &coverage);
    let history = graph::render_history(&idx, &twaps, &coverage)?;
    let readme = graph::render_readme(&idx, &twaps)?;

    let write = |name: &str, body: &str| -> Result<(), String> {
        let path = args.out_dir.join(name);
        std::fs::write(&path, body).map_err(|e| format!("{}: {e}", path.display()))
    };
    write("index.html", &html)?;
    write("cx.json", &json)?;
    write("history.md", &history)?;
    write("README.md", &readme)?;

    eprintln!("cx: wrote index.html, cx.json, history.md, README.md to {}", args.out_dir.display());
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("cx: {e}");
            ExitCode::FAILURE
        }
    }
}

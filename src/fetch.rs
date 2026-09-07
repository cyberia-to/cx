//! the IO shell: pull public fixes, parse the four feed shapes.
//!
//! network access shells out to `curl`, which keeps the crate dependency-free.
//! every response is cached on disk, so a rebuild of the site does not
//! re-hammer the sources and a failed run can be replayed exactly.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::num::{fdiv, parse_date, parse_fixed, SCALE};
use crate::series::Series;

pub struct Fetcher {
    cache_dir: PathBuf,
    offline: bool,
}

#[derive(Debug)]
pub enum FetchError {
    Curl(String),
    Empty(String),
    Cache(String),
    Parse(String),
}

impl std::fmt::Display for FetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FetchError::Curl(m) => write!(f, "curl: {m}"),
            FetchError::Empty(m) => write!(f, "empty response: {m}"),
            FetchError::Cache(m) => write!(f, "cache: {m}"),
            FetchError::Parse(m) => write!(f, "parse: {m}"),
        }
    }
}

impl Fetcher {
    pub fn new(cache_dir: impl AsRef<Path>, offline: bool) -> Self {
        Self {
            cache_dir: cache_dir.as_ref().to_path_buf(),
            offline,
        }
    }

    /// fetch a URL, or return the cached body when one exists. `key` names the
    /// cache entry; distinct URLs must use distinct keys.
    pub fn get(&self, key: &str, url: &str) -> Result<String, FetchError> {
        let path = self.cache_dir.join(format!("{key}.raw"));
        if path.exists() {
            return std::fs::read_to_string(&path)
                .map_err(|e| FetchError::Cache(format!("{}: {e}", path.display())));
        }
        if self.offline {
            return Err(FetchError::Cache(format!("{key} missing and --offline set")));
        }

        std::fs::create_dir_all(&self.cache_dir)
            .map_err(|e| FetchError::Cache(format!("{}: {e}", self.cache_dir.display())))?;

        let out = Command::new("curl")
            .args([
                "-sS",
                "--fail",
                "--max-time",
                "60",
                "--retry",
                "3",
                "--retry-delay",
                "2",
                "-A",
                "cx-index/0.1 (+https://cyberia.my/cx)",
                url,
            ])
            .output()
            .map_err(|e| FetchError::Curl(e.to_string()))?;

        if !out.status.success() {
            let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
            return Err(FetchError::Curl(format!("{url}: {err}")));
        }
        let body = String::from_utf8_lossy(&out.stdout).to_string();
        if body.trim().is_empty() {
            return Err(FetchError::Empty(url.to_string()));
        }
        std::fs::write(&path, &body)
            .map_err(|e| FetchError::Cache(format!("{}: {e}", path.display())))?;
        Ok(body)
    }
}

// ---------------------------------------------------------------------------
// FRED: `observation_date,SERIES` CSV. missing observations are a bare dot.
// ---------------------------------------------------------------------------

pub fn fred_url(series_id: &str) -> String {
    format!("https://fred.stlouisfed.org/graph/fredgraph.csv?id={series_id}")
}

pub fn parse_fred_csv(body: &str) -> Result<Series, FetchError> {
    let mut series = Series::new();
    for line in body.lines().skip(1) {
        let mut parts = line.split(',');
        let (Some(date), Some(value)) = (parts.next(), parts.next()) else {
            continue;
        };
        let value = value.trim();
        if value == "." || value.is_empty() {
            continue;
        }
        let Some(day) = parse_date(date.trim()) else {
            continue;
        };
        let Some(v) = parse_fixed(value) else {
            continue;
        };
        if v > 0 {
            series.push(day, v);
        }
    }
    if series.is_empty() {
        return Err(FetchError::Parse("FRED csv held no observations".into()));
    }
    series.normalise();
    Ok(series)
}

// ---------------------------------------------------------------------------
// LBMA: [{"d":"1968-04-01","v":[37.7,15.68,null]}, …] — v[0] is the USD fix.
// ---------------------------------------------------------------------------

pub const LBMA_GOLD_URL: &str = "https://prices.lbma.org.uk/json/gold_pm.json";

pub fn parse_lbma(body: &str) -> Result<Series, FetchError> {
    let mut series = Series::new();
    for chunk in body.split("\"d\":\"").skip(1) {
        let Some((date, rest)) = chunk.split_once('"') else {
            continue;
        };
        let Some(day) = parse_date(date) else {
            continue;
        };
        let Some(values) = rest.split_once("\"v\":[").map(|(_, v)| v) else {
            continue;
        };
        let Some((first, _)) = values.split_once(|c| c == ',' || c == ']') else {
            continue;
        };
        let Some(v) = parse_fixed(first) else {
            continue; // "null" and friends
        };
        if v > 0 {
            series.push(day, v);
        }
    }
    if series.is_empty() {
        return Err(FetchError::Parse("LBMA json held no fixes".into()));
    }
    series.normalise();
    Ok(series)
}

// ---------------------------------------------------------------------------
// Frankfurter (ECB): {"rates":{"2016-01-04":{"CNY":6.534}, …}}
// the feed quotes CNY per USD; the basket holds yuan, so the leg is its
// reciprocal — the USD price of one yuan.
// ---------------------------------------------------------------------------

pub fn frankfurter_url(from: &str, to: &str) -> String {
    format!("https://api.frankfurter.dev/v1/{from}..{to}?base=USD&symbols=CNY")
}

pub fn parse_frankfurter_cny(body: &str) -> Result<Series, FetchError> {
    let Some((_, rates)) = body.split_once("\"rates\":{") else {
        return Err(FetchError::Parse("frankfurter json had no rates".into()));
    };
    let mut series = Series::new();
    let mut rest = rates;
    loop {
        // each entry reads `"YYYY-MM-DD":{"CNY":<rate>}` — walk quote to quote
        // and keep the tokens that are dates.
        let Some(open) = rest.find('"') else { break };
        let after = &rest[open + 1..];
        let Some(close) = after.find('"') else { break };
        let token = &after[..close];
        let tail = &after[close + 1..];
        rest = tail;

        let Some(day) = parse_date(token) else {
            continue;
        };
        // confine the search to this entry, so a date without a rate cannot
        // borrow the next entry's number.
        let entry = tail.split('}').next().unwrap_or("");
        let Some((_, after_key)) = entry.split_once("CNY\":") else {
            continue;
        };
        let number: String = after_key
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
            .collect();
        let Some(rate) = parse_fixed(&number) else {
            continue;
        };
        // reciprocal: USD per CNY
        let Some(usd_per_cny) = fdiv(SCALE, rate) else {
            continue;
        };
        if usd_per_cny > 0 {
            series.push(day, usd_per_cny);
        }
    }
    if series.is_empty() {
        return Err(FetchError::Parse("frankfurter json held no CNY rates".into()));
    }
    series.normalise();
    Ok(series)
}

// ---------------------------------------------------------------------------
// Coinbase Exchange: [[time, low, high, open, close, volume], …], newest
// first, at most 300 candles per request — so the range is walked in slices.
// ---------------------------------------------------------------------------

pub fn coinbase_url(product: &str, start: &str, end: &str) -> String {
    format!(
        "https://api.exchange.coinbase.com/products/{product}/candles\
         ?granularity=86400&start={start}T00:00:00Z&end={end}T00:00:00Z"
    )
}

pub fn parse_coinbase_candles(body: &str) -> Result<Series, FetchError> {
    let mut series = Series::new();
    let trimmed = body.trim();
    let inner = trimmed
        .strip_prefix('[')
        .and_then(|s| s.strip_suffix(']'))
        .unwrap_or(trimmed);
    for candle in inner.split('[').skip(1) {
        let Some((fields, _)) = candle.split_once(']') else {
            continue;
        };
        let parts: Vec<&str> = fields.split(',').collect();
        if parts.len() < 5 {
            continue;
        }
        let Ok(ts) = parts[0].trim().parse::<i64>() else {
            continue;
        };
        let Some(close) = parse_fixed(parts[4].trim()) else {
            continue;
        };
        if close > 0 {
            // candle timestamps are UTC midnight; integer division lands on
            // the day the candle covers.
            series.push(ts.div_euclid(86_400), close);
        }
    }
    if series.is_empty() {
        return Err(FetchError::Parse("coinbase returned no candles".into()));
    }
    series.normalise();
    Ok(series)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::num::format_date;

    #[test]
    fn parses_fred_csv_and_skips_missing_days() {
        let csv = "observation_date,DCOILBRENTEU\n2016-01-04,36.34\n2016-01-05,.\n2016-01-06,34.19\n";
        let series = parse_fred_csv(csv).expect("parses");
        assert_eq!(series.len(), 2);
        let (day, value) = series.last().expect("has a last point");
        assert_eq!(format_date(day), "2016-01-06");
        assert_eq!(value, parse_fixed("34.19").expect("number"));
    }

    #[test]
    fn parses_lbma_and_skips_null_fixes() {
        let json = r#"[{"d":"2016-01-04","v":[1073.5,730.1,null]},{"d":"2016-01-05","v":[null,null,null]}]"#;
        let series = parse_lbma(json).expect("parses");
        assert_eq!(series.len(), 1);
        assert_eq!(
            series.last().expect("point").1,
            parse_fixed("1073.5").expect("number")
        );
    }

    #[test]
    fn parses_frankfurter_as_usd_per_yuan() {
        let json = r#"{"amount":1.0,"base":"USD","rates":{"2016-01-04":{"CNY":6.534},"2016-01-05":{"CNY":6.5209}}}"#;
        let series = parse_frankfurter_cny(json).expect("parses");
        assert_eq!(series.len(), 2);
        let (_, first) = series.last().expect("point");
        // 1 / 6.5209 ≈ 0.153353
        assert!(first > parse_fixed("0.15").expect("n"));
        assert!(first < parse_fixed("0.16").expect("n"));
    }

    #[test]
    fn parses_coinbase_candles_by_close() {
        // 1454284800 = 2016-02-01
        let json = "[[1454284800,366.26,379,367.89,371.33,7931.2],[1454198400,365,382.5,378.46,367.95,5506.7]]";
        let series = parse_coinbase_candles(json).expect("parses");
        assert_eq!(series.len(), 2);
        let (day, close) = series.last().expect("point");
        assert_eq!(format_date(day), "2016-02-01");
        assert_eq!(close, parse_fixed("371.33").expect("number"));
    }

    #[test]
    fn rejects_bodies_with_no_observations() {
        assert!(parse_fred_csv("observation_date,X\n").is_err());
        assert!(parse_lbma("[]").is_err());
        assert!(parse_coinbase_candles("[]").is_err());
    }
}

//! the page: everything the shell is given, assembled from the index.

use super::shell;
use crate::chart::View;
use crate::index::{Group, Index};
use crate::num::{format_date, format_fixed, format_thousands, SCALE};
use crate::series::Daily;

/// hosts we run ourselves; a load from one of these is ours, not a third party's.
const OWN_HOSTS: [&str; 2] = ["cyberia.my", "cyberia.to"];

fn is_own_host(url: &str) -> bool {
    let rest = url
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_start_matches("//");
    let host = rest.split(['/', ':', '?']).next().unwrap_or("");
    OWN_HOSTS
        .iter()
        .any(|own| host == *own || host.ends_with(&format!(".{own}")))
}

/// everything the page makes the browser fetch from someone else's host.
/// links a reader may click are theirs to follow; a stylesheet, script, font or
/// image is loaded for them, and a third-party one stalls the page on a server
/// we do not run — an external stylesheet also holds every inline script.
pub fn third_party_loads(html: &str) -> Vec<String> {
    let mut found = Vec::new();
    for tag in html.split('<').skip(1) {
        let tag = tag.split('>').next().unwrap_or("");
        let name = tag.split_whitespace().next().unwrap_or("").to_ascii_lowercase();
        let attr = match name.as_str() {
            "link" => "href=\"",
            "script" | "img" | "iframe" | "source" => "src=\"",
            _ => continue,
        };
        if let Some((_, rest)) = tag.split_once(attr) {
            let url = rest.split('"').next().unwrap_or("");
            let absolute =
                url.starts_with("http://") || url.starts_with("https://") || url.starts_with("//");
            if absolute && !is_own_host(url) {
                found.push(url.to_string());
            }
        }
    }
    found
}

fn group_class(g: Group) -> &'static str {
    match g {
        Group::Crypto => "crypto",
        Group::Fiat => "fiat",
        Group::Elements => "elements",
    }
}

pub fn render(
    idx: &Index,
    twaps: &[(&str, Daily)],
    views: &[View],
    indexation_default: &str,
) -> Result<String, String> {
    let (day, level) = idx.latest().ok_or("no index level")?;
    // the page opens on the whole history: a century-scale unit is read at the
    // pace of years, and a week of a 365-day average is a flat line
    let default = views
        .iter()
        .position(|v| v.key == "all")
        .ok_or("no 'all' chart view")?;
    let first = &views[default];

    let tabs_html: String = views
        .iter()
        .enumerate()
        .map(|(i, v)| {
            format!(
                "<button class=\"tab{}\" data-range=\"{}\" role=\"tab\">{}</button>",
                if i == default { " on" } else { "" },
                v.key,
                v.label
            )
        })
        .collect();

    let readout = format!(
        "dollars per CX · fix of {} · hover the line for any day",
        format_date(day)
    );

    let views_json: String = {
        let mut out = String::from("{");
        for (i, v) in views.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push_str(&format!(
                "\"{}\":{{\"svg\":\"{}\",\"labels\":\"{}\",\"pts\":{},\"log\":{}}}",
                v.key,
                v.svg.replace('\\', "\\\\").replace('"', "\\\""),
                v.labels.replace('\\', "\\\\").replace('"', "\\\""),
                v.points_json,
                v.logarithmic
            ));
        }
        out.push('}');
        out
    };
    let holdings = idx.holdings(twaps);

    let mut legs_html = String::new();
    for h in holdings.iter() {
        let fix = twaps
            .iter()
            .find(|(t, _)| *t == h.leg.ticker)
            .and_then(|(_, d)| d.value_on(day))
            .unwrap_or(0);
        let of_peak = if h.alive {
            format!("{}%", format_fixed(h.of_peak_bp * SCALE / 100, 0))
        } else {
            "left the basket".to_string()
        };
        legs_html.push_str(&format!(
            "<tr><td><span class=\"d {cls}\"></span>{ticker}</td>\
             <td class=\"n\">{fix}</td><td class=\"n\">{qty}</td>\
             <td class=\"n\">{weight}%</td><td class=\"n\">{of_peak}</td></tr>",
            cls = group_class(h.leg.group),
            ticker = h.leg.ticker,
            fix = format_thousands(fix, h.leg.fix_places()),
            qty = format_thousands(h.quantity, h.leg.quantity_places(h.quantity)),
            weight = format_fixed(h.weight_bp * SCALE / 100, 0),
        ));
    }

    // the bar shows the basket as it is held today — the weights, which the
    // daily rebalance keeps exactly; a dead leg has no segment.
    let mut bar_html = String::new();
    for h in holdings.iter().filter(|h| h.alive) {
        let pct = format_fixed(h.weight_bp * SCALE / 100, 0);
        bar_html.push_str(&format!(
            "<div class=\"seg {}\" style=\"flex:{}\" title=\"{} — {}%\"><i>{}</i></div>",
            group_class(h.leg.group),
            h.weight_bp,
            h.leg.ticker,
            pct,
            pct
        ));
    }

    let mut group_totals = [(Group::Crypto, 0i128), (Group::Fiat, 0), (Group::Elements, 0)];
    for h in holdings.iter() {
        for slot in group_totals.iter_mut() {
            if slot.0 == h.leg.group {
                slot.1 += h.weight_bp;
            }
        }
    }
    let deaths_html = if idx.deaths.is_empty() {
        "No leg has left the basket.".to_string()
    } else {
        idx.deaths
            .iter()
            .map(|d| {
                format!(
                    "{} left the basket on {} — its average closed at {} against a peak of {}.",
                    d.leg.ticker,
                    format_date(d.day),
                    format_thousands(d.fix, d.leg.fix_places()),
                    format_thousands(d.peak, d.leg.fix_places())
                )
            })
            .collect::<Vec<_>>()
            .join(" ")
    };
    let chips_html: String = group_totals
        .iter()
        .map(|(g, share)| {
            format!(
                "<span><span class=\"d {}\"></span>{} <b>{}%</b></span>",
                group_class(*g),
                g.label(),
                format_fixed(*share * SCALE / 100, 1)
            )
        })
        .collect();

    Ok(shell::page(&shell::Parts {
        level: format_thousands(level, 2),
        base_date: format_date(idx.base_day),
        tabs: tabs_html,
        readout,
        chart_svg: &first.svg,
        chart_labels: &first.labels,
        bar: bar_html,
        chips: chips_html,
        legs: legs_html,
        deaths: deaths_html,
        views: views_json,
        calc: crate::calculator::html(indexation_default, &format_date(idx.base_day)),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rendered() -> String {
        let (idx, twaps) = crate::testdata::one_death();
        let views = crate::chart::render_views(&idx.level);
        render(&idx, &twaps, &views, "7.0").expect("the page renders")
    }

    #[test]
    fn flags_what_the_browser_is_made_to_fetch_elsewhere() {
        let html = r#"<link href="https://fonts.googleapis.com/css2?family=Play" rel="stylesheet">
<script src="//cdn.example.com/x.js"></script><img src="https://x.example/p.png">"#;
        assert_eq!(third_party_loads(html).len(), 3);
    }

    #[test]
    fn a_load_from_a_host_we_run_is_ours() {
        let html = r#"<script src="https://cyberia.to/player.js" async data-download="/music/atlas.shrugged.set.mp3"></script>
<img src="https://static.cyberia.my/x.png">"#;
        assert!(third_party_loads(html).is_empty());
        // a lookalike is still someone else
        let fake = r#"<script src="https://cyberia.to.evil.example/p.js"></script>"#;
        assert_eq!(third_party_loads(fake).len(), 1);
    }

    #[test]
    fn the_page_opens_on_the_whole_history() {
        let html = rendered();
        // ALL is the tab marked on; no other tab is
        assert!(html.contains("<button class=\"tab on\" data-range=\"all\" role=\"tab\">ALL</button>"));
        assert!(!html.contains("class=\"tab on\" data-range=\"7d\""));
    }

    #[test]
    fn a_dead_leg_is_published_as_having_left_the_basket() {
        let html = rendered();
        // its row holds no weight, and the death gauge says what happened
        assert!(html.contains("</span>UX</td>"));
        assert!(html.contains("<td class=\"n\">0%</td><td class=\"n\">left the basket</td></tr>"));
        // the sentence under the table names the day and both figures
        assert!(html.contains(
            "UX left the basket on 1970-01-03 — its average closed at 9.00 against a peak of 100.00."
        ));
        // and the weight bar carries no segment for it
        assert!(!html.contains("title=\"UX —"));
        // a living leg reads its share of the dead one's weight, and of its peak
        assert!(html.contains("<td class=\"n\">21%</td><td class=\"n\">100%</td></tr>"));
    }

    #[test]
    fn leaves_own_assets_and_clickable_links_alone() {
        let html = r#"<link rel="preload" href="/fonts/play-400.woff2" as="font">
<a href="https://fred.stlouisfed.org/">FRED</a><script>const a = "https://x";</script>"#;
        assert!(third_party_loads(html).is_empty());
    }
}

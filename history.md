---
tags: cyberia, protocol, cybernomics, cx
alias: cx history, century index history, index history, cx series
crystal-type: pattern
crystal-domain: cyberia
---
# cx history

the published level of the [[century-index]] from its base date to 2026-11-16. one CX is the basket that cost $1 on 2022-01-07; today it costs $1.31. every price enters as a trailing 365-day average, so the series moves at the pace of a year, not a day — which is what a century-scale obligation asks of it.

## reading

- since 2022-01-07 the basket returned +31.7% in dollars
- the last twelve months: +0.3%
- the range since the base date: 0.78 low, 1.37 high

## holdings

what one CX holds on 2026-11-16 — the weights are fixed and the basket is rebalanced to them every day, so the holdings change daily and the weights never do. of peak is the death gauge: today's trailing average against the highest since the base date.

| leg | one CX holds | unit | fix | weight | of peak |
|---|---|---|---|---|---|
| BTC | 0.000003475 | BTC | 75,824.45 | 20% | 73% |
| ETH | 0.000085182 | ETH | 2,320.41 | 15% | 72% |
| CNY | 1.350 | CNY | 0.146371 | 15% | 93% |
| USD | 0.19766 | USD | 1 | 15% | 100% |
| GOLD | 0.000044437 | troy oz | 4,448.03 | 15% | 99% |
| CU | 0.000009894 | tonne | 13,318.30 | 10% | 100% |
| OIL | 0.0007121 | bbl | 92.51 | 5% | 91% |
| UX | 0.0009484 | lb U3O8 | 69.47 | 5% | 98% |

## monthly level

| month | level | month | level |
|---|---|---|---|
| 2022-01 | 1.01 | 2024-07 | 1.03 |
| 2022-02 | 1.01 | 2024-08 | 1.05 |
| 2022-03 | 1.02 | 2024-09 | 1.07 |
| 2022-04 | 1.02 | 2024-10 | 1.10 |
| 2022-05 | 1.02 | 2024-11 | 1.12 |
| 2022-06 | 1.01 | 2024-12 | 1.15 |
| 2022-07 | 1.00 | 2025-01 | 1.17 |
| 2022-08 | 0.99 | 2025-02 | 1.19 |
| 2022-09 | 0.96 | 2025-03 | 1.19 |
| 2022-10 | 0.93 | 2025-04 | 1.19 |
| 2022-11 | 0.90 | 2025-05 | 1.19 |
| 2022-12 | 0.86 | 2025-06 | 1.20 |
| 2023-01 | 0.84 | 2025-07 | 1.22 |
| 2023-02 | 0.82 | 2025-08 | 1.25 |
| 2023-03 | 0.80 | 2025-09 | 1.27 |
| 2023-04 | 0.79 | 2025-10 | 1.30 |
| 2023-05 | 0.78 | 2025-11 | 1.31 |
| 2023-06 | 0.78 | 2025-12 | 1.32 |
| 2023-07 | 0.79 | 2026-01 | 1.33 |
| 2023-08 | 0.80 | 2026-02 | 1.33 |
| 2023-09 | 0.80 | 2026-03 | 1.35 |
| 2023-10 | 0.81 | 2026-04 | 1.36 |
| 2023-11 | 0.83 | 2026-05 | 1.37 |
| 2023-12 | 0.85 | 2026-06 | 1.37 |
| 2024-01 | 0.87 | 2026-07 | 1.35 |
| 2024-02 | 0.89 | 2026-08 | 1.34 |
| 2024-03 | 0.92 | 2026-09 | 1.33 |
| 2024-04 | 0.95 | 2026-10 | 1.32 |
| 2024-05 | 0.98 | 2026-11 | 1.31 |
| 2024-06 | 1.01 | — | — |

## sources

this series is a reconstruction from free public data — evidence and orientation, never the contractual fix. a signed annex names its own sources with fallbacks, per the cessation waterfall in [[century-index]].

| leg | source | observed range |
|---|---|---|
| BTC | Coinbase BTC-USD daily close | 2021-01-07 … 2026-11-16 · 2140 observations |
| ETH | Coinbase ETH-USD daily close | 2021-01-07 … 2026-11-16 · 2140 observations |
| CNY | ECB reference rate via Frankfurter | 2021-01-07 … 2026-11-13 · 1501 observations |
| USD | quote currency, fixed at 1 | 2021-01-07 … 2021-01-07 · 1 observations |
| GOLD | PAXG-USD daily close, Yahoo Finance (one token = one troy ounce; LBMA PM fix as fallback) | 2020-11-16 … 2026-11-16 · 2192 observations |
| CU | IMF global copper price via FRED (monthly) | 1992-01-01 … 2026-09-01 · 417 observations |
| OIL | Brent Europe spot via FRED (daily) | 1987-05-20 … 2026-11-11 · 9128 observations |
| UX | IMF uranium price via FRED (monthly) | 1992-01-01 … 2026-09-01 · 417 observations |

copper and uranium publish monthly and carry forward between prints, as a closed market does. the uranium assessment is the weakest fix in the basket, which is why it carries the smallest weight.

## rebalancing

each day the index earns the weighted return of its legs at trailing-average prices — `I(t) = I(t−1) · Σ wᵢ · P̄ᵢ(t)/P̄ᵢ(t−1)`. a leg that ran is sold back to its weight, a leg that fell is bought back to it, so CX is the same portfolio for everyone on every day and its composition owes nothing to the base date. buying what falls has a price: a leg going to zero would take the index with it, at the power of its weight. the death rule bounds that — a leg leaves the basket the day its trailing average closes below a tenth of its highest since the base date, its weight goes pro rata to the rest, and it never returns. the most a leg can cost is 1 − 0.1^w: 37% for bitcoin, 29% for a 15% leg, 11% for a 5% leg.

no leg has left the basket.

## what a lease actually paid

the machinery of §2 run from the base date: the basket priced in bitcoin, each annual step collared at +35%/−15%, the dual floor holding underneath. a lease signed on 2022-01-07 at one CX of rent invoiced $0.99 in its first year and $2.49 in its last — +25.6% a year, compounded.

| year | basket in btc | rent owed | invoice | set by |
|---|---|---|---|---|
| 2022-01-07 | 0.00002098 | 0.00002098 | $0.99 | basket |
| 2023-01-07 | 0.00003114 | 0.00003617 | $0.99 | dollar floor |
| 2024-01-07 | 0.00002933 | 0.00003403 | $0.99 | dollar floor |
| 2025-01-07 | 0.00001729 | 0.00002892 | $1.93 | collar, fall held |
| 2026-01-07 | 0.00001303 | 0.00002458 | $2.49 | collar, fall held |

of the 4 anniversaries after signing: the basket set the rent outright in 0 years, the collar capped a rise in 0 and held a fall in 2, the dollar floor bound in 2 and the satoshi floor in 0. where a floor bound, the tenant owed year-zero money regardless of the basket — the dollar leg when bitcoin had fallen, the satoshi leg when it had outrun the basket: the protocol's bitcoin-standard reading. where the collar held a fall, the rent in sats came down 15% a year while the basket in sats fell faster; the dollar invoice still moved with bitcoin. 

the fix is published daily at [cyberia.my/cx](https://cyberia.my/cx) and rebuilt from source by [[cx]]. generated 2026-11-16.

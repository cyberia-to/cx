---
tags: cyberia, protocol, cybernomics
alias: century index, rent index, land rent index, LRI, cyberia index, rent indexation, value preservation index, century annex, rent annex
crystal-type: pattern
crystal-domain: cyberia
---
# century index

the unit of account for century-scale obligations — ticker CX. eight world assets at fixed weights, rebalanced every day: whoever owes the index owes CX, and the weights never change. first application — long-duration payment streams: land leases, city concessions, infrastructure rents ([[development|cyber valley]] instrument B is the first consumer; model annex in §6). as the ecosystem matures, [[marketplace]] listings and treasury accounts quote in the same unit.

## 1. why

a 25–80 year lease outlives every currency it could be written in. the local currency debases fastest; the reserve currency compounded ~1.25x US CPI over 2020–2025 alone; the land itself outgrows any number fixed in year 0. the century index makes the payment a portfolio instead of a number — rent is a number of CX, and what a CX buys moves with the world. a city is a 50-year project funded by 25-year leases: the [[development|bootstrapped city]] stands on rent that keeps its value.

## 2. definition

the index is its weights. eight legs at fixed weights wᵢ; each day CX earns the weighted return of its legs at trailing 365-day average prices P̄ᵢ:

    I(t) = I(t−1) · Σ wᵢ · P̄ᵢ(t) / P̄ᵢ(t−1)        I(2022-01-07) = $1
    R(t) = R₀ · I(t) / I(t₀)                       — the rent, a number of CX

the basket is rebalanced to its weights every day: a leg that ran is sold back to its weight, a leg that fell is bought back to it. CX is therefore the same portfolio for everyone on every day, and its composition owes nothing to the base date — the base is the day one CX cost one dollar, a normalisation and nothing more. one CX holds wᵢ · I(t) / P̄ᵢ(t) of each leg; the holdings change daily, the weights never. the tenant owes R₀ CX, settled in the payment currency at published fixes; renewals scale the same way, L(T) = L₀ · I(T)/I(t₀).

death of a leg. buying what falls has a price: a leg going to zero would take a rebalanced index with it, geometrically, at the power of its weight. so a leg leaves the basket the day its trailing average closes below one tenth of its highest trailing average since the base date; its weight goes pro rata to the remaining legs from the next day, and it never returns. the fall that crosses the line is paid, the recovery, if any, is not collected. the most a leg can cost the index is 1 − 0.1^wᵢ: 37% for bitcoin, 29% for a 15% leg, 11% for a 5% leg. a fixed-quantity basket would have lost at most the weight itself, once — and would have drifted into whichever leg ran, which over a century is the larger failure for a unit of account.

measured in itself the index is constant, so the obligation carries no numéraire: the rent is R₀ CX a year, every year, and nothing sits between the index and the invoice. on the anniversary the R₀ CX are converted into the payment currency at the published fix of that day (T6) — dollars, rupiah or satoshi are the currency of settlement, never the unit of account. no collar and no floor: a clamp needs a ruler outside the index, and a ruler outside the index is a second bet laid over the basket. the smoothing lives inside the index (the 365-day average), and so does the protection (the rebalance, the death rule). a lessor who wants a floor in the settlement currency negotiates it as a separate clause and pays for it in R₀; it is not part of the index. the machines once laid on top — a bitcoin ruler, a collar in sats, a dual floor — were run against the reconstructed path and retired on 2026-11-16: `audit/rent-machine-2026-11-16.md`.

## 3. basket

eight primary assets — things the world prices directly, each with a deep liquid public fix. three layers: fiat of the present (30%), money that outlives fiat (50%), energy (20%).

| leg | weight | layer | primary fix | fallback |
|---|---|---|---|---|
| BTC | 20% | hard money | Pyth BTC/USD daily close | [CME CF BRR](https://www.cfbenchmarks.com/data/indices/BRR), then median of 3 named exchanges |
| ETH | 15% | hard money | Pyth ETH/USD daily close | [CME CF ETH RR](https://www.cfbenchmarks.com/data/indices/ETHUSD_RR), then median of 3 named exchanges |
| GOLD | 15% | hard money | Pyth XAU/USD | [LBMA PM fix](https://www.lbma.org.uk/prices-and-data/precious-metal-prices), then [COMEX settle](https://www.cmegroup.com/markets/metals/precious/gold.html) |
| CNY | 15% | today's fiat | Pyth USD/CNH (offshore quote — freely traded, no PBOC fixing) | [WM/Refinitiv](https://www.lseg.com/en/data-analytics/financial-benchmarks/wm-refinitiv-fx-benchmarks), then [PBOC parity](https://www.chinamoney.com.cn/english/bmkcpr/) |
| USD | 15% | today's fiat | 1 (quote currency) | — |
| CU | 10% | energy | [LME copper cash settle](https://www.lme.com/en/Metals/Non-ferrous/LME-Copper) | [COMEX HG settle](https://www.cmegroup.com/markets/metals/base/copper.html) |
| OIL | 5% | energy | [ICE Brent front-month settle](https://www.ice.com/products/219/Brent-Crude-Futures) | [EIA Brent spot](https://www.eia.gov/dnav/pet/hist/RBRTED.htm) |
| UX | 5% | energy | [CME UxC U3O8 front-month settle](https://www.cmegroup.com/markets/metals/other/uranium.html) | [UxC](https://www.uxc.com/) / [TradeTech](https://www.uranium.info/) weekly spot, [Sprott physical trust](https://sprott.com/investment-strategies/physical-commodity-funds/uranium/) NAV as market check |

fixes are two-tier: aggregated on-chain oracle feeds first ([Pyth](https://pyth.network/price-feeds), 100+ first-party publishers, signed, historical via [benchmarks](https://docs.pyth.network/benchmarks); [Chainlink](https://data.chain.link/) second), institutional settles as the T3 fallback tier. every price enters as a trailing 365-day average of daily fixes — annual TWAP, immune to single-day manipulation. the index publishes 365 days a year; a closed market carries its last fix forward. the physical legs settle on exchanges — the settle IS the world price, and the oracle mirrors it on-chain.

why these eight. BTC is hard money and, through mining difficulty, the only global price of electricity. ETH is the productive digital asset. gold has held value for five thousand years. CNY and USD are the working money of the present. copper is the bottleneck of electrification: an EV carries 3–4x the copper of a combustion car, grids must double, a new mine takes 15 years. oil is the shock hedge — every crisis prices through it first. uranium, held as U3O8 yellowcake (CME ticker UX), is the baseload of the AI age; its fix, an assessment wrapped in an exchange settle, is the weakest in the basket and is sized accordingly.

the primary-asset rule keeps the rest out: equity indices are claims on institutions, carbon allowances are claims on policy, lithium is abundant, hydrogen has no market to price. watchlist, should a basket ever be revised: compute, a depoliticized carbon price.

## 4. mechanics

- reset: annual, on the contract anniversary, using the TWAP window that ends 30 days before payment — the tenant knows the invoice a month ahead
- settlement conversion is never capped — devaluation of the local currency flows through in full
- backtest, the [[cx]] reconstruction at trailing-average prices from the 2022-01-07 base: one CX cost $1.32 on 2026-11-16, and no leg has approached the death line (bitcoin and ether sit at 73% of their peak averages, the rest above 90%). a lease of the base date invoiced, per CX a year, $1.00 → $0.86 → $0.86 → $1.16 → $1.32 on its five anniversaries — the index and nothing else. the largest annual move of the index on this path was +34.5%, the deepest −13.9%: the 365-day average is the collar
- the tenant's and the lessor's bargain lives in the negotiated R₀, never in the formula

## 5. contract theses

model clauses; the annex algorithm is the contract, reproducible by a junior accountant from public data.

- T1 annex: R₀ in CX, I(t₀), the weights, the rebalance and death rule of §2, fix sources with fallbacks, the settlement currency and its fix, one worked invoice. prevails over prose. model: §6.
- T2 fix death ≠ asset death: death of a price SOURCE triggers replacement, which must price the same asset (T3). death of the ASSET is the §2 rule and nothing else — the leg leaves at a tenth of its peak average, its weight goes pro rata to the rest, no party decides anything. a fall short of the line triggers nothing; the basket rides it.
- T3 cessation waterfall: dead = administrator cessation, 30 days unpublished, or methodology change. then: named fallback → regulator-designated successor (LIBOR→SOFR pattern) → equivalent fix by independent expert → last TWAP frozen as a bridge, never a settlement.
- T5 recomputation: tenant may recompute any invoice from public sources within 30 days; recomputation prevails, manifest errors corrected retroactively. index disputes are arithmetic, never renegotiation.
- T6 settlement: the formula adapts to local currency law — e.g. Indonesia ([UU 7/2011](https://peraturan.bpk.go.id/Details/39197/uu-no-7-tahun-2011)) requires IDR settlement at [JISDOR](https://www.bi.go.id/en/statistik/informasi-kurs/jisdor/default.aspx) on the invoice date.
- T7 continuity: the annex survives assignment, sublease, succession; renewals reference the same R₀ in CX. the index, not the parties, defines the obligation.
- T8 oracle precedence: the on-chain fix (§7) is evidence and automation; on divergence the annex computation from public fixes prevails.

## 6. model annex

worked example at R₀ = $100,000/yr with indicative fixes; a signed annex replaces them with the published index at signing.

| parameter | value |
|---|---|
| R₀ (year-0 rent) | $100,000 = 80,000 CX at I(t₀) = $1.25 |
| weights BTC/ETH/GOLD/CNY/USD/CU/OIL/UX | 20/15/15/15/15/10/5/5, rebalanced daily |
| death rule | a leg below a tenth of its peak 365-day average since 2022-01-07 leaves, weight pro rata, never returns |
| reset | annual anniversary, TWAP window ends 30 days before payment |
| settlement | IDR at JISDOR on invoice date (UU 7/2011) |

the obligation is 80,000 CX a year for the life of the contract. what one CX holds on any day is published with the fix — on the example date, wᵢ · $1.25 / P̄ᵢ of each leg — and is nobody's to set.

worked invoice, hypothetical year-1 fix — the index at I(t₁) = $1.356291 (+8.5%), JISDOR 19,000:

1. mark the rent to the index: 80,000 CX · $1.356291 = $108,503.30
2. settle: × 19,000 = Rp 2,061,562,700

reading of this year: the basket grew 8.5%, the rent grew 8.5%. had bitcoin tripled or halved that year, the invoice would read the same — the tenant owes the basket, not any one leg of it.

## 7. oracle

the index publishes as a daily on-chain fix at the Ethereum contract `cyberia.eth/index` — one canonical number any lease, [[marketplace]] listing, or treasury anywhere can reference. per T8 the on-chain fix is evidence and automation: on divergence the annex computation from the named public fixes prevails. the long game: the fix migrates into the [[cybergraph]] — fixed-point over the Goldilocks field per [[soft3]], signed by the publishing [[neuron]], provable by [[zheng]].

what the index has actually read, day by day, lives in [[cx]] — a reconstruction from free public sources, published at [cyberia.my/cx](https://cyberia.my/cx) and recomputed daily.

\* an optional clause exists for long leases: every fifth anniversary, by mutual written consent only, one leg of at most 10% weight may be replaced at the then-current TWAP, value-neutral. the standard annex leaves it out — each review is a negotiation, and a lease that renegotiates its basket loses the certainty the index is for.

# cx

the daily fix of the [century index](https://cx.cyberia.my) — ticker CX, the unit
of account for century-scale obligations. the specification is
[specs/century-index.md](specs/century-index.md); this crate computes and
publishes what it defines.

the pipeline is four steps: pull each leg from its public source, carry every fix
onto a dense daily calendar, average each leg over the trailing 365 days, then
chain the weighted daily returns — the basket rebalanced to its weights every day.

```
cargo run --release -- --out out
```

writes three artefacts:

| file | goes to |
|---|---|
| `index.html` | `cx.cyberia.my` |
| `cx.json` | `cx.cyberia.my/cx.json` |
| `history.md` | `history.md` in this repository — the record, a page of the graph |

the daily workflow in the pages repository (cyberia-to/cyberia.my,
`cx-daily.yml`) runs this at 06:20 UTC, carries the page and the json to the
site, and commits the record here. this repository mounts into the graph as a
child of cyberia, so the record and the specification read as pages there.

## the index

the index is its weights. eight legs at fixed weights — BTC 20, ETH 15, CNY 15,
USD 15, GOLD 15, CU 10, OIL 5, UX 5 — and each day CX earns the weighted return of
its legs at trailing-average prices:

```
I(t) = I(t−1) · Σ wᵢ · P̄ᵢ(t) / P̄ᵢ(t−1)        I(2022-01-07) = $1
```

a leg that ran is sold back to its weight, a leg that fell is bought back to it,
so CX is the same portfolio for everyone on every day and its composition owes
nothing to the base date. the base is a normalisation: the day one CX cost one
dollar.

buying what falls has a price: a leg going to zero would take the index with it,
geometrically at the power of its weight. the death rule bounds that. a leg
leaves the basket the day its trailing average closes below a tenth of its
highest trailing average since the base date; its weight goes pro rata to the
remaining legs from the next day, and it never returns. the most a leg can cost
is 1 − 0.1^w — 37% for bitcoin, 29% for a 15% leg, 11% for a 5% leg. the fall
that crosses the line is paid; the recovery, if any, is not collected.

## the arithmetic

every price, weight and index level is an `i128` scaled by 1e-9; the level is
carried at 1e-18 between days so a day's truncation cannot reach the published
digits over a century of steps. no floating point enters the index path, so the
same day's data yields the same level on any machine — the fix is reproducible
by anyone holding the sources, which is the point of an obligation nobody has to
be trusted about. chart geometry is the one exception, and it rounds to whole
viewBox units.

the crate carries no dependencies: network IO shells out to `curl`, the four feed
shapes have purpose-built parsers.

## sources

| leg | source | cadence |
|---|---|---|
| BTC, ETH | Coinbase Exchange daily candles | daily |
| GOLD | PAXG-USD daily close, Yahoo Finance chart (one token = one troy ounce); LBMA PM fix as fallback — LBMA has answered 403 to machines since 2026-09-29 | daily |
| CNY | ECB reference rate via Frankfurter | daily |
| USD | the quote currency, fixed at 1 | — |
| OIL | Brent Europe spot via FRED | daily |
| CU, UX | IMF copper and uranium assessments via FRED | monthly |

this is a reconstruction from free public data — evidence and orientation, never
the contractual fix. a signed lease names its own sources with fallbacks, per the
cessation waterfall in the protocol.

every leg carries a full 365-day window behind the base date; the rule that a
younger leg averages over what it has, once 90 days exist, stays as a safeguard.

one honest limit, stated on the published pages: copper and uranium print
monthly and carry forward between prints.

## flags

| flag | effect |
|---|---|
| `--out DIR` | where the artefacts land (default `out`) |
| `--cache DIR` | raw response cache (default `out/cache`) |
| `--today YYYY-MM-DD` | pin the run date, for reproducing a past fix |
| `--offline` | refuse the network, serve the cache only |

## tests

```
cargo test --release
```

covers the fixed-point and date arithmetic, the carry-forward rule and its refusal
to invent history, the trailing average including the young-leg case, every feed
parser, and the index invariants — base level, proportional response, a day
earning the weighted return, the rebalancing gain on a round trip, holdings at
the weights after any price path, the death rule firing below a tenth of the
peak and not at it, a dying leg costing at most 1 − 0.1^w, and a century of flat
days not drifting a digit.

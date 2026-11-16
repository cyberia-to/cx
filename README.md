# cx

the daily fix of the [century index](https://cyberia.my/cx) — ticker CX, the unit
of account for century-scale obligations.

the pipeline is four steps: pull each leg from its public source, carry every fix
onto a dense daily calendar, average each leg over the trailing 365 days, then
hold the quantities fixed from the base date and sum them at today's averages.

```
cargo run --release -- --out out
```

writes four artefacts:

| file | goes to |
|---|---|
| `index.html` | `cyberia.my/cx` |
| `cx.json` | `cyberia.my/cx/cx.json` |
| `README.md` | `cyberia/protocol/cx/README.md` |
| `history.md` | `cyberia/protocol/cx/cx history.md` |

## the arithmetic

every price, weight and index level is an `i128` scaled by 1e-9. no floating
point enters the index path, so the same day's data yields the same level on any
machine — the fix is reproducible by anyone holding the sources, which is the
point of an obligation nobody has to be trusted about. chart geometry is the one
exception, and it rounds to whole viewBox units.

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

the base date is fixed: 2022-01-07, the day one CX cost one dollar. a base that
slid with the calendar would recompute the quantities every morning while the
page promised they never change. every leg carries a full 365-day window behind
that date; the rule that a younger leg averages over what it has, once 90 days
exist, stays as a safeguard.

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
parser, and the index invariants — base level, proportional response, a leg going
to zero costing at most its weight.

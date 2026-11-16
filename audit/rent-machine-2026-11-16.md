# the rent machine — decision memo, 2026-11-16

question: what sits between the index and the invoice. today (§2 of the spec,
decided 2026-07-31) the annual rent is priced in bitcoin, collared +35%/−15% a
year in sats, and floored at year-0 sats and year-0 dollars. the owner, reading
the reconstructed path, called the result wrong: the tenant pays $2.50 for a
basket worth $1.32. this memo puts five machines on the same path and asks the
owner to pick one.

## the path

the cx reconstruction from the base, 365-day averages, read on each anniversary
(`cx --offline --cache out/cache --today <date>`; the walk is
`/tmp/cx-decision/paths.py`, a throwaway):

| date | index $ | BTC avg $ | index y/y |
|---|---|---|---|
| 2022-01-07 | 1.0000 | 47,664 | |
| 2023-01-07 | 0.8609 | 27,643 | −13.9% |
| 2024-01-07 | 0.8619 | 29,381 | +0.1% |
| 2025-01-07 | 1.1594 | 67,049 | +34.5% |
| 2026-01-07 | 1.3236 | 101,518 | +14.2% |
| 2026-11-16 | 1.3177 | 75,824 | −0.4% (partial year) |

## the machines

- A, status quo: rent priced in bitcoin, collar +35/−15 in sats, dual floor
- B, plain: invoice = R₀ · I(t)/I(t₀), nothing else
- C, plain + collar +35/−15 in index dollars
- D, plain + year-0 dollar floor
- E, plain + collar ±20 + year-0 floor — the pre-bitcoin design of July

what a lease of one CX a year invoiced, in dollars, and which rule set it:

| machine | 2022 | 2023 | 2024 | 2025 | 2026 | 2026-11 | paid / basket |
|---|---|---|---|---|---|---|---|
| A status quo | 1.00 | 1.00 usd floor | 1.00 usd floor | 1.94 collar down | 2.50 collar down | 1.59 sat floor | 1.38x |
| B plain | 1.00 | 0.86 | 0.86 | 1.16 | 1.32 | 1.32 | 1.00x |
| C + collar | 1.00 | 0.86 | 0.86 | 1.16 | 1.32 | 1.32 | 1.00x |
| D + floor | 1.00 | 1.00 floor | 1.00 floor | 1.16 | 1.32 | 1.32 | 1.04x |
| E + collar ±20 + floor | 1.00 | 1.00 floor | 1.00 floor | 1.16 | 1.32 | 1.32 | 1.04x |

the same machines on three synthetic years, year 0 = 1.00 at BTC 50,000:

| year 1 | A | B | C | D | E |
|---|---|---|---|---|---|
| crash: basket −30%, BTC −70% | 1.00 floor | 0.70 | 0.85 collar | 1.00 floor | 1.00 floor |
| run: basket +30%, BTC ×3 | 3.00 sat floor | 1.30 | 1.30 | 1.30 | 1.20 collar |
| BTC to 1% of itself, basket −20% | 1.00 floor | 0.80 | 0.85 collar | 1.00 floor | 1.00 floor |

## reading

A tracks bitcoin, not the index. once bitcoin outruns the basket, the sat floor
and the sat collar hold the rent in sats while the dollar value of those sats
runs: 2026-01 invoices 1.89 times the basket, the synthetic run 2.3 times. the
protection against the opposite tail is the dollar floor, which is machine D.
A is machine D plus a long bitcoin position the tenant is made to write. over
the four observed years the tenant paid 38% more than the basket was worth.

B pays the basket, exactly, every year. its only exposure is the basket's own
path, which is what the index is for.

C's collar never binds. a 365-day average moves at most +34.5% / −13.9% a year
on this path; the averaging inside the index is already the collar, and the
clause would add text and no behaviour. it earns its place only in a crash
deeper than any year observed, where it shifts 15 points from lessor to tenant.

D is the lessor's put: in 2023–24 the tenant paid 1.00 for 0.86, 4% over the
path; in a crash 1.00 for 0.70. a put has a price, and the price belongs in
R₀, not in the formula — a lessor who wants it negotiates a lower R₀ for it.

E is D with a cap on upside; the cap bites in the run year and nowhere else.

## recommendation

B. the index is the smoothing and the protection; everything laid on top
either does nothing (C) or turns the lease into a bet on something other than
the basket (A). the floor (D) stays available as a negotiated clause, priced in
R₀, in index dollars — never in bitcoin.

what changes on B: spec §2 loses the ruler, collar and floor; §4 loses the
collar and the lessor's-call bullets; §6 annex step 2–5 collapse into one line;
the page's mechanics list and pricer lose the collar; `collared_path` becomes a
plain anniversary path, and the pricer's indexation slider reads the index's
own annual rate, unclamped.

decision: the owner's. this memo is the link.

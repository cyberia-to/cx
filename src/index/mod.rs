//! the basket and the index level.
//!
//! the index is its weights. each day CX earns the weighted return of its
//! legs at trailing-average prices — `I(t) = I(t−1) · Σ wᵢ · P̄ᵢ(t)/P̄ᵢ(t−1)` —
//! so the basket is rebalanced to its weights every day and holds the same
//! portfolio for everyone on every day, whatever the base date.
//!
//! daily rebalancing buys what falls, so a leg going to zero would take the
//! index with it, geometrically at the power of its weight. the death rule
//! bounds that: a leg leaves the basket the day its trailing average closes
//! below a tenth of its highest trailing average since the base date, its
//! weight goes pro rata to the rest, and it never returns.
//!
//! three files: `basket` holds the legs and the weights, `level` walks the days
//! and applies the death rule, `rent` reads the invoice off the level.

mod basket;
mod level;
mod rent;

pub use basket::*;
pub use level::*;
pub use rent::*;

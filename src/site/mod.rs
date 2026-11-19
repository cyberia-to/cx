//! the published page at cx.cyberia.my and the machine-readable fix beside it.
//!
//! three files: `shell` is the HTML document, `page` fills it from the index,
//! `json` publishes the same numbers as data.

mod json;
mod page;
mod shell;

pub use json::*;
pub use page::*;

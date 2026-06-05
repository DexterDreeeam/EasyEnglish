//! `ee-dict` — offline English→Chinese dictionary bundle and access layer.
//!
//! See `Dict/.design.md` for the design and `Dict/.interface.md` for the API
//! contract. The public surface is intentionally small: an `Entry` record,
//! a `DictError` enum, and a `DictStore` that opens or seeds a SQLite file.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod entry;
mod store;

pub use entry::{DictError, Entry};
pub use store::DictStore;

//! `ee-utils` — reusable concurrency utilities shared by `ee-core` and the
//! platform UI crates (`ee-win`, `ee-mac`, `ee-linux`).
//!
//! Currently ships exactly one type family: [`DynamicResult`] +
//! [`DynamicResultProducer`] + [`Signal`], a thread-safe growing-state handle
//! with cancellation and termination signalling. See `Utils/.design.md` and
//! `Utils/.interface.md` for the contract.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod dynamic_result;

pub use dynamic_result::{DynamicResult, DynamicResultProducer, Signal};

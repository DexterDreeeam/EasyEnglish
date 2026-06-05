//! `ee-core` — Core workspace services: Storage, Search, Algo, and Hub.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod record;
#[allow(non_snake_case)]
mod RecordProvider;
mod search;
mod storage;
mod algo;
mod hub;

pub use record::{Record, RecordModel, RecordType, SerializableRecord, WordEn, WordData, Pronunciation, Definition, Inflections, Example, Note, History};
pub use RecordProvider::RecordProvider;
pub use search::Search;
pub use storage::{Storage, StorageError};
pub use hub::Hub;

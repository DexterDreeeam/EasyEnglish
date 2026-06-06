//! `ee-core` — Core workspace services: Storage, Search, Algo, and Hub.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod record;
mod record_provider;
mod search;
mod storage;
mod algo;
mod hub;

pub use record::{Record, RecordModel, RecordType, SerializableRecord, WordEn, WordData, Pronunciation, Definition, Inflections, Example, Note, History};
pub use record_provider::RecordProvider;
pub use search::Search;
pub use storage::{Storage, StorageError};
pub use hub::Hub;
pub use algo::{levenshtein_distance, rank_candidates};

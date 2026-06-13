//! `ee-core` — Core workspace services: Storage, Search, Algo, and Hub.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod algo;
mod hub;
mod record;
mod record_provider;
mod search;
mod storage;

pub use algo::{levenshtein_distance, prefix_candidates, rank_candidates};
pub use hub::Hub;
pub use record::{
    Definition, Example, History, Inflections, Note, Pronunciation, Record, RecordModel,
    RecordType, SerializableRecord, WordCn, WordData, WordEn,
};
pub use record_provider::RecordProvider;
pub use search::Search;
pub use storage::{Storage, StorageError};

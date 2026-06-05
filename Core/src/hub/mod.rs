//! `hub` — Unified orchestrator for multi-source RecordProvider lookups and streaming.

use std::sync::Arc;
use ee_utils::{DynamicResult, DynamicResultProducer};
use crate::{Record, RecordProvider};

/// Unified query orchestrator coordinating multiple `RecordProvider` sources.
pub struct Hub {
    // We wrap individual providers in Arc so that we can easily clone and share them with the spawned background workers
    providers: Vec<Arc<dyn RecordProvider + Send + Sync>>,
}

impl Hub {
    /// Create a new, empty hub instance.
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
        }
    }

    /// Add a new data provider source to this hub.
    pub fn add_provider(&mut self, provider: Arc<dyn RecordProvider + Send + Sync>) {
        self.providers.push(provider);
    }

    /// Query all registered providers asynchronously inside a background worker thread.
    ///
    /// It returns an immediate, non-blocking `DynamicResult<Vec<Record>>` handle.
    /// As results are found sequentially across providers, the dynamic result is updated.
    pub fn query(&self, key: &str) -> DynamicResult<Vec<Record>> {
        let (consumer, producer) = DynamicResult::new(Vec::<Record>::new());

        let key = key.to_string();
        let providers = self.providers.clone();

        // Spawn a background worker thread via ee-utils's DynamicResultProducer.spawn
        producer.spawn(move |prod: &DynamicResultProducer<Vec<Record>>| {
            for provider in providers.iter() {
                enum EventMessage {
                    Result(Option<String>),
                    Cancelled,
                }

                let (tx, rx) = std::sync::mpsc::channel();
                let active = std::sync::Arc::new(std::sync::Mutex::new(true));

                let mut get_result = None;
                let mut cancelled = false;

                // Use a scoped thread context to allow safe borrowing of local references (prod and provider)
                std::thread::scope(|s| {
                    let tx_get = tx.clone();
                    let provider = Arc::clone(provider);
                    let k = key.clone();

                    // Thread 1: Perform the potentially blocking get
                    s.spawn(move || {
                        let res = provider.get(&k);
                        let _ = tx_get.send(EventMessage::Result(res));
                    });

                    let tx_cancel = tx.clone();
                    let active_cancel = std::sync::Arc::clone(&active);

                    // Thread 2: Concurrently wait for cancellation signal
                    s.spawn(move || {
                        while *active_cancel.lock().unwrap() {
                            if prod.wait_for_cancel(Some(std::time::Duration::from_millis(20))) {
                                let _ = tx_cancel.send(EventMessage::Cancelled);
                                break;
                            }
                        }
                    });

                    // Coordinator blocks cleanly on the channel, eliminating busy polling
                    match rx.recv() {
                        Ok(EventMessage::Result(res)) => {
                            get_result = res;
                        }
                        Ok(EventMessage::Cancelled) => {
                            cancelled = true;
                        }
                        Err(_) => {}
                    }

                    // Deactivate Thread 2 so that it exits immediately on scope termination
                    *active.lock().unwrap() = false;
                });

                if cancelled {
                    break;
                }

                if let Some(val) = get_result {
                    let rec = Record::new(&key, val);
                    
                    // Atomically update the consumer's list with the new record
                    prod.update(|records| {
                        records.push(rec);
                    });
                }
            }
        });

        consumer
    }
}

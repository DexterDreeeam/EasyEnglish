//! `hub` — Unified orchestrator for multi-source RecordProvider lookups and streaming.

use crate::{Record, RecordProvider};
use ee_utils::{DynamicResult, DynamicResultProducer};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Unified query orchestrator coordinating multiple `RecordProvider` sources.
pub struct Hub {
    // We wrap individual providers in Arc so that we can easily clone and share them with the spawned background workers
    providers: Vec<Arc<dyn RecordProvider + Send + Sync>>,
    reload_flag: Arc<AtomicBool>,
}

impl Hub {
    /// Create a new, empty hub instance.
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
            reload_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Add a new data provider source to this hub.
    pub fn add_provider(&mut self, provider: Arc<dyn RecordProvider + Send + Sync>) {
        self.providers.push(provider);
    }

    /// Dynamically change the providers list.
    ///
    /// This immediately triggers a reload signal, stopping any active query threads.
    pub fn set_providers(&mut self, new_providers: Vec<Arc<dyn RecordProvider + Send + Sync>>) {
        self.reload_flag.store(true, Ordering::SeqCst);
        self.reload_flag = Arc::new(AtomicBool::new(false));
        self.providers = new_providers;
    }

    /// Query all registered providers asynchronously inside a background worker thread.
    ///
    /// It returns an immediate, non-blocking `DynamicResult<Vec<Record>>` handle.
    /// As results are found sequentially across providers, the dynamic result is updated.
    pub fn query(&self, keys: &[String]) -> DynamicResult<Vec<Record>> {
        let (consumer, producer) = DynamicResult::new(Vec::<Record>::new());

        let keys_cloned: Vec<String> = keys.to_vec();
        let providers = self.providers.clone();
        let reload = Arc::clone(&self.reload_flag);

        // Spawn a background worker thread via ee-utils's DynamicResultProducer.spawn
        producer.spawn(move |prod: &DynamicResultProducer<Vec<Record>>| {
            for key in keys_cloned.iter() {
                for provider in providers.iter() {
                    // Check if reload or cancel was triggered during our query
                    if reload.load(Ordering::SeqCst) {
                        prod.fail("Reload");
                        return;
                    }
                    if prod.wait_for_cancel(Some(std::time::Duration::from_millis(0))) {
                        return;
                    }

                    enum EventMessage {
                        Result(Option<String>),
                        Cancelled,
                    }

                    let (tx, rx) = std::sync::mpsc::channel();
                    let active = std::sync::Arc::new(std::sync::Mutex::new(true));

                    let mut get_result = None;
                    let mut cancelled = false;

                    // Use a scoped thread context to allow safe borrowing of local references (prod and provider)
                    let reload_check = Arc::clone(&reload);
                    let key_for_get = key.clone();
                    std::thread::scope(|s| {
                        let tx_get = tx.clone();
                        let provider = Arc::clone(provider);
                        let k = key_for_get.clone();

                        // Thread 1: Perform the potentially blocking get
                        s.spawn(move || {
                            let res = provider.get(&k);
                            let _ = tx_get.send(EventMessage::Result(res));
                        });

                        let tx_cancel = tx.clone();
                        let active_cancel = std::sync::Arc::clone(&active);

                        // Thread 2: Concurrently wait for cancellation signal or reload signal
                        s.spawn(move || {
                            while *active_cancel.lock().unwrap() {
                                if reload_check.load(Ordering::SeqCst) {
                                    let _ = tx_cancel.send(EventMessage::Cancelled);
                                    break;
                                }
                                if prod.wait_for_cancel(Some(std::time::Duration::from_millis(20)))
                                {
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

                    if cancelled || reload.load(Ordering::SeqCst) {
                        if reload.load(Ordering::SeqCst) {
                            prod.fail("Reload");
                        }
                        return;
                    }

                    if let Some(val) = get_result {
                        let rec = Record::new(key.clone(), val);

                        // Atomically update the consumer's list with the new record
                        prod.update(|records| {
                            records.push(rec);
                        });
                    }
                }
            }
        });

        consumer
    }
}

impl Default for Hub {
    fn default() -> Self {
        Self::new()
    }
}

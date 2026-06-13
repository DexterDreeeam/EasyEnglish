//! `DynamicResult` — a thread-safe shared state that grows over time, with
//! cancellation, completion, failure, and condvar-driven wake-ups.
//!
//! The full contract lives in `Utils/.interface.md`. This module's doc
//! comments cover only what is strictly needed to understand the code.

use std::any::Any;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::time::{Duration, Instant};

/// Outcome of [`DynamicResult::wait`].
#[derive(Debug, Clone)]
pub enum Signal {
    /// The producer mutated the state at least once since the previous wake.
    Changed,
    /// The producer finished normally, the consumer cancelled, or the
    /// producer was dropped. The state at this point is the final state.
    Finished,
    /// The producer reported failure via [`DynamicResultProducer::fail`], or a
    /// worker spawned via [`DynamicResultProducer::spawn`] panicked.
    Failed(String),
    /// The `timeout` argument of `wait` elapsed without any other event.
    TimedOut,
}

struct Inner<T> {
    state: RwLock<T>,
    cancelled: AtomicBool,
    completed: AtomicBool,
    failed: Mutex<Option<String>>,
    mu: Mutex<()>,
    cv: Condvar,
}

impl<T> Inner<T> {
    fn snapshot_terminal(&self) -> Option<Signal> {
        if let Some(reason) = self.failed.lock().expect("failed lock").as_ref() {
            return Some(Signal::Failed(reason.clone()));
        }
        if self.cancelled.load(Ordering::Acquire) || self.completed.load(Ordering::Acquire) {
            return Some(Signal::Finished);
        }
        None
    }

    fn try_set_cancelled(&self) -> bool {
        let _g = self.mu.lock().expect("mu lock");
        if self.snapshot_terminal().is_some() {
            return false;
        }
        self.cancelled.store(true, Ordering::Release);
        true
    }

    fn try_set_completed(&self) -> bool {
        let _g = self.mu.lock().expect("mu lock");
        if self.snapshot_terminal().is_some() {
            return false;
        }
        self.completed.store(true, Ordering::Release);
        true
    }

    fn try_set_failed(&self, reason: String) -> bool {
        let _g = self.mu.lock().expect("mu lock");
        if self.snapshot_terminal().is_some() {
            return false;
        }
        *self.failed.lock().expect("failed lock") = Some(reason);
        true
    }

    fn bump(&self) {
        let _g = self.mu.lock().expect("mu lock");
        self.cv.notify_all();
    }

    fn wait(&self, timeout: Option<Duration>) -> Signal {
        if let Some(s) = self.snapshot_terminal() {
            return s;
        }
        let g = self.mu.lock().expect("mu lock");
        if let Some(s) = self.snapshot_terminal() {
            return s;
        }
        let (_g, timed_out) = match timeout {
            None => (self.cv.wait(g).expect("cv wait"), false),
            Some(t) => {
                let (g2, res) = self.cv.wait_timeout(g, t).expect("cv wait_timeout");
                (g2, res.timed_out())
            }
        };
        if let Some(s) = self.snapshot_terminal() {
            return s;
        }
        if timed_out {
            Signal::TimedOut
        } else {
            Signal::Changed
        }
    }

    fn wait_for_cancel(&self, timeout: Option<Duration>) -> bool {
        let deadline = timeout.map(|t| Instant::now() + t);
        loop {
            if self.cancelled.load(Ordering::Acquire) {
                return true;
            }
            if self.completed.load(Ordering::Acquire)
                || self.failed.lock().expect("failed lock").is_some()
            {
                return false;
            }
            let g = self.mu.lock().expect("mu lock");
            if self.cancelled.load(Ordering::Acquire) {
                return true;
            }
            if self.completed.load(Ordering::Acquire)
                || self.failed.lock().expect("failed lock").is_some()
            {
                return false;
            }
            let timed_out = match deadline {
                None => {
                    let _guard = self.cv.wait(g).expect("cv wait");
                    false
                }
                Some(d) => {
                    let now = Instant::now();
                    if now >= d {
                        return self.cancelled.load(Ordering::Acquire);
                    }
                    let (_guard, res) = self.cv.wait_timeout(g, d - now).expect("cv wait_timeout");
                    res.timed_out()
                }
            };
            if timed_out {
                return self.cancelled.load(Ordering::Acquire);
            }
        }
    }

    fn write_and_bump(&self, f: impl FnOnce(&mut T)) {
        let mut g = match self.state.write() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        f(&mut g);
        drop(g);
        self.bump();
    }
}

/// Consumer-side handle. Reads state, observes signals, and may cancel.
pub struct DynamicResult<T: Send + Sync + 'static> {
    inner: Arc<Inner<T>>,
}

/// Producer-side handle. Mutates state and signals completion / failure.
pub struct DynamicResultProducer<T: Send + Sync + 'static> {
    inner: Arc<Inner<T>>,
}

impl<T: Send + Sync + 'static> DynamicResult<T> {
    /// Construct. Returns `(consumer, producer)` sharing one allocation.
    pub fn new(initial: T) -> (Self, DynamicResultProducer<T>) {
        let inner = Arc::new(Inner {
            state: RwLock::new(initial),
            cancelled: AtomicBool::new(false),
            completed: AtomicBool::new(false),
            failed: Mutex::new(None),
            mu: Mutex::new(()),
            cv: Condvar::new(),
        });
        (
            DynamicResult {
                inner: inner.clone(),
            },
            DynamicResultProducer { inner },
        )
    }

    /// Clone snapshot of the current state.
    pub fn get(&self) -> T
    where
        T: Clone,
    {
        match self.inner.state.read() {
            Ok(g) => g.clone(),
            Err(p) => p.into_inner().clone(),
        }
    }

    /// Signal cancellation to the producer. Idempotent.
    pub fn cancel(&self) {
        if self.inner.try_set_cancelled() {
            self.inner.bump();
        }
    }

    /// Block until the state changes, the result terminates, or `timeout`
    /// elapses. `None` means wait indefinitely.
    pub fn wait(&self, timeout: Option<Duration>) -> Signal {
        self.inner.wait(timeout)
    }
}

impl<T: Send + Sync + 'static> Drop for DynamicResult<T> {
    fn drop(&mut self) {
        if self.inner.try_set_cancelled() {
            self.inner.bump();
        }
    }
}

impl<T: Send + Sync + 'static> DynamicResultProducer<T> {
    /// Mutate the state under a write lock. Always runs `f`. Always notifies
    /// waiters.
    pub fn update(&self, f: impl FnOnce(&mut T)) {
        self.inner.write_and_bump(f);
    }

    /// Mark the result completed. Idempotent. No-op if already terminated.
    pub fn complete(&self) {
        if self.inner.try_set_completed() {
            self.inner.bump();
        }
    }

    /// Mark the result failed with `reason`. Idempotent. No-op if already
    /// terminated.
    pub fn fail(&self, reason: impl Into<String>) {
        if self.inner.try_set_failed(reason.into()) {
            self.inner.bump();
        }
    }

    /// Block until the consumer signals cancellation, the result otherwise
    /// terminates, or `timeout` elapses. Returns `true` only if cancellation
    /// was observed.
    pub fn wait_for_cancel(&self, timeout: Option<Duration>) -> bool {
        self.inner.wait_for_cancel(timeout)
    }

    /// Spawn a worker thread running `f`. If `f` panics, the panic message is
    /// captured and forwarded via [`Self::fail`]. Otherwise the producer is
    /// dropped at the end of `f`, which calls [`Self::complete`].
    pub fn spawn<F>(self, f: F)
    where
        F: FnOnce(&Self) + Send + 'static,
    {
        std::thread::spawn(move || {
            let producer = self;
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                f(&producer);
            }));
            if let Err(payload) = result {
                producer.fail(format!("worker panicked: {}", payload_to_string(payload)));
            }
        });
    }
}

impl<T: Send + Sync + 'static> Drop for DynamicResultProducer<T> {
    fn drop(&mut self) {
        if self.inner.try_set_completed() {
            self.inner.bump();
        }
    }
}

fn payload_to_string(payload: Box<dyn Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&'static str>() {
        return (*s).to_string();
    }
    if let Some(s) = payload.downcast_ref::<String>() {
        return s.clone();
    }
    "panic with non-string payload".to_string()
}

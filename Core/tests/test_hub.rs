//! Integration tests for `Hub` — see `Core/tests/.test.md`.

use std::sync::Arc;
use ee_core::{Hub, Record, RecordProvider};
use ee_utils::Signal;

struct MockProvider {
    name: String,
    value: String,
}

impl RecordProvider for MockProvider {
    fn get(&self, key: &str) -> Option<String> {
        if key == "test" {
            Some(self.value.clone())
        } else {
            None
        }
    }
}

#[test]
fn hub_queries_multiple_providers_streaming() {
    let mut hub = Hub::new();
    
    hub.add_provider(Arc::new(MockProvider {
        name: "p1".to_string(),
        value: "v1".to_string(),
    }));
    hub.add_provider(Arc::new(MockProvider {
        name: "p2".to_string(),
        value: "v2".to_string(),
    }));

    let result_handle = hub.query("test");
    
    // Wait for the background worker thread to finish processing
    let mut finished = false;
    for _ in 0..50 {
        match result_handle.wait(Some(std::time::Duration::from_millis(10))) {
            Signal::Finished => {
                finished = true;
                break;
            }
            _ => {}
        }
    }

    assert!(finished);
    let records = result_handle.get();
    
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].value, "v1");
    assert_eq!(records[1].value, "v2");
}

struct SlowProvider {
    delay: std::time::Duration,
}

impl RecordProvider for SlowProvider {
    fn get(&self, key: &str) -> Option<String> {
        std::thread::sleep(self.delay);
        if key == "test" {
            Some("slow_val".to_string())
        } else {
            None
        }
    }
}

#[test]
fn hub_can_be_cancelled_mid_get() {
    let mut hub = Hub::new();
    hub.add_provider(Arc::new(SlowProvider {
        delay: std::time::Duration::from_millis(500),
    }));

    let result_handle = hub.query("test");
    
    // Let the worker loop run and hit the wait, then immediately cancel
    std::thread::sleep(std::time::Duration::from_millis(50));
    result_handle.cancel();

    // Verify it finishes immediately without waiting the full 500ms
    let start = std::time::Instant::now();
    let sig = result_handle.wait(Some(std::time::Duration::from_millis(200)));
    let elapsed = start.elapsed();

    assert!(matches!(sig, Signal::Finished));
    assert!(elapsed < std::time::Duration::from_millis(200));
}

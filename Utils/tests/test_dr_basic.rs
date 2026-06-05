//! Basic behavior of `DynamicResult` / `DynamicResultProducer`.

use std::time::Duration;

use ee_utils::{DynamicResult, DynamicResultProducer, Signal};

#[test]
fn new_returns_consumer_and_producer() {
    let (_r, _p): (DynamicResult<i32>, DynamicResultProducer<i32>) = DynamicResult::new(0);
}

#[test]
fn get_returns_initial_value() {
    let (r, _p) = DynamicResult::new(42_i32);
    assert_eq!(r.get(), 42);
}

#[test]
fn get_reflects_update() {
    let (r, p) = DynamicResult::new(Vec::<i32>::new());
    p.update(|v| v.push(1));
    p.update(|v| v.push(2));
    assert_eq!(r.get(), vec![1, 2]);
}

#[test]
fn cancel_then_wait_returns_finished() {
    let (r, _p) = DynamicResult::new(0_i32);
    r.cancel();
    match r.wait(Some(Duration::from_millis(50))) {
        Signal::Finished => {}
        other => panic!("expected Finished, got {:?}", other),
    }
}

#[test]
fn wait_with_zero_timeout_returns_timed_out_when_no_event_pending() {
    let (r, _p) = DynamicResult::new(0_i32);
    match r.wait(Some(Duration::from_millis(0))) {
        Signal::TimedOut => {}
        other => panic!("expected TimedOut, got {:?}", other),
    }
}

#[test]
fn update_wakes_waiter_with_changed() {
    use std::sync::Arc;
    let (r, p) = DynamicResult::new(0_i32);
    // Keep the producer alive in main until after the consumer's wait returns,
    // so the worker thread's drop doesn't race with the wait wake-up and turn
    // the expected `Changed` into a `Finished`.
    let p_main = Arc::new(p);
    let p_thread = p_main.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(20));
        p_thread.update(|v| *v = 99);
    });
    let s = r.wait(Some(Duration::from_secs(2)));
    match s {
        Signal::Changed => assert_eq!(r.get(), 99),
        other => panic!("expected Changed, got {:?}", other),
    }
    drop(p_main);
}

#[test]
fn update_after_cancel_still_runs_state_visible_via_get() {
    // D-always semantics: update always runs even after cancel.
    let (r, p) = DynamicResult::new(0_i32);
    r.cancel();
    p.update(|v| *v = 7);
    assert_eq!(r.get(), 7);
}

#[test]
fn dynamic_result_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<DynamicResult<Vec<i32>>>();
}

#[test]
fn producer_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<DynamicResultProducer<Vec<i32>>>();
}

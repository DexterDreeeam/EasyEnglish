//! Producer surface: `complete`, `fail`, `wait_for_cancel`, Drop semantics.

use std::time::{Duration, Instant};

use ee_utils::{DynamicResult, Signal};

const SHORT: Duration = Duration::from_millis(50);

#[test]
fn complete_yields_finished_to_consumer() {
    let (r, p) = DynamicResult::new(0_i32);
    p.complete();
    assert!(matches!(r.wait(Some(SHORT)), Signal::Finished));
}

#[test]
fn complete_is_idempotent() {
    let (r, p) = DynamicResult::new(0_i32);
    p.complete();
    p.complete();
    p.complete();
    assert!(matches!(r.wait(Some(SHORT)), Signal::Finished));
}

#[test]
fn complete_after_cancel_does_not_overwrite_terminal_state() {
    let (r, p) = DynamicResult::new(0_i32);
    r.cancel();
    p.complete();
    assert!(matches!(r.wait(Some(SHORT)), Signal::Finished));
}

#[test]
fn fail_propagates_reason_via_signal() {
    let (r, p) = DynamicResult::new(0_i32);
    p.fail("disk error");
    match r.wait(Some(SHORT)) {
        Signal::Failed(reason) => assert_eq!(reason, "disk error"),
        other => panic!("expected Failed, got {:?}", other),
    }
}

#[test]
fn fail_is_idempotent_first_reason_wins() {
    let (r, p) = DynamicResult::new(0_i32);
    p.fail("first reason");
    p.fail("second reason");
    match r.wait(Some(SHORT)) {
        Signal::Failed(reason) => assert_eq!(reason, "first reason"),
        other => panic!("expected Failed, got {:?}", other),
    }
}

#[test]
fn fail_after_complete_is_noop() {
    let (r, p) = DynamicResult::new(0_i32);
    p.complete();
    p.fail("too late");
    assert!(matches!(r.wait(Some(SHORT)), Signal::Finished));
}

#[test]
fn wait_for_cancel_returns_true_on_consumer_cancel() {
    let (r, p) = DynamicResult::new(0_i32);
    let handle = std::thread::spawn(move || p.wait_for_cancel(Some(Duration::from_secs(2))));
    std::thread::sleep(Duration::from_millis(20));
    r.cancel();
    assert!(handle.join().unwrap());
}

#[test]
fn wait_for_cancel_returns_false_on_complete() {
    let (r, p) = DynamicResult::new(0_i32);
    let p_arc = std::sync::Arc::new(p);
    let p_for_thread = p_arc.clone();
    let handle =
        std::thread::spawn(move || p_for_thread.wait_for_cancel(Some(Duration::from_secs(2))));
    std::thread::sleep(Duration::from_millis(20));
    p_arc.complete();
    assert!(!handle.join().unwrap());
    drop(r);
}

#[test]
fn wait_for_cancel_returns_false_on_fail() {
    let (r, p) = DynamicResult::new(0_i32);
    let p_arc = std::sync::Arc::new(p);
    let p_for_thread = p_arc.clone();
    let handle =
        std::thread::spawn(move || p_for_thread.wait_for_cancel(Some(Duration::from_secs(2))));
    std::thread::sleep(Duration::from_millis(20));
    p_arc.fail("boom");
    assert!(!handle.join().unwrap());
    drop(r);
}

#[test]
fn wait_for_cancel_returns_false_on_timeout() {
    let (_r, p) = DynamicResult::new(0_i32);
    let started = Instant::now();
    let cancelled = p.wait_for_cancel(Some(Duration::from_millis(30)));
    assert!(!cancelled);
    assert!(started.elapsed() < Duration::from_millis(500));
}

#[test]
fn wait_for_cancel_returns_immediately_if_already_cancelled() {
    let (r, p) = DynamicResult::new(0_i32);
    r.cancel();
    let started = Instant::now();
    let cancelled = p.wait_for_cancel(Some(Duration::from_secs(5)));
    assert!(cancelled);
    assert!(started.elapsed() < Duration::from_millis(50));
}

#[test]
fn drop_producer_calls_complete() {
    let (r, p) = DynamicResult::new(0_i32);
    drop(p);
    assert!(matches!(r.wait(Some(SHORT)), Signal::Finished));
}

#[test]
fn drop_producer_after_cancel_is_noop() {
    let (r, p) = DynamicResult::new(0_i32);
    r.cancel();
    drop(p);
    assert!(matches!(r.wait(Some(SHORT)), Signal::Finished));
}

#[test]
fn drop_producer_after_fail_keeps_failed_signal() {
    let (r, p) = DynamicResult::new(0_i32);
    p.fail("boom");
    drop(p);
    match r.wait(Some(SHORT)) {
        Signal::Failed(reason) => assert_eq!(reason, "boom"),
        other => panic!("expected Failed, got {:?}", other),
    }
}

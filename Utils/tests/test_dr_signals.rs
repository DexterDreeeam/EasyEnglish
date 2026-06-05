//! `Signal` variants and the terminal-state decision table.

use std::time::Duration;

use ee_utils::{DynamicResult, Signal};

const SHORT: Duration = Duration::from_millis(50);

#[test]
fn cancel_yields_finished() {
    let (r, _p) = DynamicResult::new(0_i32);
    r.cancel();
    matches_finished(r.wait(Some(SHORT)));
}

#[test]
fn complete_yields_finished() {
    let (r, p) = DynamicResult::new(0_i32);
    p.complete();
    matches_finished(r.wait(Some(SHORT)));
}

#[test]
fn fail_yields_failed_with_reason() {
    let (r, p) = DynamicResult::new(0_i32);
    p.fail("storage error: disk full");
    match r.wait(Some(SHORT)) {
        Signal::Failed(reason) => assert_eq!(reason, "storage error: disk full"),
        other => panic!("expected Failed, got {:?}", other),
    }
}

#[test]
fn cancel_before_complete_wait_returns_finished() {
    let (r, p) = DynamicResult::new(0_i32);
    r.cancel();
    p.complete();
    matches_finished(r.wait(Some(SHORT)));
}

#[test]
fn fail_before_cancel_wait_returns_failed_not_finished() {
    let (r, p) = DynamicResult::new(0_i32);
    p.fail("boom");
    r.cancel();
    match r.wait(Some(SHORT)) {
        Signal::Failed(reason) => assert_eq!(reason, "boom"),
        other => panic!("expected Failed, got {:?}", other),
    }
}

#[test]
fn fail_first_reason_wins() {
    let (r, p) = DynamicResult::new(0_i32);
    p.fail("first");
    p.fail("second");
    match r.wait(Some(SHORT)) {
        Signal::Failed(reason) => assert_eq!(reason, "first"),
        other => panic!("expected Failed, got {:?}", other),
    }
}

#[test]
fn wait_after_terminated_returns_terminal_immediately() {
    let (r, _p) = DynamicResult::new(0_i32);
    r.cancel();
    let started = std::time::Instant::now();
    let s = r.wait(None);
    assert!(started.elapsed() < Duration::from_millis(20));
    matches_finished(s);
}

#[test]
fn timeout_returns_timed_out_when_no_event_pending() {
    let (r, _p) = DynamicResult::new(0_i32);
    let started = std::time::Instant::now();
    match r.wait(Some(Duration::from_millis(40))) {
        Signal::TimedOut => {}
        other => panic!("expected TimedOut, got {:?}", other),
    }
    // ~40ms but allow generous margin for slow runners.
    assert!(started.elapsed() < Duration::from_millis(500));
}

#[test]
fn timeout_returns_changed_when_update_arrives_first() {
    use std::sync::Arc;
    let (r, p) = DynamicResult::new(0_i32);
    let p_main = Arc::new(p);
    let p_thread = p_main.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(20));
        p_thread.update(|v| *v = 7);
    });
    match r.wait(Some(Duration::from_secs(2))) {
        Signal::Changed => assert_eq!(r.get(), 7),
        other => panic!("expected Changed, got {:?}", other),
    }
    drop(p_main);
}

fn matches_finished(s: Signal) {
    match s {
        Signal::Finished => {}
        other => panic!("expected Finished, got {:?}", other),
    }
}

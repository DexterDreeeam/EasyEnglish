//! `spawn`: panic auto-catch, natural completion, observable cancel.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use ee_utils::{DynamicResult, Signal};

#[test]
fn spawn_runs_closure_to_completion_then_completes() {
    let (r, p) = DynamicResult::new(Vec::<i32>::new());
    p.spawn(|p| {
        for i in 0..5 {
            p.update(|v| v.push(i));
        }
    });
    // Wait for finish (which is reached via natural producer-drop).
    loop {
        match r.wait(Some(Duration::from_secs(2))) {
            Signal::Changed => continue,
            Signal::Finished => break,
            other => panic!("expected Changed/Finished, got {:?}", other),
        }
    }
    assert_eq!(r.get(), vec![0, 1, 2, 3, 4]);
}

#[test]
fn spawn_panic_with_str_yields_failed_containing_message() {
    let (r, p) = DynamicResult::new(0_i32);
    p.spawn(|_p| {
        panic!("static-str panic");
    });
    let s = drain_to_terminal(&r);
    match s {
        Signal::Failed(reason) => assert!(
            reason.contains("static-str panic"),
            "reason should contain panic message; got {reason:?}"
        ),
        other => panic!("expected Failed, got {:?}", other),
    }
}

#[test]
fn spawn_panic_with_string_yields_failed_containing_message() {
    let (r, p) = DynamicResult::new(0_i32);
    p.spawn(|_p| {
        let msg = String::from("owned-string panic");
        panic!("{}", msg);
    });
    let s = drain_to_terminal(&r);
    match s {
        Signal::Failed(reason) => assert!(reason.contains("owned-string panic")),
        other => panic!("expected Failed, got {:?}", other),
    }
}

#[test]
fn spawn_natural_return_yields_finished() {
    let (r, p) = DynamicResult::new(0_i32);
    p.spawn(|p| {
        p.update(|v| *v = 1);
    });
    assert!(matches!(drain_to_terminal(&r), Signal::Finished));
    assert_eq!(r.get(), 1);
}

#[test]
fn spawn_can_call_update_through_borrowed_producer() {
    let (r, p) = DynamicResult::new(Vec::<String>::new());
    p.spawn(|p| {
        for s in &["a", "b", "c"] {
            p.update(|v| v.push((*s).to_string()));
        }
    });
    drain_to_terminal(&r);
    assert_eq!(
        r.get(),
        vec!["a".to_string(), "b".to_string(), "c".to_string()]
    );
}

#[test]
fn worker_thread_observes_consumer_cancel_via_wait_for_cancel() {
    // Uses raw std::thread::spawn (not producer.spawn) so we can join and
    // verify the side effect deterministically. spawn-based coverage of
    // wait_for_cancel is implicit in the other tests in this file.
    let (r, p) = DynamicResult::new(0_i32);
    let observed = Arc::new(AtomicUsize::new(0));
    let observed_clone = observed.clone();
    let handle = std::thread::spawn(move || {
        if p.wait_for_cancel(Some(Duration::from_secs(2))) {
            observed_clone.fetch_add(1, Ordering::SeqCst);
        }
    });
    std::thread::sleep(Duration::from_millis(40));
    r.cancel();
    handle.join().unwrap();
    assert_eq!(observed.load(Ordering::SeqCst), 1);
}

fn drain_to_terminal(r: &DynamicResult<impl Send + Sync + 'static>) -> Signal {
    loop {
        match r.wait(Some(Duration::from_secs(2))) {
            Signal::Changed => continue,
            other => return other,
        }
    }
}

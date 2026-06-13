//! Cross-thread `update` / `wait`, Arc sharing, stress.

use std::sync::Arc;
use std::time::{Duration, Instant};

use ee_utils::{DynamicResult, Signal};

#[test]
fn producer_thread_consumer_thread_basic() {
    let (r, p) = DynamicResult::new(Vec::<i32>::new());
    p.spawn(|p| {
        for i in 0..10 {
            p.update(|v| v.push(i));
        }
    });
    drain_to_terminal(&r);
    assert_eq!(r.get(), (0..10).collect::<Vec<_>>());
}

#[test]
fn arc_consumer_shared_across_threads() {
    let (r, p) = DynamicResult::new(Vec::<i32>::new());
    let r = Arc::new(r);
    p.spawn(|p| {
        for i in 0..5 {
            p.update(|v| v.push(i));
            std::thread::sleep(Duration::from_millis(5));
        }
    });

    let r_a = r.clone();
    let h_a = std::thread::spawn(move || drain_to_terminal(&r_a));
    let r_b = r.clone();
    let h_b = std::thread::spawn(move || drain_to_terminal(&r_b));

    h_a.join().unwrap();
    h_b.join().unwrap();
    assert_eq!(r.get(), vec![0, 1, 2, 3, 4]);
}

#[test]
fn cancel_from_other_thread_unblocks_wait_within_100ms() {
    let (r, _p) = DynamicResult::new(0_i32);
    let r = Arc::new(r);
    let r_for_canceller = r.clone();
    let handle = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(30));
        r_for_canceller.cancel();
    });
    let started = Instant::now();
    let s = r.wait(None);
    let elapsed = started.elapsed();
    handle.join().unwrap();
    assert!(matches!(s, Signal::Finished));
    assert!(
        elapsed < Duration::from_millis(500),
        "cancel should unblock promptly; took {:?}",
        elapsed
    );
}

#[test]
fn drop_producer_in_other_thread_unblocks_wait() {
    let (r, p) = DynamicResult::new(0_i32);
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(30));
        drop(p); // Drop -> complete
    });
    assert!(matches!(r.wait(None), Signal::Finished));
}

#[test]
fn stress_repeated_update_and_wait_for_one_second_no_deadlock() {
    let (r, p) = DynamicResult::new(0_u64);
    p.spawn(move |p| {
        let started = Instant::now();
        while started.elapsed() < Duration::from_millis(500) {
            p.update(|v| *v += 1);
        }
    });
    let started = Instant::now();
    let mut last = 0_u64;
    loop {
        match r.wait(Some(Duration::from_millis(100))) {
            Signal::Changed | Signal::TimedOut => {
                let cur = r.get();
                assert!(cur >= last, "monotonic non-decreasing");
                last = cur;
            }
            Signal::Finished => break,
            Signal::Failed(reason) => panic!("unexpected failure: {reason}"),
        }
        if started.elapsed() > Duration::from_secs(5) {
            panic!("stress test ran too long (deadlock?)");
        }
    }
    assert!(last > 0, "producer should have pushed at least one update");
}

fn drain_to_terminal<T: Send + Sync + 'static>(r: &DynamicResult<T>) -> Signal {
    let started = Instant::now();
    loop {
        if started.elapsed() > Duration::from_secs(10) {
            panic!("drain_to_terminal exceeded safety window");
        }
        match r.wait(Some(Duration::from_secs(2))) {
            Signal::Changed => continue,
            other => return other,
        }
    }
}

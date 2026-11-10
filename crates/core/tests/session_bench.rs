//! Cost of a step back after 10⁶ executed steps (target < 50 ms natively).
//!
//! `cargo test --release -p stapelwerk-core --test session_bench -- --ignored --nocapture`

use std::time::{Duration, Instant};

use stapelwerk_core::machine::Status;
use stapelwerk_core::session::Session;

const SPIN: &str = "\
main:   mov r1, 0
        li r2, 0x8000
        mov r0, 64
        sys ALLOC
        mov r3, r0
.loop:  add r1, 1
        and r4, r1, 4095
        li r5, SCREEN
        add r5, r4
        stb r1, [r5]
        st r1, [r3]
        jmp .loop
";

fn timed(f: impl FnOnce()) -> Duration {
    let t = Instant::now();
    f();
    t.elapsed()
}

#[test]
#[ignore = "benchmark; run in release with --ignored --nocapture"]
fn step_back_after_a_million_steps() {
    let mut s = Session::sandbox();
    s.assemble(SPIN).expect("assembles");
    let run = timed(|| assert_eq!(s.step(1_000_000), Status::Paused));
    let now = s.steps();
    assert_eq!(now, 1_000_000);
    let k = s.snapshot_interval();

    // back(1) from 10⁶, several times in a row.
    let mut worst = Duration::ZERO;
    let mut total = Duration::ZERO;
    for _ in 0..20 {
        let d = timed(|| {
            s.back(1);
        });
        worst = worst.max(d);
        total += d;
    }
    assert_eq!(s.steps(), now - 20);

    // Worst case: one step before a snapshot boundary (replays K − 1 steps).
    s.goto(now);
    let t = now / k * k - 1;
    let worst_goto = timed(|| {
        s.goto(t);
    });
    assert_eq!(s.steps(), t);

    println!(
        "10^6 steps forward: {run:?}; K = {k}, snapshots = {}; back(1): avg {:?}, max {worst:?}; \
         goto(boundary − 1) = {worst_goto:?}",
        s.snapshot_count(),
        total / 20,
    );
    if !cfg!(debug_assertions) {
        assert!(worst < Duration::from_millis(50));
        assert!(worst_goto < Duration::from_millis(50));
    }
}

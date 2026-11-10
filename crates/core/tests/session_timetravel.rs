//! Time-travel invariant (SPEC §17 test 8): after N steps with ≥ 3 thinnings, `goto(k)` in random
//! order reproduces the digest d_k recorded on the way (screen, heap, output included), with
//! `set_reg` edits and interactive input given while WAITING_INPUT, and after discarding a future.

use std::collections::BTreeMap;

use stapelwerk_core::machine::Status;
use stapelwerk_core::rng::SplitMix64;
use stapelwerk_core::session::{Session, SessionConfig};

/// Reads numbers until 0; per number one heap block and a long loop that paints random pixels,
/// writes into the block and prints a dot every 1024 iterations.
const PROGRAM: &str = "\
main:   mov r4, 0           ; total
        mov r5, 0           ; current block
round:  mov r0, r5
        sys FREE
        sys READ_INT
        cmp r0, 0
        jz done
        add r4, r0
        mov r0, 24
        sys ALLOC
        mov r5, r0
        st r4, [r5]
        mov r6, 15000
inner:  sys RAND
        and r0, 4095
        li r1, SCREEN
        add r1, r0
        stb r6, [r1]
        st r6, [r5 + 4]
        and r2, r6, 1023
        jnz .skip
        mov r0, 46
        sys WRITE_CHAR
.skip:  sub r6, 1
        jnz inner
        mov r0, r4
        sys WRITE_INT
        jmp round
done:   mov r0, r4
        sys WRITE_INT
        halt
";

const INPUT: [i32; 5] = [5, 7, 11, 13, 0];

struct Rec {
    digest: String,
    lines: Vec<(u32, u64)>,
}

fn record(s: &Session, map: &mut BTreeMap<u64, Rec>) {
    map.insert(
        s.steps(),
        Rec {
            digest: s.digest(),
            lines: s.line_counts(),
        },
    );
}

/// Steps forward in random chunks until HALTED, recording every PAUSED state; feeds input on
/// WAITING_INPUT (from the second wait on, after going back and forward again first) and edits r3 now and then.
fn drive(s: &mut Session, rng: &mut SplitMix64, map: &mut BTreeMap<u64, Rec>, input: &mut usize) {
    let mut waits = 0;
    let mut chunks = 0u32;
    loop {
        let n = 1 + rng.below(6000);
        match s.step(n) {
            Status::Paused => {
                chunks += 1;
                if chunks.is_multiple_of(37) {
                    assert_eq!(s.set_reg(3, rng.next_u32()), Ok(Status::Paused));
                }
                record(s, map);
            }
            Status::Halted => {
                record(s, map);
                return;
            }
            Status::WaitingInput => {
                waits += 1;
                let w = s.steps();
                if waits > 1 {
                    // Back without input, forward again: the program waits at the same step.
                    s.back(300);
                    assert_eq!(s.steps(), w - 300);
                    assert_eq!(s.goto(w), Status::Paused);
                    assert_eq!(s.steps(), w);
                    assert_eq!(s.step(1), Status::WaitingInput);
                    assert_eq!(s.steps(), w);
                }
                let v = INPUT
                    .get(*input)
                    .copied()
                    .expect("program wants more input");
                *input += 1;
                assert_eq!(s.push_int(v), Status::WaitingInput);
            }
            other => panic!("unexpected status {other:?} at step {}", s.steps()),
        }
    }
}

fn check(s: &mut Session, rng: &mut SplitMix64, map: &BTreeMap<u64, Rec>, picks: usize) {
    let keys: Vec<u64> = map.keys().copied().collect();
    let mut order: Vec<u64> = Vec::with_capacity(picks + 2);
    order.push(0);
    order.push(*keys.last().unwrap());
    for _ in 0..picks {
        order.push(keys[rng.below(keys.len() as u64) as usize]);
    }
    for k in order {
        s.goto(k);
        assert_eq!(s.steps(), k);
        let rec = &map[&k];
        assert_eq!(s.digest(), rec.digest, "digest at step {k}");
        assert_eq!(s.line_counts(), rec.lines, "line counts at step {k}");
    }
}

#[test]
fn goto_reproduces_recorded_digests() {
    let mut s = Session::new(SessionConfig {
        seed: 4711,
        ..SessionConfig::default()
    });
    s.assemble(PROGRAM).expect("assembles");
    let mem = s.memory().as_ptr();
    let mut rng = SplitMix64::new(0x5EED);
    let mut map = BTreeMap::new();
    let mut input = 0;
    record(&s, &mut map);
    drive(&mut s, &mut rng, &mut map, &mut input);
    assert_eq!(input, INPUT.len());

    let end = s.steps();
    assert!(end > 4 * 15_000 * 9, "{end} steps");
    assert!(s.thin_count() >= 3, "thinned {}×", s.thin_count());
    assert_eq!(s.snapshot_interval(), 1024 << s.thin_count());
    assert!(s.snapshot_count() <= 64);
    let (kinds, vals) = s.output();
    assert!(kinds.len() > 50, "dots and sums on the tape");
    assert_eq!(vals.last(), Some(&36));
    assert_eq!(s.state().heap.blocks().len(), 1);
    assert!(s.state().mem.screen().iter().any(|&b| b != 0));
    assert!(map.len() > 100);

    check(&mut s, &mut rng, &map, 80);

    // Discard the future: edit in the middle, record the new future, check again.
    let keys: Vec<u64> = map.keys().copied().collect();
    let mid = keys[keys.len() / 2];
    s.goto(mid);
    assert_eq!(s.set_reg(3, 0xDEAD_BEEF), Ok(Status::Paused));
    let old_end = map[keys.last().unwrap()].digest.clone();
    map.retain(|&k, _| k <= mid);
    record(&s, &mut map);
    drive(&mut s, &mut rng, &mut map, &mut input);
    assert_eq!(s.steps(), end, "the program ignores r3");
    assert_ne!(s.digest(), old_end, "r3 differs at the end");
    check(&mut s, &mut rng, &map, 40);

    assert_eq!(s.memory().as_ptr(), mem);
}

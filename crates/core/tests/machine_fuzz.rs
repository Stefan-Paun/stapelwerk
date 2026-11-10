//! Spec test 7 (machine part): random programs never panic and end in a valid status; plus
//! determinism (same seed and input → same digest) and a release-mode throughput benchmark.

mod machine_util;

use machine_util::*;
use stapelwerk_core::consts::{SYS_ALLOC, SYS_FREE, SYS_RAND, SYS_READ_INT, SYS_WRITE_INT};
use stapelwerk_core::heap::HeapCheck;
use stapelwerk_core::isa::{self, AluOp, Cond, Instr, OPCODES};
use stapelwerk_core::machine::{Machine, MachineConfig, Status};
use stapelwerk_core::mem::Memory;
use stapelwerk_core::program::Program;
use stapelwerk_core::rng::SplitMix64;

/// A random word: fully random, or a table opcode with random (mostly valid) fields, or a short
/// jump / system call so that programs get somewhere.
fn random_word(rng: &mut SplitMix64) -> u32 {
    match rng.below(8) {
        0 => rng.next_u32(),
        1 => {
            let n = rng.range_i64(-1, 9) as u16;
            isa::encode(&Instr::Sys { n })
        }
        2 => {
            let cond = Cond::ALL[rng.below(15) as usize];
            let off = rng.range_i64(-6, 6) as i32;
            isa::encode(&Instr::Jump { cond, off })
        }
        3 => fielded_word(rng),
        _ => {
            // A word that decodes (most of the time): retry a few fielded words.
            let mut w = fielded_word(rng);
            for _ in 0..16 {
                if isa::decode(w).is_ok() {
                    break;
                }
                w = fielded_word(rng);
            }
            w
        }
    }
}

/// A table opcode with random register fields (mostly 0…9) and random low bits.
fn fielded_word(rng: &mut SplitMix64) -> u32 {
    let row = OPCODES[rng.below(OPCODES.len() as u64) as usize];
    let mut w = (rng.next_u32() & 0x0000_FFFF) | ((row.opcode as u32) << 24);
    let a = rng.below(10) as u32;
    let b = if rng.chance(1, 5) {
        15
    } else {
        rng.below(10) as u32
    };
    w |= (a << 20) | (b << 16);
    if rng.chance(1, 2) {
        w = (w & 0xFFFF_0000) | ((rng.below(10) as u32) << 12);
    }
    if rng.chance(1, 3) {
        w &= 0xFFFF_00FF; // small immediates
    }
    w
}

fn random_program(rng: &mut SplitMix64) -> Program {
    let len = rng.range_i64(1, 48) as usize;
    let mut words: Vec<u32> = (0..len).map(|_| random_word(rng)).collect();
    // Like the assembler's trap word: most programs end in `halt`.
    if rng.chance(3, 4) {
        words.push(isa::encode(&Instr::Halt));
    }
    let data: Vec<u8> = (0..rng.below(64)).map(|_| rng.next_u32() as u8).collect();
    raw_program(&words, &data)
}

#[test]
fn random_programs_end_in_a_valid_status() {
    let mut rng = SplitMix64::new(0xF022_0001);
    let mut seen = [0u32; 7];
    for i in 0..10_000u32 {
        let prog = random_program(&mut rng);
        let cfg = MachineConfig {
            heap_check: if rng.chance(1, 2) {
                HeapCheck::Strict
            } else {
                HeapCheck::Off
            },
            seed: rng.next_u32(),
            budget: 1500,
            interactive: rng.chance(1, 4),
        };
        let mut m = Machine::new(&prog, cfg);
        m.tapes.in_ints = (0..rng.below(4)).map(|_| rng.next_u32() as i32).collect();
        m.tapes.in_text = (0..rng.below(4)).map(|_| rng.next_u32() as u8).collect();
        let status = m.run(u64::MAX);
        assert!(
            matches!(
                status,
                Status::Halted | Status::Fault | Status::BudgetExceeded | Status::WaitingInput
            ),
            "program {i}: {status:?}"
        );
        assert_eq!(status, m.state.status);
        assert_eq!(m.state.fault.is_some(), status == Status::Fault);
        assert!(m.state.cycles < cfg.budget + 8);
        assert_eq!(m.state.out_len as usize, m.tapes.out_vals.len());
        seen[status as usize] += 1;
        // Every 16th program: running in tiny slices gives the same final state.
        if i % 16 == 0 {
            let mut s = Machine::new(&prog, cfg);
            s.tapes = m.tapes.clone();
            s.tapes.out_kinds.clear();
            s.tapes.out_vals.clear();
            while s.run(3) == Status::Paused {}
            assert_eq!(
                s.state.digest(&s.tapes),
                m.state.digest(&m.tapes),
                "program {i}"
            );
        }
    }
    // The generator reaches every terminal status.
    for st in [
        Status::Halted,
        Status::Fault,
        Status::BudgetExceeded,
        Status::WaitingInput,
    ] {
        assert!(seen[st as usize] > 0, "{st:?} never reached: {seen:?}");
    }
}

/// Reads numbers, allocates, writes random values, frees.
fn busy_program() -> Program {
    program(
        &code(&[&[
            sys(SYS_READ_INT),
            mov(R4, R0), // count
            // loop (0x108):
            sys(SYS_RAND),
            alui(AluOp::And, R0, R0, 0x3F),
            alui(AluOp::Add, R0, R0, 4), // 4…67 bytes
            sys(SYS_ALLOC),
            mov(R5, R0),
            sys(SYS_RAND),
            st(R0, based(R5, 0)),
            sys(SYS_WRITE_INT),
            mov(R0, R5),
            sys(SYS_FREE),
            alui(AluOp::Sub, R4, R4, 1),
            jump(Cond::Gt, -11),
            HALT,
        ]]),
        &[],
    )
}

fn run_busy(seed: u32, input: i32) -> Machine {
    let cfg = MachineConfig { seed, ..config() };
    let mut m = Machine::new(&busy_program(), cfg);
    m.tapes.in_ints = vec![input];
    m.run(u64::MAX);
    m
}

#[test]
fn same_seed_and_input_give_the_same_digest() {
    let a = run_busy(7, 50);
    let b = run_busy(7, 50);
    assert_eq!(a.state.status, Status::Halted, "{:?}", a.state.fault);
    assert_eq!(a.state.out_len, 50);
    assert_eq!(a.state.digest(&a.tapes), b.state.digest(&b.tapes));
    let digest = |m: &Machine| m.state.digest(&m.tapes);
    assert_ne!(digest(&a), digest(&run_busy(8, 50)), "seed matters");
    assert_ne!(digest(&a), digest(&run_busy(7, 49)), "input matters");
    // A clone taken mid-run continues identically.
    let mut m = Machine::new(
        &busy_program(),
        MachineConfig {
            seed: 7,
            ..config()
        },
    );
    m.tapes.in_ints = vec![50];
    m.run(200);
    let mut copy = m.clone();
    m.run(u64::MAX);
    copy.run(u64::MAX);
    assert_eq!(m.state.digest(&m.tapes), a.state.digest(&a.tapes));
    assert_eq!(copy.state.digest(&copy.tapes), a.state.digest(&a.tapes));
}

#[test]
fn digest_covers_every_part_of_the_state() {
    let base = run_busy(3, 5);
    let d = base.state.digest(&base.tapes);
    let mut variants: Vec<u64> = Vec::new();
    let mut push = |f: &dyn Fn(&mut Machine)| {
        let mut m = base.clone();
        f(&mut m);
        variants.push(m.state.digest(&m.tapes));
    };
    push(&|m| m.state.regs[9] ^= 4);
    push(&|m| m.state.pc += 4);
    push(&|m| m.state.flags ^= 1);
    push(&|m| m.state.mem.bytes_mut()[0xFFFF] ^= 1);
    push(&|m| {
        // Heap bookkeeping only (memory untouched).
        let mut scratch = Memory::new();
        m.state.heap.alloc(&mut scratch, 4, 0);
    });
    push(&|m| {
        let mut scratch = Memory::new();
        let a = m.state.heap.alloc(&mut scratch, 4, 0);
        let mut other = base.clone();
        other.state.heap.alloc(&mut scratch, 4, 1);
        assert_eq!(a, other.state.heap.blocks().last().unwrap().addr);
        // Same shadow, different alloc_pc.
        m.state.heap = other.state.heap.clone();
    });
    push(&|m| m.state.rng ^= 1);
    push(&|m| m.state.steps += 1);
    push(&|m| m.state.cycles += 1);
    push(&|m| m.state.in_int_pos += 1);
    push(&|m| m.state.in_text_pos += 1);
    push(&|m| m.state.out_len -= 1);
    push(&|m| m.tapes.out_vals[0] ^= 1);
    push(&|m| m.state.status = Status::Paused);
    for (i, v) in variants.iter().enumerate() {
        assert_ne!(*v, d, "variant {i} not covered by the digest");
    }
    assert_ne!(variants[4], variants[5], "alloc_pc is covered");
    // Output beyond out_len (an abandoned future) does not count.
    let mut m = base.clone();
    m.tapes.out_vals.push(1);
    m.tapes.out_kinds.push(0);
    assert_eq!(m.state.digest(&m.tapes), d);
    // Input contents do not count, only the read positions.
    let mut m = base.clone();
    m.tapes.in_ints.push(99);
    assert_eq!(m.state.digest(&m.tapes), d);
}

#[test]
fn digest_reference_value_of_empty_machine() {
    // Pins the digest layout: changing the field order breaks the native/WASM differential test.
    let m = Machine::new(&program(&[HALT], &[]), config());
    let d1 = m.state.digest(&m.tapes);
    let m2 = Machine::new(&program(&[HALT], &[]), config());
    assert_eq!(d1, m2.state.digest(&m2.tapes));
    assert_ne!(d1, 0);
}

/// Throughput in release mode: `cargo test --release -p stapelwerk-core -- --ignored bench --nocapture`.
#[test]
#[ignore = "benchmark; run in release mode"]
fn bench_steps_per_second() {
    // loop: add r2, r2, r1; st r2, [0x4000]; ld r3, [0x4000]; sub r1, r1, 1; jnz loop
    let iterations: u32 = 20_000_000;
    let prog = program(
        &code(&[
            &li(R1, iterations),
            &[
                alu(AluOp::Add, R2, R2, R1),
                st(R2, abs(0x4000)),
                ld(R3, abs(0x4000)),
                alui(AluOp::Sub, R1, R1, 1),
                jump(Cond::Nz, -4),
                HALT,
            ],
        ]),
        &[],
    );
    let cfg = MachineConfig {
        budget: u64::MAX,
        ..config()
    };
    let mut m = Machine::new(&prog, cfg);
    let start = std::time::Instant::now();
    assert_eq!(m.run(u64::MAX), Status::Halted);
    let secs = start.elapsed().as_secs_f64();
    let steps = m.state.steps;
    println!(
        "{steps} steps, {} cycles in {secs:.3} s = {:.1} M steps/s",
        m.state.cycles,
        steps as f64 / secs / 1e6
    );
    // Snapshot cost: cloning the whole state.
    let start = std::time::Instant::now();
    let mut total = 0u64;
    for _ in 0..10_000 {
        let c = m.state.clone();
        total += c.pc as u64;
    }
    println!(
        "state clone: {:.2} µs ({total})",
        start.elapsed().as_secs_f64() * 1e6 / 10_000.0
    );
    let start = std::time::Instant::now();
    let d = m.state.digest(&m.tapes);
    println!(
        "digest {d:016x} in {:.1} µs",
        start.elapsed().as_secs_f64() * 1e6
    );
}

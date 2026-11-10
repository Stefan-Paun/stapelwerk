//! System calls: input channels (test mode and interactive WAITING_INPUT), output tape and its
//! limit, WRITE_STR, RAND, unknown numbers, costs.

mod machine_util;

use machine_util::*;
use stapelwerk_core::consts::{
    OUT_KIND_BYTE, OUT_KIND_INT, SYS_RAND, SYS_READ_CHAR, SYS_READ_INT, SYS_WRITE_CHAR,
    SYS_WRITE_INT, SYS_WRITE_STR,
};
use stapelwerk_core::fault::FaultKind;
use stapelwerk_core::isa::{AluOp, FLAG_C, FLAG_Z};
use stapelwerk_core::machine::{Machine, MachineConfig, Status};
use stapelwerk_core::rng::xorshift32;

fn interactive() -> MachineConfig {
    MachineConfig {
        interactive: true,
        ..config()
    }
}

#[test]
fn read_int_in_test_mode() {
    let mut m = machine(&[
        sys(SYS_READ_INT),
        mov(R1, R0),
        sys(SYS_READ_INT),
        mov(R2, R0),
        sys(SYS_READ_INT),
        HALT,
    ]);
    m.tapes.in_ints = vec![-5, 7];
    assert_eq!(m.run(u64::MAX), Status::Fault);
    assert_eq!(m.state.reg(R1), (-5i32) as u32);
    assert_eq!(m.state.reg(R2), 7);
    assert_eq!(m.state.in_int_pos, 2);
    assert_eq!(fault_kind(&m), FaultKind::InputEnd);
    assert_eq!(m.state.pc, 0x110);
}

#[test]
fn read_char_in_test_mode_returns_minus_one_at_end() {
    let mut m = machine(&[
        sys(SYS_READ_CHAR),
        mov(R1, R0),
        sys(SYS_READ_CHAR),
        mov(R2, R0),
        sys(SYS_READ_CHAR),
        HALT,
    ]);
    m.tapes.in_text = vec![b'h', 0xFF];
    assert_eq!(m.run(u64::MAX), Status::Halted);
    assert_eq!(m.state.reg(R1), b'h' as u32);
    assert_eq!(m.state.reg(R2), 0xFF, "bytes are zero-extended");
    assert_eq!(m.state.reg(R0), 0xFFFF_FFFF);
    assert_eq!(m.state.in_text_pos, 2);
}

#[test]
fn interactive_input_waits_without_side_effects() {
    let prog = program(
        &[
            sys(SYS_READ_INT),
            sys(SYS_WRITE_INT),
            sys(SYS_READ_CHAR),
            mov(R1, R0),
            HALT,
        ],
        &[],
    );
    let mut m = Machine::new(&prog, interactive());
    assert!(!m.tapes.input_closed);
    let before = m.state.digest(&m.tapes);
    assert_eq!(m.run(u64::MAX), Status::WaitingInput);
    assert_eq!(m.state.status, Status::WaitingInput);
    assert_eq!((m.state.pc, m.state.steps, m.state.cycles), (0x100, 0, 0));
    assert_eq!(digest_without_status(&m.state, &m.tapes), before);
    // Single steps wait too.
    assert_eq!(m.step(), Status::WaitingInput);
    assert_eq!(m.state.steps, 0);
    // Input arrives: the same `sys` executes again.
    m.tapes.in_ints.push(42);
    assert_eq!(m.run(u64::MAX), Status::WaitingInput);
    assert_eq!(m.state.pc, 0x108);
    assert_eq!(m.state.steps, 2);
    assert_eq!(m.state.cycles, 8);
    assert_eq!(m.tapes.out_vals, vec![42]);
    // Closing the input lets READ_CHAR return -1.
    m.tapes.input_closed = true;
    assert_eq!(m.run(u64::MAX), Status::Halted);
    assert_eq!(m.state.reg(R1), 0xFFFF_FFFF);
}

#[test]
fn interactive_read_int_after_close_faults() {
    let mut m = Machine::new(&program(&[sys(SYS_READ_INT), HALT], &[]), interactive());
    assert_eq!(m.run(u64::MAX), Status::WaitingInput);
    m.tapes.input_closed = true;
    assert_eq!(m.run(u64::MAX), Status::Fault);
    assert_eq!(fault_kind(&m), FaultKind::InputEnd);
}

#[test]
fn write_int_and_char() {
    let m = run(&[
        movi(R0, -123),
        sys(SYS_WRITE_INT),
        movi(R0, 0x141),
        sys(SYS_WRITE_CHAR),
        HALT,
    ]);
    assert_eq!(m.state.out_len, 2);
    assert_eq!(m.tapes.out_kinds, vec![OUT_KIND_INT, OUT_KIND_BYTE]);
    assert_eq!(m.tapes.out_vals, vec![-123, 0x41]);
    assert_eq!(m.state.reg(R0), 0x141, "WRITE_* leave r0 alone");
}

#[test]
fn write_str() {
    let mut data = b"Hallo\0\0".to_vec();
    data.resize(0x1100, 0);
    // A string of exactly 4096 characters at 0x4010.
    for b in data.iter_mut().skip(0x10).take(4096) {
        *b = b'z';
    }
    let m = run_cfg(
        &[
            movi(R0, 0x4000),
            sys(SYS_WRITE_STR),
            movi(R0, 0x4005),
            sys(SYS_WRITE_STR), // empty string
            movi(R0, 0x4010),
            sys(SYS_WRITE_STR),
            HALT,
        ],
        &data,
        config(),
    );
    assert_eq!(m.state.status, Status::Halted, "{:?}", m.state.fault);
    assert_eq!(m.state.out_len, 5 + 4096);
    assert_eq!(&m.tapes.out_vals[..5], &[72, 97, 108, 108, 111]);
    assert!(m.tapes.out_kinds.iter().all(|&k| k == OUT_KIND_BYTE));
    assert_eq!(m.state.reg(R0), 0x4010);
    assert_eq!(m.state.cycles, 3 + 3 * 4 + 1, "sys costs 4 cycles");
}

#[test]
fn write_str_from_heap_and_text() {
    // Strings may live anywhere readable, including code.
    let m = run(&[movi(R0, 0x101), sys(SYS_WRITE_STR), HALT]);
    assert_eq!(m.state.status, Status::Halted);
    // Word at 0x100 is `mov r0, 0x101` = 0x11000101: bytes 01 01 00 11 → string from 0x101 is [0x01].
    assert_eq!(m.tapes.out_vals, vec![1]);
    // Null segment is not readable.
    assert_eq!(
        fault_of(&[movi(R0, 0x10), sys(SYS_WRITE_STR), HALT]),
        FaultKind::NullAccess {
            addr: 0x10,
            access: stapelwerk_core::fault::Access::Read
        }
    );
}

#[test]
fn output_limit_counts_whole_calls() {
    let mut m = Machine::new(
        &program(
            &[
                movi(R0, 0x4000),
                sys(SYS_WRITE_STR),
                movi(R0, 1),
                sys(SYS_WRITE_INT),
                sys(SYS_WRITE_INT),
                HALT,
            ],
            b"abc\0",
        ),
        config(),
    );
    // Pretend 8190 entries are already on the tape.
    for _ in 0..8190 {
        m.state.out_len = m.tapes.push_output(m.state.out_len, OUT_KIND_INT, 0);
    }
    assert_eq!(m.run(u64::MAX), Status::Fault);
    assert_eq!(fault_kind(&m), FaultKind::OutputLimit);
    assert_eq!(
        m.state.pc, 0x104,
        "WRITE_STR of 3 entries does not fit into 2"
    );
    assert_eq!(m.state.out_len, 8190);
    // Two single entries still fit; the third does not.
    let mut m = machine(&[
        movi(R0, 1),
        sys(SYS_WRITE_INT),
        sys(SYS_WRITE_INT),
        sys(SYS_WRITE_INT),
        HALT,
    ]);
    for _ in 0..8190 {
        m.state.out_len = m.tapes.push_output(m.state.out_len, OUT_KIND_INT, 0);
    }
    assert_eq!(m.run(u64::MAX), Status::Fault);
    assert_eq!(m.state.out_len, 8192);
    assert_eq!(m.state.pc, 0x10C);
}

#[test]
fn rand_is_xorshift32_from_the_seed() {
    let prog = program(
        &[sys(SYS_RAND), mov(R1, R0), sys(SYS_RAND), mov(R2, R0), HALT],
        &[],
    );
    let mut m = Machine::new(&prog, config());
    m.run(u64::MAX);
    let first = xorshift32(0x9E37_79B9);
    assert_eq!(m.state.reg(R1), first);
    assert_eq!(m.state.reg(R2), xorshift32(first));
    assert_eq!(m.state.rng, xorshift32(first));
    let mut m = Machine::new(
        &prog,
        MachineConfig {
            seed: 1,
            ..config()
        },
    );
    m.run(u64::MAX);
    assert_eq!(m.state.reg(R1), 0x0004_2021);
}

#[test]
fn syscalls_change_only_r0_and_keep_flags() {
    let m = run(&[
        movi(R1, -1),
        alui(AluOp::Add, R1, R1, 1), // Z C
        movi(R1, 11),
        movi(R2, 22),
        sys(SYS_RAND),
        sys(SYS_WRITE_INT),
        movi(R0, 16),
        sys(stapelwerk_core::consts::SYS_ALLOC),
        HALT,
    ]);
    assert_eq!(m.state.flags, FLAG_Z | FLAG_C);
    assert_eq!((m.state.reg(R1), m.state.reg(R2)), (11, 22));
    assert_eq!(m.state.reg(R0), 0x8000);
}

#[test]
fn unknown_syscalls() {
    for n in [0u16, 9, 99, 0xFFFF] {
        assert_eq!(
            fault_of(&[sys(n), HALT]),
            FaultKind::UnknownSyscall { n: n as u32 }
        );
    }
}

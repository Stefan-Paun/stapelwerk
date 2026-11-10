//! Spec test 9: every `fault.*` key in docs/ERRORS.md has a triggering program, and every fault
//! is precise (the state digest before and after the faulting step is equal except for status and
//! fault).

mod machine_util;

use machine_util::*;
use stapelwerk_core::consts::{SYS_READ_INT, SYS_WRITE_CHAR, SYS_WRITE_INT, SYS_WRITE_STR};
use stapelwerk_core::fault::{Access, FaultKind};
use stapelwerk_core::isa::{AluOp, Cond, Instr};
use stapelwerk_core::machine::{Machine, MachineConfig, Status};
use stapelwerk_core::program::Program;

const ERRORS_MD: &str = include_str!("../../../docs/ERRORS.md");

/// `fault.*` keys listed in ERRORS.md, in order.
fn documented_fault_keys() -> Vec<&'static str> {
    ERRORS_MD
        .lines()
        .filter_map(|l| l.strip_prefix("- `"))
        .filter_map(|l| l.split('`').next())
        .filter(|k| k.starts_with("fault."))
        .collect()
}

/// One triggering program per fault key: (program, config, expected fault).
fn triggers() -> Vec<(Program, MachineConfig, FaultKind)> {
    let p = |c: &[Instr]| program(c, &[]);
    let cfg = config();
    let long_string = vec![b'A'; 4097];
    let mut t = vec![
        (
            raw_program(&[0x0200_0000, 0x0300_0000], &[]),
            cfg,
            FaultKind::IllegalInstruction { word: 0x0300_0000 },
        ),
        (
            raw_program(&[0x0200_0000, 0], &[]),
            cfg,
            FaultKind::ZeroInstruction,
        ),
        (p(&[NOP, NOP]), cfg, FaultKind::ExecOutsideCode),
        (
            p(&[movi(R1, 0x4000), Instr::JumpReg { rs: R1 }]),
            cfg,
            FaultKind::BadJumpTarget { target: 0x4000 },
        ),
        (
            p(&[movi(R1, 0x6000), Instr::Push { rs: R1 }, RET]),
            cfg,
            FaultKind::BadReturnAddress { target: 0x6000 },
        ),
        (
            p(&[movi(R1, 7), st(R1, abs(0x0010))]),
            cfg,
            FaultKind::NullAccess {
                addr: 0x10,
                access: Access::Write,
            },
        ),
        (
            p(&code(&[&li(R1, 0x1_0000), &[ld(R2, based(R1, 4))]])),
            cfg,
            FaultKind::OutOfRange {
                addr: 0x1_0004,
                access: Access::Read,
            },
        ),
        (
            p(&[ld(R1, abs(0xC000))]),
            cfg,
            FaultKind::GuardPage {
                addr: 0xC000,
                access: Access::Read,
            },
        ),
        (
            p(&[
                movi(R1, 1),
                Instr::Stb {
                    rs: R1,
                    m: abs(0x0102),
                },
            ]),
            cfg,
            FaultKind::WriteToCode { addr: 0x0102 },
        ),
        (
            p(&[movi(R1, 0x4001), ld(R2, based(R1, 0))]),
            cfg,
            FaultKind::Misaligned {
                addr: 0x4001,
                size: 4,
            },
        ),
        (
            // Endless recursion: f: call f.
            p(&[Instr::Call { off: 0 }]),
            cfg,
            FaultKind::StackOverflow { sp: 0xD000 },
        ),
        (
            p(&[Instr::Pop { rd: R0 }]),
            cfg,
            FaultKind::StackUnderflow { sp: 0xF000 },
        ),
        (
            p(&[alui(AluOp::Sub, Sp, Sp, 2), Instr::Push { rs: R0 }]),
            cfg,
            FaultKind::StackMisaligned { sp: 0xEFFE },
        ),
        (
            p(&[movi(R1, 5), alui(AluOp::Rem, R2, R1, 0)]),
            cfg,
            FaultKind::DivZero,
        ),
        (p(&[sys(SYS_READ_INT)]), cfg, FaultKind::InputEnd),
        (
            p(&[ld(R1, abs(0x8000))]),
            cfg,
            FaultKind::HeapUnallocated { addr: 0x8000 },
        ),
        (
            p(&code(&[
                &alloc(5),
                &[
                    movi(R1, 1),
                    Instr::Stb {
                        rs: R1,
                        m: based(R0, 8),
                    },
                ],
            ])),
            cfg,
            FaultKind::HeapOverflow { addr: 0x8008 },
        ),
        (
            p(&code(&[
                &alloc(16),
                &[mov(R4, R0), free(), ld(R1, based(R4, 4))],
            ])),
            cfg,
            FaultKind::UseAfterFree { addr: 0x8004 },
        ),
        (
            p(&code(&[&alloc(16), &[alui(AluOp::Add, R0, R0, 8), free()]])),
            cfg,
            FaultKind::InvalidFree { addr: 0x8008 },
        ),
        (
            p(&code(&[
                &alloc(16),
                &[mov(R4, R0), free(), mov(R0, R4), free()],
            ])),
            cfg,
            FaultKind::DoubleFree { addr: 0x8000 },
        ),
        (p(&[sys(99)]), cfg, FaultKind::UnknownSyscall { n: 99 }),
        (
            program(&[movi(R0, 0x4000), sys(SYS_WRITE_STR)], &long_string),
            cfg,
            FaultKind::StringTooLong { addr: 0x4000 },
        ),
        (
            // 8193 × WRITE_CHAR: the last one faults.
            p(&[
                movi(R1, 8193),
                movi(R0, 65),
                sys(SYS_WRITE_CHAR),
                alui(AluOp::Sub, R1, R1, 1),
                jump(Cond::Nz, -2),
                HALT,
            ]),
            cfg,
            FaultKind::OutputLimit,
        ),
    ];
    // Generous budget so that no trigger is cut short (the output loop needs about 49000 cycles).
    for (_, c, _) in &mut t {
        c.budget = 1_000_000;
    }
    t
}

/// Runs to the fault, then replays to the step before it and checks that the faulting step
/// changed nothing but status and fault.
fn run_precisely(prog: &Program, cfg: MachineConfig) -> Machine {
    let mut first = Machine::new(prog, cfg);
    first.run(u64::MAX);
    let Some(fault) = first.state.fault else {
        return first;
    };
    let mut m = Machine::new(prog, cfg);
    while m.state.steps < fault.step {
        assert_eq!(m.step(), Status::Paused);
    }
    let before = m.state.digest(&m.tapes);
    let before_state = m.state.clone();
    let tapes_before = m.tapes.clone();
    assert_eq!(m.step(), Status::Fault);
    assert_eq!(
        digest_without_status(&m.state, &m.tapes),
        before,
        "fault {:?} changed the state",
        m.state.fault
    );
    let mut s = m.state.clone();
    s.status = before_state.status;
    s.fault = None;
    assert_eq!(s, before_state);
    assert_eq!(m.tapes, tapes_before);
    assert_eq!(m.state.fault, Some(fault));
    assert_eq!(fault.pc, m.state.pc);
    assert_eq!(fault.step, m.state.steps);
    m
}

#[test]
fn errors_md_lists_23_fault_keys() {
    let keys = documented_fault_keys();
    assert_eq!(keys.len(), FaultKind::COUNT as usize);
    assert_eq!(keys.first(), Some(&"fault.illegal_instruction"));
}

#[test]
fn every_documented_fault_key_has_a_trigger() {
    let triggers = triggers();
    let mut covered: Vec<&str> = Vec::new();
    for (prog, cfg, expected) in &triggers {
        let m = run_precisely(prog, *cfg);
        assert_eq!(m.state.status, Status::Fault, "expected {expected:?}");
        assert_eq!(m.state.fault.map(|f| f.kind), Some(*expected));
        covered.push(expected.key());
    }
    for key in documented_fault_keys() {
        assert!(covered.contains(&key), "no triggering test for {key}");
    }
    assert_eq!(
        covered,
        documented_fault_keys(),
        "trigger table in ERRORS.md order"
    );
}

#[test]
fn triggers_fault_the_same_way_interactively_with_closed_input() {
    for (prog, cfg, expected) in triggers() {
        let cfg = MachineConfig {
            interactive: true,
            ..cfg
        };
        let mut m = Machine::new(&prog, cfg);
        m.tapes.input_closed = true;
        m.run(u64::MAX);
        assert_eq!(m.state.fault.map(|f| f.kind), Some(expected));
    }
}

#[test]
fn memory_check_order() {
    // Range before alignment.
    assert_eq!(
        fault_of(&code(&[&li(R1, 0x1_0001), &[ld(R2, based(R1, 0))]])),
        FaultKind::OutOfRange {
            addr: 0x1_0001,
            access: Access::Read
        }
    );
    // Alignment before segment.
    assert_eq!(
        fault_of(&[ld(R2, abs(0x0002))]),
        FaultKind::Misaligned { addr: 2, size: 4 }
    );
    assert_eq!(
        fault_of(&[st(R2, abs(0xC002))]),
        FaultKind::Misaligned {
            addr: 0xC002,
            size: 4
        }
    );
    // Segment before heap: guard is not heap; heap misaligned word before shadow.
    assert_eq!(
        fault_of(&[st(R2, abs(0x8002))]),
        FaultKind::Misaligned {
            addr: 0x8002,
            size: 4
        }
    );
    // Byte accesses ignore alignment.
    assert_eq!(
        fault_of(&[Instr::Ldb {
            rd: R1,
            m: abs(0xC001)
        }]),
        FaultKind::GuardPage {
            addr: 0xC001,
            access: Access::Read
        }
    );
    // Writing code: word and byte; reading code is fine.
    assert_eq!(
        fault_of(&[st(R1, abs(0x3FFC))]),
        FaultKind::WriteToCode { addr: 0x3FFC }
    );
    // Null segment read.
    assert_eq!(
        fault_of(&[ld(R1, abs(0x00FC))]),
        FaultKind::NullAccess {
            addr: 0xFC,
            access: Access::Read
        }
    );
    // Effective address wraps modulo 2^32 before the range check.
    let m = run(&code(&[
        &li(R1, 0xFFFF_FFF0),
        &[ld(R2, based(R1, 0x4010)), HALT],
    ]));
    assert_eq!(m.state.status, Status::Halted);
}

#[test]
fn write_str_faults_are_checked_before_output() {
    // A string in a heap block without terminator runs into the red zone (strict heap check):
    // the whole call faults and nothing is written.
    let m = run(&code(&[
        &[movi(R0, 0x41), sys(SYS_WRITE_CHAR)],
        &alloc(4),
        &[sys(SYS_WRITE_STR)],
    ]));
    assert_eq!(fault_kind(&m), FaultKind::HeapOverflow { addr: 0x8004 });
    assert_eq!(m.state.out_len, 1, "only the WRITE_CHAR entry");
    assert_eq!(m.tapes.out_vals, vec![0x41]);
    // A non-terminated string at the end of the screen runs out of range.
    let m = run(&code(&[
        &[movi(R1, 0x41)],
        &li(R2, 0xFFFE),
        &[
            Instr::Stb {
                rs: R1,
                m: based(R2, 0),
            },
            Instr::Stb {
                rs: R1,
                m: based(R2, 1),
            },
            mov(R0, R2),
            sys(SYS_WRITE_INT),
            sys(SYS_WRITE_STR),
        ],
    ]));
    assert_eq!(
        fault_kind(&m),
        FaultKind::OutOfRange {
            addr: 0x1_0000,
            access: Access::Read
        }
    );
    assert_eq!(m.state.out_len, 1, "only the WRITE_INT entry");
}

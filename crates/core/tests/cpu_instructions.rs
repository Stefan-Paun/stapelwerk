//! Unit tests per instruction, with programs built by hand via `isa::encode`.

mod machine_util;

use machine_util::*;
use stapelwerk_core::fault::FaultKind;
use stapelwerk_core::isa::{AluOp, Cond, FLAG_C, FLAG_N, FLAG_V, FLAG_Z, Instr};
use stapelwerk_core::machine::{Breakpoints, Machine, MachineConfig, RunObserver, Status};

fn reg(m: &Machine, r: stapelwerk_core::isa::Reg) -> u32 {
    m.state.reg(r)
}

#[test]
fn halt_counts_one_step_and_cycle_and_keeps_pc() {
    let mut m = machine(&[HALT]);
    assert_eq!(m.step(), Status::Halted);
    assert_eq!((m.state.pc, m.state.steps, m.state.cycles), (0x100, 1, 1));
    // A halted machine does nothing more.
    assert_eq!(m.step(), Status::Halted);
    assert_eq!(m.run(100), Status::Halted);
    assert_eq!((m.state.steps, m.state.cycles), (1, 1));
}

#[test]
fn nop_advances_pc() {
    let mut m = machine(&[NOP, HALT]);
    assert_eq!(m.step(), Status::Paused);
    assert_eq!((m.state.pc, m.state.steps, m.state.cycles), (0x104, 1, 1));
}

#[test]
fn mov_forms() {
    let m = run(&code(&[
        &[
            movi(R1, -2),
            mov(R2, R1),
            movi(R3, 0x1234),
            movhi(R3, 0xABCD),
        ],
        &li(R4, 0x8000_FFFF),
        &li(R5, 0xFFFF_0000),
        &[HALT],
    ]));
    assert_eq!(reg(&m, R1), 0xFFFF_FFFE);
    assert_eq!(reg(&m, R2), 0xFFFF_FFFE);
    assert_eq!(reg(&m, R3), 0xABCD_1234);
    assert_eq!(reg(&m, R4), 0x8000_FFFF);
    assert_eq!(reg(&m, R5), 0xFFFF_0000);
    assert_eq!(m.state.flags, 0, "mov does not touch flags");
    assert_eq!(m.state.cycles, 9);
}

#[test]
fn lea_computes_without_range_check() {
    let m = run(&[
        movi(R1, 4),
        Instr::Lea {
            rd: R2,
            m: based(R1, -8),
        },
        Instr::Lea {
            rd: R3,
            m: abs(0xFFFF),
        },
        Instr::Lea {
            rd: R4,
            m: based(R1, 0x7FFF),
        },
        HALT,
    ]);
    assert_eq!(m.state.status, Status::Halted);
    assert_eq!(reg(&m, R2), 0xFFFF_FFFC);
    assert_eq!(reg(&m, R3), 0xFFFF);
    assert_eq!(reg(&m, R4), 0x8003);
}

#[test]
fn alu_register_and_immediate_forms() {
    let m = run(&[
        movi(R1, 7),
        movi(R2, 5),
        alu(AluOp::Add, R3, R1, R2),
        alu(AluOp::Sub, R4, R2, R1),
        alui(AluOp::Add, R5, R1, -1),
        alui(AluOp::And, R6, R4, 0xFFFF),
        alui(AluOp::Shl, R7, R1, 4),
        HALT,
    ]);
    assert_eq!(reg(&m, R3), 12);
    assert_eq!(reg(&m, R4), 0xFFFF_FFFE);
    assert_eq!(reg(&m, R5), 6);
    assert_eq!(reg(&m, R6), 0xFFFE, "u16 immediate is zero-extended");
    assert_eq!(reg(&m, R7), 112);
    assert_eq!(m.state.flags, 0, "last op shl 7,4 sets no flags");
}

#[test]
fn shift_width_from_register_is_masked() {
    let m = run(&[movi(R1, 1), movi(R2, 33), alu(AluOp::Shl, R3, R1, R2), HALT]);
    assert_eq!(reg(&m, R3), 2);
}

#[test]
fn cmp_and_tst_only_set_flags() {
    let m = run(&[movi(R0, 3), movi(R1, 5), alu(AluOp::Cmp, R0, R0, R1), HALT]);
    assert_eq!(reg(&m, R0), 3);
    assert_eq!(m.state.flags, FLAG_N | FLAG_C);
    let m = run(&[movi(R0, 0x10), alui(AluOp::Tst, R0, R0, 1), HALT]);
    assert_eq!(reg(&m, R0), 0x10);
    assert_eq!(m.state.flags, FLAG_Z);
}

#[test]
fn neg_and_not() {
    let m = run(&[movi(R1, 5), Instr::Neg { rd: R2, rs: R1 }, HALT]);
    assert_eq!(reg(&m, R2), 0xFFFF_FFFB);
    assert_eq!(m.state.flags, FLAG_N | FLAG_C, "0 - 5 borrows");
    let m = run(&[movi(R1, 0), Instr::Neg { rd: R2, rs: R1 }, HALT]);
    assert_eq!(m.state.flags, FLAG_Z);
    let m = run(&code(&[
        &li(R1, 0x8000_0000),
        &[Instr::Neg { rd: R2, rs: R1 }],
        &[HALT],
    ]));
    assert_eq!(reg(&m, R2), 0x8000_0000);
    assert_eq!(m.state.flags, FLAG_N | FLAG_C | FLAG_V);
    // `not` clears C and V left over from an earlier instruction.
    let m = run(&[
        movi(R1, -1),
        alui(AluOp::Add, R1, R1, 1), // Z C
        movi(R1, 0),
        Instr::Not { rd: R2, rs: R1 },
        HALT,
    ]);
    assert_eq!(reg(&m, R2), 0xFFFF_FFFF);
    assert_eq!(m.state.flags, FLAG_N);
}

#[test]
fn alu_cycle_costs() {
    for (op, cycles) in [
        (AluOp::Add, 1),
        (AluOp::Mul, 3),
        (AluOp::Div, 8),
        (AluOp::Divu, 8),
        (AluOp::Rem, 8),
        (AluOp::Remu, 8),
        (AluOp::Sar, 1),
    ] {
        let mut m = machine(&[movi(R1, 3), alu(op, R2, R1, R1), HALT]);
        m.step();
        m.step();
        assert_eq!(m.state.cycles, 1 + cycles, "{op:?}");
        let mut m = machine(&[movi(R1, 3), alui(op, R2, R1, 1), HALT]);
        m.step();
        m.step();
        assert_eq!(m.state.cycles, 1 + cycles, "{op:?} imm");
    }
}

#[test]
fn loads_and_stores() {
    let data = [0x78, 0x56, 0x34, 0x12, 0x80, 0x7F];
    let m = run_cfg(
        &code(&[
            &[
                ld(R1, abs(0x4000)),
                Instr::Ldb {
                    rd: R2,
                    m: abs(0x4004),
                },
                Instr::Ldbs {
                    rd: R3,
                    m: abs(0x4004),
                },
                Instr::Ldbs {
                    rd: R4,
                    m: abs(0x4005),
                },
            ],
            &li(R5, 0xAABB_CCDD),
            &[
                movi(R6, 0x6000),
                st(R5, based(R6, 8)),
                Instr::Stb {
                    rs: R5,
                    m: based(R6, 13),
                },
                ld(R7, based(R6, 12)),
                HALT,
            ],
        ]),
        &data,
        config(),
    );
    assert_eq!(m.state.status, Status::Halted);
    assert_eq!(reg(&m, R1), 0x1234_5678);
    assert_eq!(reg(&m, R2), 0x80);
    assert_eq!(reg(&m, R3), 0xFFFF_FF80);
    assert_eq!(reg(&m, R4), 0x7F);
    assert_eq!(
        &m.state.mem.bytes()[0x6008..0x600C],
        &[0xDD, 0xCC, 0xBB, 0xAA]
    );
    assert_eq!(reg(&m, R7), 0xDD00);
    // ld, ldb, ldbs, st, stb cost 2 cycles each.
    assert_eq!(m.state.cycles, 4 * 2 + 2 + 1 + 3 * 2 + 1);
}

#[test]
fn text_and_screen_are_readable_byte_access_anywhere() {
    let m = run(&[
        ld(R1, abs(0x0100)),
        Instr::Ldb {
            rd: R2,
            m: abs(0x0103),
        },
        movi(R3, 9),
        Instr::Stb {
            rs: R3,
            m: abs(0xFFFF),
        },
        Instr::Ldb {
            rd: R4,
            m: abs(0xFFFF),
        },
        HALT,
    ]);
    assert_eq!(m.state.status, Status::Halted);
    assert_eq!(
        reg(&m, R1),
        stapelwerk_core::isa::encode(&ld(R1, abs(0x0100)))
    );
    assert_eq!(reg(&m, R2), 0x50);
    assert_eq!(reg(&m, R4), 9);
    assert_eq!(m.state.mem.screen()[4095], 9);
}

#[test]
fn push_and_pop() {
    let m = run(&[
        movi(R1, 11),
        Instr::Push { rs: R1 },
        Instr::Pop { rd: R2 },
        HALT,
    ]);
    assert_eq!(reg(&m, R2), 11);
    assert_eq!(reg(&m, Sp), 0xF000);
    assert_eq!(m.state.mem.peek_u32(0xEFFC), 11);
    assert_eq!(m.state.cycles, 1 + 2 + 2 + 1);
}

#[test]
fn push_sp_stores_old_value_and_pop_sp_loads() {
    let m = run(&[Instr::Push { rs: Sp }, HALT]);
    assert_eq!(m.state.mem.peek_u32(0xEFFC), 0xF000);
    assert_eq!(reg(&m, Sp), 0xEFFC);
    // pop sp: sp = value popped (not value + 4).
    let m = run(&code(&[
        &li(R1, 0xE000),
        &[Instr::Push { rs: R1 }, Instr::Pop { rd: Sp }, HALT],
    ]));
    assert_eq!(reg(&m, Sp), 0xE000);
}

#[test]
fn stack_check_table() {
    let push_at = |sp: u32| {
        let m = run(&code(&[&li(Sp, sp), &[Instr::Push { rs: R0 }, HALT]]));
        (m.state.status, m.state.fault.map(|f| f.kind))
    };
    assert_eq!(push_at(0xD004), (Status::Halted, None));
    assert_eq!(
        push_at(0xD000).1,
        Some(FaultKind::StackOverflow { sp: 0xD000 })
    );
    assert_eq!(push_at(0).1, Some(FaultKind::StackOverflow { sp: 0 }));
    assert_eq!(
        push_at(0xF004).1,
        Some(FaultKind::StackUnderflow { sp: 0xF004 })
    );
    assert_eq!(
        push_at(0xEFFE).1,
        Some(FaultKind::StackMisaligned { sp: 0xEFFE })
    );
    let pop_at = |sp: u32| {
        let m = run(&code(&[&li(Sp, sp), &[Instr::Pop { rd: R0 }, HALT]]));
        (m.state.status, m.state.fault.map(|f| f.kind))
    };
    assert_eq!(pop_at(0xEFFC), (Status::Halted, None));
    assert_eq!(pop_at(0xD000), (Status::Halted, None));
    assert_eq!(
        pop_at(0xF000).1,
        Some(FaultKind::StackUnderflow { sp: 0xF000 })
    );
    assert_eq!(
        pop_at(0xCFFC).1,
        Some(FaultKind::StackOverflow { sp: 0xCFFC })
    );
    assert_eq!(
        pop_at(0xEFF1).1,
        Some(FaultKind::StackMisaligned { sp: 0xEFF1 })
    );
}

#[test]
fn ld_st_through_sp_follow_only_segment_rights() {
    // sp in the data segment: ordinary loads and stores work, push does not.
    let m = run(&code(&[
        &li(Sp, 0x4100),
        &[
            movi(R1, 5),
            st(R1, based(Sp, 0)),
            ld(R2, based(Sp, 0)),
            HALT,
        ],
    ]));
    assert_eq!(m.state.status, Status::Halted);
    assert_eq!(reg(&m, R2), 5);
}

#[test]
fn conditional_jumps_after_cmp() {
    // For each condition: cmp a, b; j<cc> +2; mov r1, 1 (skipped if taken); halt.
    #[rustfmt::skip]
    let cases: [(i16, i16, [bool; 15]); 4] = [
        //            jmp   jz     jnz    jlt    jge    jle    jgt    jb     jae    jbe    ja     js     jns    jo     jno
        (3, 5, [true, false, true, true, false, true, false, true, false, true, false, true, false, false, true]),
        (5, 5, [true, true, false, false, true, true, false, false, true, true, false, false, true, false, true]),
        (-1, 1, [true, false, true, true, false, true, false, false, true, false, true, true, false, false, true]),
        (5, -3, [true, false, true, false, true, false, true, true, false, true, false, false, true, false, true]),
    ];
    for (a, b, expect) in cases {
        for (i, cond) in Cond::ALL.iter().enumerate() {
            let mut m = machine(&[
                movi(R2, a),
                movi(R3, b),
                alu(AluOp::Cmp, R0, R2, R3),
                jump(*cond, 2),
                movi(R1, 1),
                HALT,
            ]);
            m.run(u64::MAX);
            let taken = reg(&m, R1) == 0;
            assert_eq!(taken, expect[i], "cmp {a}, {b}; {}", cond.mnemonic());
            let cycles = if taken { 5 } else { 6 };
            assert_eq!(m.state.cycles, cycles, "jumps cost 1 cycle taken or not");
        }
    }
    // jo after an overflowing add.
    let m = run(&code(&[
        &li(R1, 0x7FFF_FFFF),
        &[
            alui(AluOp::Add, R1, R1, 1),
            jump(Cond::O, 2),
            movi(R2, 1),
            HALT,
        ],
    ]));
    assert_eq!(reg(&m, R2), 0);
    assert_eq!(m.state.flags, FLAG_N | FLAG_V);
}

#[test]
fn backward_loop() {
    // r1 = 10; loop: add r2, r2, 3; sub r1, r1, 1; jnz loop; halt
    let m = run(&[
        movi(R1, 10),
        alui(AluOp::Add, R2, R2, 3),
        alui(AluOp::Sub, R1, R1, 1),
        jump(Cond::Nz, -2),
        HALT,
    ]);
    assert_eq!(reg(&m, R2), 30);
    assert_eq!(m.state.steps, 1 + 30 + 1);
    assert_eq!(m.state.cycles, 1 + 30 + 1);
}

#[test]
fn untaken_jump_does_not_check_its_target() {
    let m = run(&[movi(R0, 1), cmpi(R0, 0), jump(Cond::Z, 1000), HALT]);
    assert_eq!(m.state.status, Status::Halted);
}

#[test]
fn jump_targets_are_checked() {
    // Beyond the end of the code (still inside the text segment).
    assert_eq!(
        fault_of(&[jump(Cond::Always, 2), HALT]),
        FaultKind::BadJumpTarget { target: 0x108 }
    );
    // Before the text segment.
    assert_eq!(
        fault_of(&[jump(Cond::Always, -1), HALT]),
        FaultKind::BadJumpTarget { target: 0xFC }
    );
    // Misaligned register target, data address, huge address.
    assert_eq!(
        fault_of(&[movi(R1, 0x102), Instr::JumpReg { rs: R1 }, HALT]),
        FaultKind::BadJumpTarget { target: 0x102 }
    );
    assert_eq!(
        fault_of(&[movi(R1, 0x4000), Instr::JumpReg { rs: R1 }, HALT]),
        FaultKind::BadJumpTarget { target: 0x4000 }
    );
    assert_eq!(
        fault_of(&[movi(R1, -4), Instr::CallReg { rs: R1 }, HALT]),
        FaultKind::BadJumpTarget {
            target: 0xFFFF_FFFC
        }
    );
    // The fault is reported at the jumping instruction.
    let m = run(&[NOP, jump(Cond::Always, 5), HALT]);
    assert_eq!(m.state.fault.map(|f| (f.pc, f.step)), Some((0x104, 1)));
    assert_eq!(m.state.pc, 0x104);
}

#[test]
fn jmp_register() {
    let m = run(&[
        movi(R1, 0x10C),
        Instr::JumpReg { rs: R1 },
        movi(R2, 1),
        HALT,
    ]);
    assert_eq!(m.state.status, Status::Halted);
    assert_eq!(reg(&m, R2), 0);
    assert_eq!(m.state.pc, 0x10C);
}

#[test]
fn call_and_ret() {
    // 0x100 movi r1, 1
    // 0x104 call f (0x110)
    // 0x108 movi r2, 2
    // 0x10C halt
    // 0x110 f: movi r3, 3
    // 0x114 ret
    let mut m = machine(&[
        movi(R1, 1),
        Instr::Call { off: 3 },
        movi(R2, 2),
        HALT,
        movi(R3, 3),
        RET,
    ]);
    m.step();
    m.step();
    assert_eq!(m.state.pc, 0x110);
    assert_eq!(reg(&m, Sp), 0xEFFC);
    assert_eq!(m.state.mem.peek_u32(0xEFFC), 0x108);
    assert_eq!(m.state.cycles, 3);
    m.step();
    m.step();
    assert_eq!(m.state.pc, 0x108);
    assert_eq!(reg(&m, Sp), 0xF000);
    assert_eq!(m.state.cycles, 6);
    assert_eq!(m.run(u64::MAX), Status::Halted);
    assert_eq!((reg(&m, R1), reg(&m, R2), reg(&m, R3)), (1, 2, 3));
}

#[test]
fn call_register_and_frames() {
    // Prologue/epilogue as in the calling convention.
    // 0x100 movi r1, 0x114
    // 0x104 call r1
    // 0x108 halt
    // 0x10C nop
    // 0x110 nop
    // 0x114 push fp
    // 0x118 mov fp, sp
    // 0x11C sub sp, sp, 8
    // 0x120 movi r0, 42
    // 0x124 st r0, [fp - 4]
    // 0x128 mov sp, fp
    // 0x12C pop fp
    // 0x130 ret
    let m = run(&[
        movi(R1, 0x114),
        Instr::CallReg { rs: R1 },
        HALT,
        NOP,
        NOP,
        Instr::Push { rs: Fp },
        mov(Fp, Sp),
        alui(AluOp::Sub, Sp, Sp, 8),
        movi(R0, 42),
        st(R0, based(Fp, -4)),
        mov(Sp, Fp),
        Instr::Pop { rd: Fp },
        RET,
    ]);
    assert_eq!(m.state.status, Status::Halted);
    assert_eq!(m.state.pc, 0x108);
    assert_eq!(reg(&m, Sp), 0xF000);
    assert_eq!(reg(&m, Fp), 0);
    assert_eq!(m.state.mem.peek_u32(0xEFFC), 0x108, "return address");
    assert_eq!(m.state.mem.peek_u32(0xEFF4), 42, "local at [fp - 4]");
}

#[test]
fn call_checks_target_before_stack() {
    // Misaligned sp *and* bad target: the target is reported.
    let m = run(&[movi(Sp, 0x7002), Instr::Call { off: 100 }, HALT]);
    assert_eq!(
        fault_kind(&m),
        FaultKind::BadJumpTarget {
            target: 0x104 + 400
        }
    );
    // Good target, bad stack.
    assert_eq!(
        fault_of(&[movi(Sp, 0x7002), Instr::Call { off: 1 }, HALT]),
        FaultKind::StackMisaligned { sp: 0x7002 }
    );
}

#[test]
fn ret_checks_stack_before_target() {
    // Empty stack: underflow, even though [0xF000] (screen) holds garbage.
    assert_eq!(fault_of(&[RET]), FaultKind::StackUnderflow { sp: 0xF000 });
    // Data address on the stack.
    assert_eq!(
        fault_of(&[movi(R1, 0x4000), Instr::Push { rs: R1 }, RET]),
        FaultKind::BadReturnAddress { target: 0x4000 }
    );
    // Misaligned code address.
    assert_eq!(
        fault_of(&[movi(R1, 0x101), Instr::Push { rs: R1 }, RET]),
        FaultKind::BadReturnAddress { target: 0x101 }
    );
    // Precise: sp unchanged after the faulting ret.
    let m = run(&[movi(R1, 0x4000), Instr::Push { rs: R1 }, RET]);
    assert_eq!(m.state.reg(Sp), 0xEFFC);
    assert_eq!(m.state.pc, 0x108);
}

#[test]
fn faults_cost_nothing() {
    let m = run(&[NOP, alu(AluOp::Div, R1, R1, R2)]);
    assert_eq!(fault_kind(&m), FaultKind::DivZero);
    assert_eq!((m.state.steps, m.state.cycles), (1, 1));
    let f = m.state.fault.unwrap();
    assert_eq!((f.pc, f.step), (0x104, 1));
    assert_eq!(m.state.pc, 0x104);
}

#[test]
fn falling_off_the_code_and_zero_words() {
    assert_eq!(fault_of(&[NOP]), FaultKind::ExecOutsideCode);
    let mut m = Machine::new(&raw_program(&[0x0200_0000, 0], &[]), config());
    m.run(u64::MAX);
    assert_eq!(fault_kind(&m), FaultKind::ZeroInstruction);
    assert_eq!(m.state.pc, 0x104);
    let mut m = Machine::new(&raw_program(&[0x2F12_3000], &[]), config());
    m.run(u64::MAX);
    assert_eq!(
        fault_kind(&m),
        FaultKind::IllegalInstruction { word: 0x2F12_3000 }
    );
}

#[test]
fn budget_is_checked_before_each_instruction() {
    let cfg = MachineConfig {
        budget: 3,
        ..config()
    };
    // div (8 cycles) starts at 0 < 3 and completes; then 8 >= 3 stops the run.
    let mut m = Machine::new(
        &program(&[movi(R1, 1), alu(AluOp::Div, R2, R1, R1), HALT], &[]),
        cfg,
    );
    assert_eq!(m.run(u64::MAX), Status::BudgetExceeded);
    assert_eq!((m.state.steps, m.state.cycles), (2, 9));
    assert_eq!(m.state.pc, 0x108);
    assert_eq!(m.step(), Status::BudgetExceeded);
    assert_eq!(m.state.steps, 2);
    // Exactly at the budget stops too.
    let cfg = MachineConfig {
        budget: 2,
        ..config()
    };
    let mut m = Machine::new(&program(&[NOP, NOP, HALT], &[]), cfg);
    assert_eq!(m.run(u64::MAX), Status::BudgetExceeded);
    assert_eq!(m.state.cycles, 2);
    // Single steps respect the budget as well.
    let mut m = Machine::new(&program(&[NOP, NOP, HALT], &[]), cfg);
    assert_eq!(m.step(), Status::Paused);
    assert_eq!(m.step(), Status::Paused);
    assert_eq!(m.step(), Status::BudgetExceeded);
}

#[test]
fn time_slices_add_up_to_one_run() {
    let prog = program(
        &[
            movi(R1, 1000),
            alui(AluOp::Mul, R2, R1, 3),
            alui(AluOp::Sub, R1, R1, 1),
            jump(Cond::Nz, -2),
            HALT,
        ],
        &[],
    );
    let mut whole = Machine::new(&prog, config());
    assert_eq!(whole.run(u64::MAX), Status::Halted);
    let mut sliced = Machine::new(&prog, config());
    let mut slices = 0;
    loop {
        let before = sliced.state.cycles;
        let s = sliced.run(50);
        slices += 1;
        if s == Status::Paused {
            let used = sliced.state.cycles - before;
            assert!((50..50 + 3).contains(&used), "slice used {used}");
        } else {
            assert_eq!(s, Status::Halted);
            break;
        }
    }
    assert!(slices > 50);
    assert_eq!(sliced.state, whole.state);
    assert_eq!(
        sliced.state.digest(&sliced.tapes),
        whole.state.digest(&whole.tapes)
    );
}

#[test]
fn breakpoint_hook_stops_before_instruction_but_not_at_start() {
    let prog = program(&[NOP, NOP, NOP, HALT], &[]);
    let bps = [0x104, 0x108];
    let mut m = Machine::new(&prog, config());
    assert_eq!(
        m.run_observed(u64::MAX, &mut Breakpoints(&bps)),
        Status::Breakpoint
    );
    assert_eq!((m.state.pc, m.state.steps), (0x104, 1));
    // Resuming on a breakpoint executes it and stops at the next one.
    assert_eq!(
        m.run_observed(u64::MAX, &mut Breakpoints(&bps)),
        Status::Breakpoint
    );
    assert_eq!(m.state.pc, 0x108);
    assert_eq!(
        m.run_observed(u64::MAX, &mut Breakpoints(&bps)),
        Status::Halted
    );

    struct Counter(Vec<u32>);
    impl RunObserver for Counter {
        fn executed(&mut self, pc: u32) {
            self.0.push(pc);
        }
    }
    let mut c = Counter(Vec::new());
    let mut m = Machine::new(&prog, config());
    assert_eq!(m.run_observed(u64::MAX, &mut c), Status::Halted);
    assert_eq!(c.0, vec![0x100, 0x104, 0x108, 0x10C]);
}

#[test]
fn flags_survive_non_alu_instructions() {
    let m = run(&[
        movi(R1, -1),
        alui(AluOp::Add, R1, R1, 1), // Z C
        movi(R2, 7),
        Instr::Push { rs: R2 },
        Instr::Pop { rd: R3 },
        Instr::Lea { rd: R4, m: abs(4) },
        sys(stapelwerk_core::consts::SYS_RAND),
        NOP,
        HALT,
    ]);
    assert_eq!(m.state.flags, FLAG_Z | FLAG_C);
}

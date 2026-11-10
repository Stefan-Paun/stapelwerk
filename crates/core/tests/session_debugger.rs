//! Session (phase 2A): back_to_change, breakpoints, line counters, frames, heap blocks,
//! function tests, test setups, stable buffers.

use stapelwerk_core::consts::{OUT_KIND_INT, SCREEN_START, STACK_TOP};
use stapelwerk_core::fault::FaultKind;
use stapelwerk_core::isa::Reg;
use stapelwerk_core::level::{AddrRef, Image, Level, MemSpec, MemValues, TestCase};
use stapelwerk_core::machine::Status;
use stapelwerk_core::runner;
use stapelwerk_core::session::{
    ChangeKind, REG_FLAGS, REG_PC, Session, SessionConfig, SessionError, TestSetup, call_registers,
};

fn session(src: &str) -> Session {
    session_with(src, SessionConfig::default())
}

fn session_with(src: &str, cfg: SessionConfig) -> Session {
    let mut s = Session::new(cfg);
    if let Err(d) = s.assemble(src) {
        panic!("assembly failed: {d:#?}");
    }
    s
}

fn addr(s: &Session, name: &str) -> u32 {
    s.program()
        .and_then(|p| p.symbol(name))
        .unwrap_or_else(|| panic!("no symbol {name}"))
        .value as u32
}

fn ints(s: &Session) -> Vec<i32> {
    let (k, v) = s.output();
    k.iter()
        .zip(v)
        .filter(|(k, _)| **k == OUT_KIND_INT)
        .map(|(_, v)| *v)
        .collect()
}

#[test]
fn not_assembled() {
    let mut s = Session::sandbox();
    assert_eq!(s.status(), Status::NotAssembled);
    assert_eq!(s.step(1), Status::NotAssembled);
    assert_eq!(s.run(100), Status::NotAssembled);
    assert_eq!(s.goto(3), Status::NotAssembled);
    assert_eq!(s.reset(None), Ok(Status::NotAssembled));
    assert_eq!(s.view().status, 6);
    let err = s.assemble("main: frobnicate r1\n").unwrap_err();
    assert_eq!(err[0].key, "asm.unknown_mnemonic");
    assert_eq!(s.status(), Status::NotAssembled);
    assert!(s.assemble("main: halt\n").is_ok());
    assert_eq!(s.status(), Status::Paused);
    assert_eq!(s.step(5), Status::Halted);
    assert_eq!(s.steps(), 1);
}

// Steps: 0 mov r1 · 1 mov r2 · 2 mov r3 · 3 st · 4 mov r1 (same value) · 5 stb · 6 add · 7 nop
// · 8 halt.
const CHANGES: &str = "\
main:   mov r1, 5
        mov r2, 7
        mov r3, buf
        st r2, [r3]
        mov r1, 5
        stb r1, [r3 + 1]
        add r2, 1
        nop
        halt
.data
buf:    .word 0
";

#[test]
fn back_to_change_register_word_byte() {
    let mut s = session(CHANGES);
    let buf = addr(&s, "buf");
    assert_eq!(s.run(u64::MAX), Status::Halted);
    assert_eq!(s.steps(), 9);
    let end = s.digest();

    // r2: last change is `add` (step 6) → state 6.
    assert_eq!(
        s.back_to_change(ChangeKind::Register, 2),
        Ok(Status::Paused)
    );
    assert_eq!(s.steps(), 6);
    assert_eq!(s.state().reg(Reg::R2), 7);
    // Again from 6: the `mov r2, 7` at step 1.
    s.back_to_change(ChangeKind::Register, 2).unwrap();
    assert_eq!(s.steps(), 1);
    assert_eq!(s.state().reg(Reg::R2), 0);

    // r1: the second `mov r1, 5` does not change it → step 0.
    s.goto(9);
    assert_eq!(s.digest(), end);
    s.back_to_change(ChangeKind::Register, 1).unwrap();
    assert_eq!(s.steps(), 0);

    // Word at buf: `stb` to buf+1 (step 5) changes it last; before that `st` (step 3).
    s.goto(9);
    s.back_to_change(ChangeKind::Word, buf).unwrap();
    assert_eq!(s.steps(), 5);
    assert_eq!(s.state().mem.peek_u32(buf), 7);
    s.back_to_change(ChangeKind::Word, buf).unwrap();
    assert_eq!(s.steps(), 3);
    assert_eq!(s.state().mem.peek_u32(buf), 0);

    // Byte buf: only the `st` (step 3); byte buf+1 only the `stb` (step 5).
    s.goto(9);
    s.back_to_change(ChangeKind::Byte, buf).unwrap();
    assert_eq!(s.steps(), 3);
    s.goto(9);
    s.back_to_change(ChangeKind::Byte, buf + 1).unwrap();
    assert_eq!(s.steps(), 5);

    // Never changed: r5, byte buf+2 → step 0.
    s.goto(9);
    s.back_to_change(ChangeKind::Register, 5).unwrap();
    assert_eq!(s.steps(), 0);
    s.goto(9);
    s.back_to_change(ChangeKind::Byte, buf + 2).unwrap();
    assert_eq!(s.steps(), 0);

    // pc changes every step (halt keeps pc, so the last change is step 7).
    s.goto(9);
    s.back_to_change(ChangeKind::Register, REG_PC).unwrap();
    assert_eq!(s.steps(), 7);

    // Bad arguments.
    assert_eq!(
        s.back_to_change(ChangeKind::Word, buf + 2),
        Err(SessionError::BadArg)
    );
    assert_eq!(
        s.back_to_change(ChangeKind::Register, 12),
        Err(SessionError::BadArg)
    );
    assert_eq!(
        s.back_to_change(ChangeKind::Byte, 0x1_0000),
        Err(SessionError::BadArg)
    );
    assert_eq!(ChangeKind::from_code(1), Some(ChangeKind::Word));
    assert_eq!(ChangeKind::from_code(3), None);
}

#[test]
fn back_to_change_across_snapshots() {
    // r5 changes once early, r4 counts for a long time: the search crosses several segments.
    let src = "\
main:   mov r5, 9
        mov r4, 0
.loop:  add r4, 1
        cmp r4, 3000
        jlt .loop
        mov r6, 1
        halt
";
    let mut s = session(src);
    assert_eq!(s.run(u64::MAX), Status::Halted);
    let end = s.steps();
    assert_eq!(end, 2 + 3 * 3000 + 2);
    assert!(s.snapshot_count() >= 8);
    s.back_to_change(ChangeKind::Register, 5).unwrap();
    assert_eq!(s.steps(), 0);
    s.goto(end);
    s.back_to_change(ChangeKind::Register, 4).unwrap();
    // The last `add` is the third-last instruction before `mov r6` / `halt`… (jlt not taken).
    assert_eq!(s.steps(), end - 5);
    assert_eq!(s.state().reg(Reg::R4), 2999);
    // Flags change with the final cmp (3000 − 3000 = 0 sets Z).
    s.goto(end);
    s.back_to_change(ChangeKind::Register, REG_FLAGS).unwrap();
    assert_eq!(s.steps(), end - 4);
}

const LOOP: &str = "\
main:   mov r1, 0
loop:   add r1, 1
        cmp r1, 3
        jlt loop
        halt
";

#[test]
fn breakpoints() {
    let mut s = session(LOOP);
    let lp = addr(&s, "loop");
    assert!(s.set_breakpoint(lp, true));
    assert!(!s.set_breakpoint(lp + 2, true), "unaligned");
    assert!(!s.set_breakpoint(0x4000, true), "data");
    let trap = s.program().unwrap().trap_addr;
    assert!(!s.set_breakpoint(trap, true), "trap word");
    assert_eq!(s.breakpoints(), &[lp]);

    // Stops before the instruction, not after it.
    assert_eq!(s.run(u64::MAX), Status::Breakpoint);
    assert_eq!((s.state().pc, s.steps()), (lp, 1));
    assert_eq!(s.state().reg(Reg::R1), 0);
    // The first instruction of a run is never a stop.
    assert_eq!(s.run(u64::MAX), Status::Breakpoint);
    assert_eq!((s.steps(), s.state().reg(Reg::R1)), (4, 1));
    // step ignores breakpoints.
    assert_eq!(s.step(6), Status::Paused);
    assert_eq!(s.steps(), 10);
    assert_eq!(s.state().pc, lp + 12, "at halt");
    assert_eq!(s.run(u64::MAX), Status::Halted);

    // After stepping onto a breakpoint, run moves on.
    s.reset(None).unwrap();
    s.step(1);
    assert_eq!(s.state().pc, lp);
    assert_eq!(s.run(u64::MAX), Status::Breakpoint);
    assert_eq!(s.steps(), 4);

    // Time slices: a slice that ends right before the breakpoint reports it.
    s.reset(None).unwrap();
    assert_eq!(s.run(0), Status::Breakpoint);
    assert_eq!(s.steps(), 1);
    let mut stops = 0;
    loop {
        match s.run(1) {
            Status::Breakpoint => stops += 1,
            Status::Paused => {}
            Status::Halted => break,
            other => panic!("{other:?}"),
        }
    }
    assert_eq!(stops, 2);

    // Clearing.
    assert!(s.set_breakpoint(lp, false));
    assert!(s.breakpoints().is_empty());
    s.set_breakpoint(lp, true);
    s.clear_breakpoints();
    s.reset(None).unwrap();
    assert_eq!(s.run(u64::MAX), Status::Halted);
    // Breakpoints are cleared by a new assembly.
    s.set_breakpoint(lp, true);
    s.assemble(LOOP).unwrap();
    assert!(s.breakpoints().is_empty());
}

#[test]
fn line_counts_and_goto() {
    let mut s = session(LOOP);
    let mut seen = Vec::new();
    loop {
        seen.push((s.steps(), s.line_counts(), s.digest()));
        if s.step(1) == Status::Halted {
            break;
        }
    }
    seen.push((s.steps(), s.line_counts(), s.digest()));
    assert_eq!(
        s.line_counts(),
        vec![(1, 1), (2, 3), (3, 3), (4, 3), (5, 1)]
    );
    assert_eq!(s.hottest_line(), 2);
    for i in [5usize, 0, 9, 3, 11, 1] {
        let (k, counts, d) = &seen[i];
        s.goto(*k);
        assert_eq!(&s.line_counts(), counts, "line counts at {k}");
        assert_eq!(&s.digest(), d, "digest at {k}");
    }
    s.reset(None).unwrap();
    assert!(s.line_counts().is_empty());
    assert_eq!(s.hottest_line(), 0);
}

const FACT: &str = "\
main:   sys READ_INT
        call fak
        sys WRITE_INT
        halt

fak:    push fp
        mov fp, sp
        sub sp, 4
        st r0, [fp - 4]
        cmp r0, 1
        jgt .rec
        mov r0, 1          ; base case (fak + 24)
        jmp .out
.rec:   sub r0, 1
        call fak
        ld r1, [fp - 4]
        mul r0, r1
.out:   mov sp, fp
        pop fp
        ret
";

#[test]
fn frames_recursive_factorial() {
    let mut s = session(FACT);
    let setup = TestSetup {
        index: 2,
        input: vec![5],
        ..TestSetup::default()
    };
    assert_eq!(s.reset(Some(setup)), Ok(Status::Paused));
    assert_eq!(s.test_index(), 2);
    assert!(s.frames().is_empty(), "main has no frame");
    let base = addr(&s, "fak") + 24;
    s.set_breakpoint(base, true);
    assert_eq!(s.run(u64::MAX), Status::Breakpoint);
    let frames = s.frames();
    assert_eq!(frames.len(), 5, "fak(5) … fak(1): {frames:?}");
    assert!(frames.iter().all(|f| f.func.as_deref() == Some("fak")));
    assert!(frames.iter().all(|f| f.size == 12), "{frames:?}");
    assert_eq!(frames[0].fp, s.state().reg(Reg::Fp));
    assert!(frames.windows(2).all(|w| w[1].fp == w[0].fp + 12));
    assert_eq!(frames[4].fp, STACK_TOP - 8);
    // Inner frames return into fak, the outermost into main (after `call fak`).
    let main_ret = addr(&s, "main") + 8;
    assert_eq!(frames[4].ret, main_ret);
    assert!(frames[..4].iter().all(|f| f.ret != main_ret));
    s.clear_breakpoints();
    assert_eq!(s.run(u64::MAX), Status::Halted);
    assert_eq!(ints(&s), vec![120]);
    assert!(s.frames().is_empty());
}

#[test]
fn frame_names_without_known_call_target() {
    let src = "\
main:   mov r2, inner
        call r2
        halt
inner:  push fp
        mov fp, sp
        nop                ; inner + 8
        mov sp, fp
        pop fp
        ret
";
    let mut s = session(src);
    s.set_breakpoint(addr(&s, "inner") + 8, true);
    assert_eq!(s.run(u64::MAX), Status::Breakpoint);
    let f = s.frames();
    assert_eq!(f.len(), 1);
    assert_eq!(f[0].func.as_deref(), Some("inner"), "label at or below pc");
    assert_eq!(f[0].size, 8);
}

#[test]
fn heap_blocks_with_lines() {
    let lib = "\
lalloc: mov r0, 64
        sys ALLOC
        ret
";
    let src = "\
main:   mov r0, 12
        sys ALLOC
        mov r4, r0
        mov r0, 8
        sys ALLOC
        mov r0, r4
        sys FREE
        call lalloc
        halt
";
    let cfg = SessionConfig {
        library: Some(lib.into()),
        ..SessionConfig::default()
    };
    let mut s = session_with(src, cfg);
    assert_eq!(s.run(u64::MAX), Status::Halted);
    let blocks = s.heap_blocks();
    assert_eq!(blocks.len(), 3);
    assert_eq!(
        (blocks[0].size, blocks[0].live, blocks[0].line),
        (12, false, 2)
    );
    assert_eq!(
        (blocks[1].size, blocks[1].live, blocks[1].line),
        (8, true, 5)
    );
    assert_eq!((blocks[2].live, blocks[2].line), (true, 0), "library ALLOC");
    s.goto(5);
    let blocks = s.heap_blocks();
    assert_eq!(blocks.len(), 2);
    assert!(blocks.iter().all(|b| b.live));
    assert_eq!(s.shadow()[0], 1);
}

const FUNCS: &str = "\
main:   halt
good:   push r4
        mov r4, r0
        add r4, r1
        mov r0, r4
        pop r4
        ret
bad:    mov r4, r0
        add r4, r1
        mov r0, r4
        ret
";

fn call_setup(label: &str, seed: u32) -> TestSetup {
    TestSetup {
        regs: vec![(Reg::R0, 2), (Reg::R1, 3)],
        call: Some(label.into()),
        seed: Some(seed),
        ..TestSetup::default()
    }
}

#[test]
fn function_test_clean_and_clobbered() {
    let mut s = session(FUNCS);
    let trap = s.program().unwrap().trap_addr;

    s.reset(Some(call_setup("good", 7))).unwrap();
    let info = *s.call_info().unwrap();
    assert_eq!(info.trap, trap);
    assert_eq!(s.state().pc, addr(&s, "good"));
    assert_ne!(s.state().reg(Reg::Fp), 0);
    assert_eq!(s.state().reg(Reg::Sp), STACK_TOP - 4);
    assert_eq!(s.state().mem.peek_u32(STACK_TOP - 4), trap);
    assert_eq!(info.before[5], STACK_TOP);
    // Same random registers as the test runner.
    assert_eq!(info.before[..5], runner::function_test_regs(7));
    assert_eq!(info.before[..5], call_registers(7));
    assert_eq!(s.run(u64::MAX), Status::Halted);
    assert_eq!(s.state().pc, trap);
    assert_eq!(s.steps(), 6, "the trap's halt does not execute");
    assert_eq!(s.state().reg(Reg::R0), 5);
    assert_eq!(s.call_clobbered(), None);
    // Same seed → same random registers; another seed → others.
    let r4 = info.before[0];
    s.reset(Some(call_setup("good", 7))).unwrap();
    assert_eq!(s.state().reg(Reg::R4), r4);
    s.reset(Some(call_setup("good", 8))).unwrap();
    assert_ne!(s.state().reg(Reg::R4), r4);
    // Stepping onto the trap also ends the run.
    assert_eq!(s.step(100), Status::Halted);
    assert_eq!(s.steps(), 6);

    s.reset(Some(call_setup("bad", 7))).unwrap();
    assert_eq!(s.run(u64::MAX), Status::Halted);
    let c = s.call_clobbered().expect("r4 clobbered");
    assert_eq!(c.reg, Reg::R4);
    assert_eq!((c.before, c.after), (r4, 5));

    // Time travel inside a function test, then forward to the trap again.
    s.goto(2);
    assert_eq!(s.status(), Status::Paused);
    assert_eq!(s.run(u64::MAX), Status::Halted);
    assert_eq!(s.steps(), 4);

    assert_eq!(
        s.reset(Some(call_setup("nope", 1))),
        Err(SessionError::UnknownSymbol("nope".into()))
    );
    assert_eq!(s.steps(), 4, "failed reset changes nothing");
}

#[test]
fn test_setup_is_applied() {
    let src = "\
main:   sys READ_INT
        sys WRITE_INT
        sys READ_CHAR
        sys WRITE_INT
        sys READ_INT
        halt
.data
table:  .word 0, 0
";
    let mut s = session(src);
    let table = addr(&s, "table");
    let mut screen = vec![0u8; 4096];
    screen[5] = 14;
    let setup = TestSetup {
        index: 0,
        input: vec![42],
        in_text: b"A".to_vec(),
        regs: vec![(Reg::R7, 99)],
        sets: vec![
            MemSpec {
                addr: AddrRef::Symbol("table".into()),
                values: MemValues::Words(vec![0x1122_3344, 7]),
            },
            MemSpec {
                addr: AddrRef::Num(0x6000),
                values: MemValues::Bytes(b"hi".to_vec()),
            },
        ],
        screen: Some(screen),
        budget: Some(1234),
        ..TestSetup::default()
    };
    s.reset(Some(setup.clone())).unwrap();
    let mem = s.memory();
    assert_eq!(
        &mem[table as usize..table as usize + 4],
        &[0x44, 0x33, 0x22, 0x11]
    );
    assert_eq!(s.state().mem.peek_u32(table + 4), 7);
    assert_eq!(&mem[0x6000..0x6002], b"hi");
    assert_eq!(mem[SCREEN_START as usize + 5], 14);
    assert_eq!(s.state().reg(Reg::R7), 99);
    let v = s.view();
    assert_eq!(
        (v.test_index, v.budget, v.in_int_len, v.in_text_len),
        (0, 1234, 1, 1)
    );
    // Test mode: input is closed; pushing does nothing.
    assert_eq!(s.push_int(5), Status::Paused);
    assert_eq!(s.view().in_int_len, 1);
    assert_eq!(s.run(u64::MAX), Status::Fault);
    assert_eq!(ints(&s), vec![42, 65]);
    assert_eq!(s.fault().map(|f| f.kind), Some(FaultKind::InputEnd));
    let v = s.view();
    assert_eq!((v.status, v.fault_code, v.steps), (2, 15, 4));
    // Going back clears the fault; stepping again repeats it.
    s.back(1);
    assert_eq!(s.fault(), None);
    assert_eq!(s.step(2), Status::Fault);

    let bad = TestSetup {
        sets: vec![MemSpec {
            addr: AddrRef::Num(0xFFFF),
            values: MemValues::Words(vec![1]),
        }],
        ..setup
    };
    assert_eq!(
        s.reset(Some(bad.clone())),
        Err(SessionError::BadAddress(0xFFFF))
    );

    // `SYMBOL + n` resolves against the program; unknown or out-of-range expressions fail.
    let offset = TestSetup {
        sets: vec![MemSpec {
            addr: AddrRef::Offset("table".into(), 4),
            values: MemValues::Words(vec![9]),
        }],
        ..bad.clone()
    };
    s.reset(Some(offset)).unwrap();
    assert_eq!(s.state().mem.peek_u32(table + 4), 9);
    for (name, off) in [("nope", 4), ("table", -0x10000)] {
        let wrong = TestSetup {
            sets: vec![MemSpec {
                addr: AddrRef::Offset(name.into(), off),
                values: MemValues::Words(vec![9]),
            }],
            ..bad.clone()
        };
        let text = AddrRef::Offset(name.into(), off).text();
        assert_eq!(s.reset(Some(wrong)), Err(SessionError::UnknownSymbol(text)));
    }
}

#[test]
fn set_reg_logs_edits_and_discards_future() {
    let mut s = session(LOOP);
    let lp = addr(&s, "loop");
    s.step(4); // r1 = 1, back at loop
    assert_eq!(s.state().pc, lp);
    assert_eq!(s.set_reg(1, 2), Ok(Status::Paused));
    assert_eq!(s.run(u64::MAX), Status::Halted);
    assert_eq!(s.steps(), 8, "one iteration less");
    let end = s.digest();
    // Going back before the edit and forward again replays it.
    s.goto(2);
    assert_eq!(s.state().reg(Reg::R1), 1);
    s.goto(4);
    assert_eq!(s.state().reg(Reg::R1), 2);
    assert_eq!(s.run(u64::MAX), Status::Halted);
    assert_eq!(s.digest(), end);
    // A new edit before the old one discards it.
    s.goto(1);
    s.set_reg(1, 2).unwrap();
    assert_eq!(s.run(u64::MAX), Status::Halted);
    assert_eq!(s.steps(), 5);
    // Flags and pc.
    s.goto(1);
    s.set_reg(REG_FLAGS, 0xFF).unwrap();
    assert_eq!(s.state().flags, 0xF);
    s.set_reg(REG_PC, addr(&s, "main") + 16).unwrap();
    assert_eq!(s.step(1), Status::Halted);
    // Not while halted; bad index.
    assert_eq!(s.set_reg(1, 0), Err(SessionError::NotPaused));
    assert_eq!(s.set_reg(12, 0), Err(SessionError::BadArg));
}

#[test]
fn interactive_input() {
    let src = "\
main:   sys READ_INT
        sys WRITE_INT
        sys READ_CHAR
        sys WRITE_INT
        jmp main
";
    let mut s = session(src);
    assert_eq!(s.test_index(), -1);
    assert_eq!(s.run(u64::MAX), Status::WaitingInput);
    assert_eq!(s.steps(), 0);
    assert_eq!(s.push_int(3), Status::WaitingInput);
    assert_eq!(s.run(u64::MAX), Status::WaitingInput);
    assert_eq!(s.steps(), 2);
    s.push_text(b"xy");
    s.push_int(4);
    assert_eq!(s.run(u64::MAX), Status::WaitingInput);
    assert_eq!(ints(&s), vec![3, 120, 4, 121]);
    // Back before the reads and forward: same output, input stays.
    s.goto(2);
    assert_eq!(ints(&s), vec![3]);
    s.goto(1);
    assert!(ints(&s).is_empty());
    assert_eq!(s.run(u64::MAX), Status::WaitingInput);
    assert_eq!(ints(&s), vec![3, 120, 4, 121]);
    // Closed text input reads −1; closed number input faults.
    s.close_input();
    s.push_int(9);
    assert_eq!(s.view().in_int_len, 2, "closed input takes nothing");
    assert_eq!(s.run(u64::MAX), Status::Fault);
    assert_eq!(s.fault().map(|f| f.kind), Some(FaultKind::InputEnd));
}

#[test]
fn buffers_stay_put() {
    let mut s = session(FACT);
    let mem = s.memory().as_ptr();
    let shadow = s.shadow().as_ptr();
    let out = s.tapes().out_kinds.as_ptr();
    let vals = s.tapes().out_vals.as_ptr();
    s.push_int(6);
    s.run(u64::MAX);
    s.goto(3);
    s.reset(Some(TestSetup {
        input: vec![3],
        ..TestSetup::default()
    }))
    .unwrap();
    s.run(u64::MAX);
    s.back(5);
    s.assemble(LOOP).unwrap();
    s.run(u64::MAX);
    s.goto(0);
    assert_eq!(s.memory().as_ptr(), mem);
    assert_eq!(s.shadow().as_ptr(), shadow);
    assert_eq!(s.tapes().out_kinds.as_ptr(), out);
    assert_eq!(s.tapes().out_vals.as_ptr(), vals);
    assert!(s.tapes().out_kinds.capacity() >= 8192);
}

#[test]
fn queries() {
    let mut s = session(LOOP);
    let d = s.disasm(0x100, 3);
    assert_eq!(d.len(), 3);
    assert_eq!(d[0].text, "mov r1, 0");
    assert_eq!(d[0].label.as_deref(), Some("main"));
    assert_eq!(d[1].label.as_deref(), Some("loop"));
    assert!(s.symbols().iter().any(|x| x.name == "loop"));
    assert_eq!(s.linemap().len(), 5);
    assert_eq!(s.digest().len(), 16);
    let v = s.view();
    assert_eq!(
        (v.status, v.pc, v.regs[9], v.test_index),
        (0, 0x100, STACK_TOP, -1)
    );
    assert_eq!(v.budget, 10_000_000);
    s.run(u64::MAX);
    let v = s.view();
    assert_eq!((v.status, v.steps, v.cycles), (1, 11, 11));
}

#[test]
fn budget_and_hottest_line() {
    let src = "\
main:   mov r1, 0
spin:   add r1, 1
        jmp spin
";
    let cfg = SessionConfig {
        budget: 1000,
        ..SessionConfig::level_defaults()
    };
    let mut s = session_with(src, cfg);
    assert_eq!(s.run(u64::MAX), Status::BudgetExceeded);
    assert_eq!(s.state().cycles, 1000);
    assert_eq!(s.hottest_line(), 2);
    // Back and forward again ends the same way.
    let d = s.digest();
    s.back(10);
    assert_eq!(s.status(), Status::Paused);
    assert_eq!(s.run(u64::MAX), Status::BudgetExceeded);
    assert_eq!(s.digest(), d);
}

/// `TestSetup` as the level code would build it from a `[test]`.
fn setup_from(level: &Level, index: usize) -> TestSetup {
    let t = &level.tests[index];
    TestSetup {
        index: index as u32,
        input: t.input.clone(),
        in_text: t.in_text.clone(),
        regs: t.regs.clone(),
        sets: t.sets.clone(),
        screen: t
            .screen_init
            .as_deref()
            .and_then(|n| level.image(n))
            .map(|i| i.pixels.clone()),
        call: t.call.clone(),
        seed: t.seed,
        budget: t.budget,
    }
}

#[test]
fn reset_matches_the_test_runner() {
    let src = format!(
        "{FUNCS}\
sum:    ld r0, [table]
        ld r1, [table + 4]
        add r0, r1
        mov r2, r4
        ret
.data
table:  .word 0, 0
"
    );
    let program = stapelwerk_core::asm::assemble(&src, None, &Default::default()).unwrap();
    let mut pixels = vec![0u8; 4096];
    pixels[70] = 9;
    let level = Level {
        seed: 99,
        tests: vec![
            TestCase {
                call: Some("bad".into()),
                regs: vec![(Reg::R0, 4), (Reg::R1, 5)],
                ..TestCase::default()
            },
            TestCase {
                call: Some("sum".into()),
                sets: vec![MemSpec {
                    addr: AddrRef::Symbol("table".into()),
                    values: MemValues::Words(vec![30, 12]),
                }],
                screen_init: Some("img".into()),
                seed: Some(5),
                ..TestCase::default()
            },
            TestCase {
                input: vec![1, 2],
                regs: vec![(Reg::R3, 77)],
                budget: Some(50),
                ..TestCase::default()
            },
        ],
        images: vec![Image {
            name: "img".into(),
            pixels,
        }],
        ..Level::default()
    };
    let cfg = SessionConfig {
        seed: level.seed,
        budget: level.budget,
        heap_check: level.heap_check,
        ..SessionConfig::default()
    };
    let mut s = Session::new(cfg);
    s.load_program(program.clone());
    for i in 0..level.tests.len() {
        let mut p = runner::prepare_test(&level, &program, &level.tests[i]);
        s.reset(Some(setup_from(&level, i))).unwrap();
        assert_eq!(
            s.digest_u64(),
            p.machine.state.digest(&p.machine.tapes),
            "start of test {i}"
        );
        let status = runner::run_prepared(&mut p);
        assert_eq!(s.run(u64::MAX), status, "test {i}");
        assert_eq!(
            s.digest_u64(),
            p.machine.state.digest(&p.machine.tapes),
            "end of test {i}"
        );
    }
}

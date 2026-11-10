//! Heap manager: first fit, layout, red zone, shadow map, fill patterns, strict vs. off checks,
//! double/invalid free, leak detection.

mod machine_util;

use machine_util::*;
use stapelwerk_core::fault::{Access, FaultKind};
use stapelwerk_core::heap::{
    Block, Heap, HeapCheck, SHADOW_ALLOCATED, SHADOW_FREE, SHADOW_FREED, SHADOW_REDZONE,
};
use stapelwerk_core::isa::{AluOp, Instr};
use stapelwerk_core::machine::Status;
use stapelwerk_core::mem::Memory;

fn fresh() -> (Heap, Memory) {
    (Heap::new(), Memory::new())
}

fn shadow_range(h: &Heap, from: u32, to: u32) -> Vec<u8> {
    (from..to)
        .step_by(4)
        .map(|a| h.shadow_at(a).unwrap())
        .collect()
}

#[test]
fn layout_shadow_and_fill() {
    let (mut h, mut m) = fresh();
    assert_eq!(h.shadow().len(), 4096);
    let a = h.alloc(&mut m, 5, 0x104);
    assert_eq!(a, 0x8000);
    // round_up(5, 4) = 8 bytes allocated, span round_up(5, 8) + 8 = 16.
    assert_eq!(
        shadow_range(&h, 0x8000, 0x8014),
        vec![
            SHADOW_ALLOCATED,
            SHADOW_ALLOCATED,
            SHADOW_REDZONE,
            SHADOW_REDZONE,
            SHADOW_FREE
        ]
    );
    assert_eq!(&m.bytes()[0x8000..0x8008], &[0xCD; 8]);
    assert_eq!(
        &m.bytes()[0x8008..0x8010],
        &[0; 8],
        "red zone is not filled"
    );
    // A 12-byte block: 12 bytes allocated, round_up(12, 8) = 16 → 4 bytes padding + 8 red zone.
    let b = h.alloc(&mut m, 12, 0x10C);
    assert_eq!(b, 0x8010);
    assert_eq!(
        shadow_range(&h, 0x8010, 0x8028),
        vec![
            SHADOW_ALLOCATED,
            SHADOW_ALLOCATED,
            SHADOW_ALLOCATED,
            SHADOW_REDZONE,
            SHADOW_REDZONE,
            SHADOW_REDZONE
        ]
    );
    assert_eq!(h.alloc(&mut m, 1, 0), 0x8028);
    assert_eq!(h.live_count(), 3);
    assert_eq!(
        h.blocks()[1],
        Block {
            addr: 0x8010,
            size: 12,
            live: true,
            alloc_pc: 0x10C
        }
    );
    // FREE fills the whole span with 0xDD and marks it freed.
    h.free(&mut m, a).unwrap();
    assert_eq!(&m.bytes()[0x8000..0x8010], &[0xDD; 16]);
    assert_eq!(shadow_range(&h, 0x8000, 0x8010), vec![SHADOW_FREED; 4]);
    assert_eq!(h.live_count(), 2);
    assert!(!h.blocks()[0].live);
}

#[test]
fn freed_space_is_reused_first_fit() {
    let (mut h, mut m) = fresh();
    let a = h.alloc(&mut m, 8, 0); // span 16
    let b = h.alloc(&mut m, 8, 0);
    assert_eq!((a, b), (0x8000, 0x8010));
    h.free(&mut m, a).unwrap();
    // 16 bytes need a span of 24: the hole of 16 is too small.
    assert_eq!(h.alloc(&mut m, 16, 0), 0x8020);
    // 4 bytes (span 16) fit into the hole.
    assert_eq!(h.alloc(&mut m, 4, 0), 0x8000);
    assert_eq!(&m.bytes()[0x8000..0x8004], &[0xCD; 4]);
    // The freed block entry was replaced by the new block.
    assert_eq!(h.blocks().len(), 3);
    assert!(h.blocks().iter().all(|b| b.live));
    assert_eq!(h.live_count(), 3);
}

#[test]
fn freed_and_free_granules_merge_for_first_fit() {
    let (mut h, mut m) = fresh();
    let a = h.alloc(&mut m, 8, 0); // 0x8000, span 16
    h.free(&mut m, a).unwrap();
    // A span of 32 starts on freed granules and continues on never-used ones.
    assert_eq!(h.alloc(&mut m, 24, 0), 0x8000);
}

#[test]
fn heap_full_and_size_limits() {
    let (mut h, mut m) = fresh();
    assert_eq!(h.alloc(&mut m, 0, 0), 0);
    assert_eq!(h.alloc(&mut m, 0x4000, 0), 0, "span 0x4008 does not fit");
    assert_eq!(h.alloc(&mut m, u32::MAX, 0), 0);
    assert_eq!(h.alloc(&mut m, 0x3FF8, 0), 0x8000, "span exactly 16 KiB");
    assert_eq!(h.alloc(&mut m, 1, 0), 0);
    let (mut h, mut m) = fresh();
    let mut count = 0;
    while h.alloc(&mut m, 8, 0) != 0 {
        count += 1;
    }
    assert_eq!(count, 0x4000 / 16);
    let addrs: Vec<u32> = h.blocks().iter().map(|b| b.addr).collect();
    for a in addrs {
        h.free(&mut m, a).unwrap();
    }
    assert_eq!(h.live_count(), 0);
    assert!(h.shadow().iter().all(|&s| s == SHADOW_FREED));
    assert_eq!(h.alloc(&mut m, 0x3FF8, 0), 0x8000);
    assert_eq!(h.blocks().len(), 1, "all freed entries were replaced");
}

#[test]
fn free_errors() {
    let (mut h, mut m) = fresh();
    assert_eq!(h.free(&mut m, 0), Ok(()));
    let a = h.alloc(&mut m, 16, 0);
    assert_eq!(
        h.free(&mut m, a + 4),
        Err(FaultKind::InvalidFree { addr: a + 4 })
    );
    assert_eq!(
        h.free(&mut m, 0x4000),
        Err(FaultKind::InvalidFree { addr: 0x4000 })
    );
    let before = (h.clone(), m.clone());
    h.free(&mut m, a).unwrap();
    assert_ne!((h.clone(), m.clone()), before);
    let after = (h.clone(), m.clone());
    assert_eq!(h.free(&mut m, a), Err(FaultKind::DoubleFree { addr: a }));
    assert_eq!((h, m), after, "a failed FREE changes nothing");
}

#[test]
fn check_access_states() {
    let (mut h, mut m) = fresh();
    let a = h.alloc(&mut m, 8, 0);
    let b = h.alloc(&mut m, 8, 0);
    h.free(&mut m, b).unwrap();
    let s = HeapCheck::Strict;
    assert_eq!(h.check_access(a, 4, Access::Read, s), Ok(()));
    assert_eq!(h.check_access(a + 7, 1, Access::Write, s), Ok(()));
    assert_eq!(
        h.check_access(a + 8, 1, Access::Read, s),
        Err(FaultKind::HeapOverflow { addr: a + 8 })
    );
    assert_eq!(
        h.check_access(b + 4, 4, Access::Read, s),
        Err(FaultKind::UseAfterFree { addr: b + 4 })
    );
    assert_eq!(
        h.check_access(0xBFFC, 4, Access::Write, s),
        Err(FaultKind::HeapUnallocated { addr: 0xBFFC })
    );
    // Outside the heap and in off mode everything passes.
    assert_eq!(h.check_access(0x7FFF, 1, Access::Read, s), Ok(()));
    assert_eq!(h.check_access(0xC000, 4, Access::Read, s), Ok(()));
    assert_eq!(
        h.check_access(0xBFFC, 4, Access::Read, HeapCheck::Off),
        Ok(())
    );
}

/// Program: p = ALLOC(16); r4 = p; FREE(p); r1 = [r4]; halt.
fn use_after_free_program() -> Vec<Instr> {
    code(&[
        &alloc(16),
        &[mov(R4, R0), free(), ld(R1, based(R4, 0)), HALT],
    ])
}

#[test]
fn use_after_free_strict_vs_off() {
    let m = run(&use_after_free_program());
    assert_eq!(fault_kind(&m), FaultKind::UseAfterFree { addr: 0x8000 });
    let m = run_cfg(&use_after_free_program(), &[], config_off());
    assert_eq!(m.state.status, Status::Halted);
    assert_eq!(m.state.reg(R1), 0xDDDD_DDDD);
}

#[test]
fn unallocated_and_overflow_strict_vs_off() {
    let prog = [ld(R1, abs(0x9000)), HALT];
    assert_eq!(fault_of(&prog), FaultKind::HeapUnallocated { addr: 0x9000 });
    let m = run_cfg(&prog, &[], config_off());
    assert_eq!((m.state.status, m.state.reg(R1)), (Status::Halted, 0));

    // Writing one word past an 8-byte block hits the red zone.
    let prog = code(&[
        &alloc(8),
        &[
            movi(R1, 0x55),
            st(R1, based(R0, 4)),
            st(R1, based(R0, 8)),
            HALT,
        ],
    ]);
    assert_eq!(fault_of(&prog), FaultKind::HeapOverflow { addr: 0x8008 });
    let m = run_cfg(&prog, &[], config_off());
    assert_eq!(m.state.status, Status::Halted);
    assert_eq!(m.state.mem.peek_u32(0x8008), 0x55);
}

#[test]
fn free_faults_are_always_active() {
    for cfg in [config(), config_off()] {
        let m = run_cfg(
            &code(&[&alloc(8), &[alui(AluOp::Add, R0, R0, 4), free(), HALT]]),
            &[],
            cfg,
        );
        assert_eq!(fault_kind(&m), FaultKind::InvalidFree { addr: 0x8004 });
        let m = run_cfg(
            &code(&[&alloc(8), &[mov(R4, R0), free(), mov(R0, R4), free(), HALT]]),
            &[],
            cfg,
        );
        assert_eq!(fault_kind(&m), FaultKind::DoubleFree { addr: 0x8000 });
    }
}

#[test]
fn program_alloc_use_free_and_leak_detection() {
    // a = ALLOC(12); a[0] = 7; a[8] = 9; r5 = a[0] + a[8]; b = ALLOC(4); FREE(a); halt
    let prog = code(&[
        &alloc(12),
        &[
            mov(R4, R0),
            movi(R1, 7),
            st(R1, based(R4, 0)),
            movi(R1, 9),
            st(R1, based(R4, 8)),
            ld(R2, based(R4, 0)),
            ld(R3, based(R4, 8)),
            alu(AluOp::Add, R5, R2, R3),
        ],
        &alloc(4),
        &[mov(R6, R0), mov(R0, R4), free(), HALT],
    ]);
    let m = run(&prog);
    assert_eq!(m.state.status, Status::Halted, "{:?}", m.state.fault);
    assert_eq!(m.state.reg(R5), 16);
    assert_eq!(m.state.reg(R6), 0x8018);
    assert_eq!(m.state.heap.live_count(), 1, "the 4-byte block leaks");
    let leaked: Vec<_> = m.state.heap.blocks().iter().filter(|b| b.live).collect();
    assert_eq!(leaked.len(), 1);
    // alloc_pc is the address of the second `sys ALLOC` (word index 11).
    assert_eq!(leaked[0].alloc_pc, 0x100 + 11 * 4);
    assert_eq!(leaked[0].size, 4);

    // Freeing both leaves the heap empty.
    let prog = code(&[&alloc(12), &[mov(R4, R0), free(), HALT]]);
    let m = run(&prog);
    assert_eq!(m.state.heap.live_count(), 0);
    // ALLOC(0) returns 0 and FREE(0) is a no-op.
    let m = run(&code(&[&alloc(0), &[mov(R1, R0), free(), HALT]]));
    assert_eq!(m.state.status, Status::Halted);
    assert_eq!(m.state.reg(R1), 0);
    assert!(m.state.heap.blocks().is_empty());
}

#[test]
fn alloc_treats_r0_as_unsigned() {
    let m = run(&[movi(R0, -1), sys(stapelwerk_core::consts::SYS_ALLOC), HALT]);
    assert_eq!(m.state.reg(R0), 0);
    assert_eq!(m.state.heap.live_count(), 0);
}

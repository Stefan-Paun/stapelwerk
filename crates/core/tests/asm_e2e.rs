//! End to end: assemble small programs and run them on the machine from phase 1A.

use stapelwerk_core::asm::{MnemonicFilter, assemble};
use stapelwerk_core::consts::{OUT_KIND_BYTE, OUT_KIND_INT, SCREEN_W, STACK_TOP};
use stapelwerk_core::fault::FaultKind;
use stapelwerk_core::isa::Reg;
use stapelwerk_core::machine::{Machine, MachineConfig, Status};
use stapelwerk_core::program::Program;

fn asm(src: &str, lib: Option<&str>) -> Program {
    match assemble(src, lib, &MnemonicFilter::All) {
        Ok(p) => p,
        Err(d) => panic!("assembly failed: {d:#?}"),
    }
}

fn run(p: &Program, input: &[i32]) -> Machine {
    let mut m = Machine::new(p, MachineConfig::default());
    m.tapes.in_ints = input.to_vec();
    m.run(u64::MAX);
    m
}

fn ints(m: &Machine) -> Vec<i32> {
    let n = m.state.out_len as usize;
    m.tapes.out_kinds[..n]
        .iter()
        .zip(&m.tapes.out_vals[..n])
        .filter(|(k, _)| **k == OUT_KIND_INT)
        .map(|(_, v)| *v)
        .collect()
}

fn text(m: &Machine) -> Vec<u8> {
    let n = m.state.out_len as usize;
    m.tapes.out_kinds[..n]
        .iter()
        .zip(&m.tapes.out_vals[..n])
        .filter(|(k, _)| **k == OUT_KIND_BYTE)
        .map(|(_, v)| *v as u8)
        .collect()
}

fn halted(m: &Machine) {
    assert_eq!(m.state.status, Status::Halted, "fault: {:?}", m.state.fault);
}

const SUM: &str = "\
; Summe aller Zahlen bis zur 0
main:   mov r4, 0
.loop:  sys READ_INT
        cmp r0, 0
        jz .done
        add r4, r0          ; Kurzform: add r4, r4, r0
        jmp .loop
.done:  mov r0, r4
        sys WRITE_INT
        halt
";

#[test]
fn sum_until_zero() {
    let p = asm(SUM, None);
    let m = run(&p, &[3, 4, 5, -2, 0, 99]);
    halted(&m);
    assert_eq!(ints(&m), vec![10]);
    assert_eq!(p.size, 9);
}

const FACT: &str = "\
main:   sys READ_INT
        call fact
        sys WRITE_INT
        halt

; fact(n) = n <= 1 ? 1 : n * fact(n - 1), with prologue and epilogue
fact:   push fp
        mov fp, sp
        sub sp, 4
        st r0, [fp - 4]
        cmp r0, 1
        jgt .rec
        mov r0, 1
        jmp .out
.rec:   sub r0, 1
        call fact
        ld r1, [fp - 4]
        mul r0, r1
.out:   mov sp, fp
        pop fp
        ret
";

#[test]
fn factorial_recursive() {
    let p = asm(FACT, None);
    for (n, f) in [(0, 1), (1, 1), (5, 120), (10, 3_628_800), (12, 479_001_600)] {
        let m = run(&p, &[n]);
        halted(&m);
        assert_eq!(ints(&m), vec![f], "fact({n})");
        assert_eq!(m.state.reg(Reg::Sp), STACK_TOP);
        assert_eq!(m.state.reg(Reg::Fp), 0);
    }
}

#[test]
fn pixels() {
    let src = "\
.equ X, 10
.equ Y, 20
main:   li r1, SCREEN + Y * SCREEN_W + X
        mov r2, YELLOW
        stb r2, [r1]
        mov r3, 0
.row:   li r4, SCREEN
        add r4, r3
        mov r5, RED
        stb r5, [r4]
        add r3, 1
        cmp r3, SCREEN_W
        jlt .row
        li r1, SCREEN + 4095
        mov r2, WHITE
        stb r2, [r1]
        halt
";
    let p = asm(src, None);
    let m = run(&p, &[]);
    halted(&m);
    let screen = m.state.mem.screen();
    assert_eq!(screen[(20 * SCREEN_W + 10) as usize], 14);
    assert!(screen[..64].iter().all(|&c| c == 4));
    assert_eq!(screen[4095], 15);
    assert_eq!(screen.iter().filter(|&&c| c != 0).count(), 66);
}

#[test]
fn heap_list() {
    let src = "\
; Liste 5 -> 4 -> 3 -> 2 -> 1 auf dem Heap, ausgeben, freigeben
main:   mov r4, 0           ; head
        mov r5, 1
.build: mov r0, 8
        sys ALLOC
        st r5, [r0]         ; value
        st r4, [r0 + 4]     ; next
        mov r4, r0
        add r5, 1
        cmp r5, 6
        jlt .build
        mov r6, r4
.print: cmp r6, 0
        jz .free
        ld r0, [r6]
        sys WRITE_INT
        ld r6, [r6 + 4]
        jmp .print
.free:  cmp r4, 0
        jz .end
        ld r6, [r4 + 4]
        mov r0, r4
        sys FREE
        mov r4, r6
        jmp .free
.end:   halt
";
    let p = asm(src, None);
    let m = run(&p, &[]);
    halted(&m);
    assert_eq!(ints(&m), vec![5, 4, 3, 2, 1]);
    assert_eq!(m.state.heap.live_count(), 0);
}

#[test]
fn strings_and_data() {
    let src = "\
main:   li r0, msg
        sys WRITE_STR
        ld r0, [table + 8]
        sys WRITE_INT
        ldbs r0, [minus]
        sys WRITE_INT
        halt
.data
msg:    .string \"Grüß dich\\n\"
table:  .word 10, 20, 30
minus:  .byte -5
";
    let p = asm(src, None);
    let m = run(&p, &[]);
    halted(&m);
    assert_eq!(text(&m), "Grüß dich\n".as_bytes());
    assert_eq!(ints(&m), vec![30, -5]);
}

#[test]
fn missing_halt_gives_zero_instruction() {
    for lib in [None, Some("helper: mov r0, 1\n        ret\n")] {
        let p = asm("main:   mov r1, 1\n        add r1, 2\n", lib);
        // The zero word right after the player's code, even with a library behind it.
        assert_eq!(p.words[2], 0);
        let m = run(&p, &[]);
        assert_eq!(m.state.status, Status::Fault);
        let f = m.state.fault.expect("fault");
        assert_eq!(f.kind, FaultKind::ZeroInstruction);
        assert_eq!(m.state.pc, 0x108);
        assert_eq!(m.state.reg(Reg::R1), 3);
    }
}

#[test]
fn library_functions_and_constants() {
    let lib = "\
; Bibliothek
square: mul r0, r0
        ret
.equ BASE, 50
.equ LIMIT, BASE * 2
";
    let src = "\
main:   mov r0, 7
        call square
        sys WRITE_INT
        mov r0, LIMIT
        sys WRITE_INT
        halt
";
    let p = asm(src, Some(lib));
    let m = run(&p, &[]);
    halted(&m);
    assert_eq!(ints(&m), vec![49, 100]);
    // A function test calls into the library and returns to the trap word.
    let trap = p.symbol("__trap").expect("trap").value as u32;
    assert_eq!(trap, p.trap_addr);
    let mut m = Machine::new(&p, MachineConfig::default());
    let square = p.symbol("square").unwrap().value as u32;
    m.state.set_reg(Reg::R0, 9);
    let sp = m.state.reg(Reg::Sp) - 4;
    m.state.mem.bytes_mut()[sp as usize..sp as usize + 4].copy_from_slice(&trap.to_le_bytes());
    m.state.set_reg(Reg::Sp, sp);
    m.state.pc = square;
    m.run(u64::MAX);
    halted(&m);
    assert_eq!(m.state.pc, trap);
    assert_eq!(m.state.reg(Reg::R0), 81);
}

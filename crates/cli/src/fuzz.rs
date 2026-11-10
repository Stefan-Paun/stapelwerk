//! `fuzz-gen`: random programs for the differential test native vs. WebAssembly.
//!
//! The programs are built to run long and to touch most of the machine, so the digests compare a
//! lot of state:
//!
//! - an outer loop (counter `r6`, 10–120 rounds) whose body mixes random register instructions
//!   (decoded from random words and printed by the disassembler, so every encoding form occurs),
//!   memory accesses, forward conditional and register jumps, balanced `push`/`pop`, inner loops
//!   (counter `r7`), system calls (`WRITE_INT`, `WRITE_CHAR`, `WRITE_STR`, `READ_CHAR`, `RAND`),
//!   `ALLOC`/`FREE` pairs and calls (direct and `call rs`),
//! - up to four subroutines with prologue (`push fp; mov fp, sp; sub sp, sp, 16`), locals at
//!   `[fp - 4]`…`[fp - 16]` and epilogue (`mov sp, fp; pop fp; ret`); a subroutine only calls
//!   subroutines with a higher number (no recursion).
//!
//! Random instructions only write `r0`–`r5`; `r6`, `r7`, `fp` and `sp` are reserved for the loop
//! counters and the stack. Memory accesses only use aligned addresses in data (from 0x4100; the
//! string for `WRITE_STR` lies at 0x4000), the unused bottom of the stack segment (0xD000–0xDFFF),
//! the screen, the locals of the current frame or the allocated part of a fresh heap block. A
//! divisor register is made odd (`or rs, rs, 1`) right before `div`/`divu`/`rem`/`remu`.
//!
//! About one program in eight faults on purpose: in one round of the outer loop it executes a
//! snippet that triggers a fault (null access, division by zero, write to code, misalignment,
//! guard page, out of range, heap errors, bad jump, stack underflow, unknown system call). Long
//! loops can also run out of the cycle budget.
//!
//! Each program is run natively in test mode (closed, empty input); `digests.txt` records the
//! state digest every 1000 steps and at the end.

use stapelwerk_core::asm::{MnemonicFilter, assemble};
use stapelwerk_core::disasm::format_instr;
use stapelwerk_core::isa::{AluOp, Cond, Instr, Reg, decode};
use stapelwerk_core::machine::{Machine, MachineConfig, Status};
use stapelwerk_core::rng::SplitMix64;
use std::fmt::Write as _;
use std::path::Path;

/// Cycle budget per program.
pub const FUZZ_BUDGET: u64 = 100_000;
/// Digest interval in steps.
pub const DIGEST_EVERY: u64 = 1000;

/// Registers random instructions may write.
const WORK: [&str; 6] = ["r0", "r1", "r2", "r3", "r4", "r5"];
/// One in this many programs faults on purpose.
const FAULT_ONE_IN: u64 = 8;

fn is_work(r: Reg) -> bool {
    r.code() < 6
}

fn is_division(op: AluOp) -> bool {
    matches!(op, AluOp::Div | AluOp::Divu | AluOp::Rem | AluOp::Remu)
}

/// Program text builder.
struct Gen<'a> {
    rng: &'a mut SplitMix64,
    out: String,
    labels: u32,
    /// Number of subroutines (`f0`…).
    funcs: u64,
}

impl Gen<'_> {
    fn line(&mut self, text: &str) {
        let _ = writeln!(self.out, "        {text}");
    }

    fn label(&mut self, name: &str) {
        let _ = writeln!(self.out, "{name}:");
    }

    fn fresh(&mut self, prefix: &str) -> String {
        self.labels += 1;
        format!("{prefix}{}", self.labels)
    }

    fn work(&mut self) -> &'static str {
        WORK[self.rng.below(WORK.len() as u64) as usize]
    }

    fn any_reg(&mut self) -> &'static str {
        const ALL: [&str; 10] = ["r0", "r1", "r2", "r3", "r4", "r5", "r6", "r7", "fp", "sp"];
        ALL[self.rng.below(10) as usize]
    }

    /// One random register instruction that writes only `r0`–`r5` (or only flags).
    fn alu(&mut self) {
        loop {
            let Ok(instr) = decode(self.rng.next_u32()) else {
                continue;
            };
            let ok = match instr {
                Instr::Alu { op, rd, rs2, .. } => {
                    if is_division(op) {
                        if !is_work(rs2) {
                            continue;
                        }
                        let r = WORK[rs2.code() as usize];
                        self.line(&format!("or {r}, {r}, 1"));
                    }
                    !op.writes_rd() || is_work(rd)
                }
                Instr::AluI { op, rd, imm, .. } => {
                    !(is_division(op) && imm == 0) && (!op.writes_rd() || is_work(rd))
                }
                Instr::Mov { rd, .. }
                | Instr::MovI { rd, .. }
                | Instr::MovHi { rd, .. }
                | Instr::Lea { rd, .. }
                | Instr::Neg { rd, .. }
                | Instr::Not { rd, .. } => is_work(rd),
                Instr::Nop => self.rng.chance(1, 4),
                _ => false,
            };
            if ok {
                let text = format_instr(&instr, 0x100);
                self.line(&text);
                return;
            }
        }
    }

    /// A load or store at a valid address (absolute or `[base ± off]`); `frame` allows locals.
    fn mem(&mut self, frame: bool) {
        let word = self.rng.chance(1, 2);
        let mnemonic = match (word, self.rng.chance(1, 2)) {
            (true, true) => "ld",
            (true, false) => "st",
            (false, true) => {
                if self.rng.chance(1, 2) {
                    "ldb"
                } else {
                    "ldbs"
                }
            }
            (false, false) => "stb",
        };
        let store = mnemonic.starts_with("st");
        let value = if store { self.any_reg() } else { self.work() };
        let step = if word { 4 } else { 1 };
        let operand = if frame && self.rng.chance(1, 3) {
            format!("[fp - {}]", 4 * (1 + self.rng.below(4)))
        } else {
            let (lo, hi) = match self.rng.below(3) {
                0 => (0x4100u64, 0x5FFF),
                1 => (0xD000, 0xDFFF),
                _ => (0xF000, 0xFFFF),
            };
            let addr = (lo + self.rng.below(hi - lo + 1 - 64) + 32) / step * step;
            if self.rng.chance(1, 2) {
                format!("[0x{addr:04X}]")
            } else {
                let base = self.work();
                let off = self.rng.range_i64(-8, 8) * step as i64;
                self.line(&format!("li {base}, 0x{:04X}", addr as i64 - off));
                match off {
                    0 => format!("[{base}]"),
                    o if o > 0 => format!("[{base} + {o}]"),
                    o => format!("[{base} - {}]", -o),
                }
            }
        };
        self.line(&format!("{mnemonic} {value}, {operand}"));
    }

    /// `ALLOC`, a few accesses inside the allocated part, `FREE`.
    fn heap(&mut self) {
        let n = 1 + self.rng.below(64);
        let words = n.div_ceil(4);
        self.line(&format!("li r0, {n}"));
        self.line("sys ALLOC");
        self.line("mov r5, r0");
        for _ in 0..1 + self.rng.below(3) {
            let off = 4 * self.rng.below(words);
            let r = self.any_reg();
            match self.rng.below(3) {
                0 => self.line(&format!("st {r}, [r5 + {off}]")),
                1 => {
                    let d = WORK[self.rng.below(5) as usize];
                    self.line(&format!("ld {d}, [r5 + {off}]"));
                }
                _ => {
                    let o = off + self.rng.below(4);
                    self.line(&format!("stb {r}, [r5 + {o}]"));
                }
            }
        }
        self.line("mov r0, r5");
        self.line("sys FREE");
    }

    /// Forward conditional jump over a few instructions.
    fn branch(&mut self, frame: bool) {
        let skip = self.fresh("S");
        let (a, b) = (self.any_reg(), self.any_reg());
        self.line(&format!("cmp {a}, {b}"));
        let cond = Cond::ALL[self.rng.below(Cond::ALL.len() as u64) as usize];
        self.line(&format!("{} {skip}", cond.mnemonic()));
        for _ in 0..1 + self.rng.below(3) {
            self.simple(frame);
        }
        self.label(&skip);
    }

    /// Forward register jump.
    fn jump_reg(&mut self) {
        let target = self.fresh("J");
        let r = self.work();
        self.line(&format!("li {r}, {target}"));
        self.line(&format!("jmp {r}"));
        self.alu();
        self.label(&target);
    }

    /// `push` … `pop` around a few instructions.
    fn push_pop(&mut self, frame: bool) {
        let r = self.any_reg();
        self.line(&format!("push {r}"));
        for _ in 0..self.rng.below(3) {
            self.simple(frame);
        }
        let d = self.work();
        self.line(&format!("pop {d}"));
    }

    /// Register or memory instruction.
    fn simple(&mut self, frame: bool) {
        if self.rng.chance(1, 4) {
            self.mem(frame);
        } else {
            self.alu();
        }
    }

    /// Inner loop with counter `r7`.
    fn inner_loop(&mut self) {
        let top = self.fresh("I");
        let n = 2 + self.rng.below(19);
        self.line(&format!("mov r7, {n}"));
        self.label(&top);
        for _ in 0..2 + self.rng.below(5) {
            self.simple(false);
        }
        self.line("sub r7, r7, 1");
        self.line(&format!("jnz {top}"));
    }

    /// A call of a subroutine with a number ≥ `from`, direct or via `call rs`.
    fn call(&mut self, from: u64) {
        if from >= self.funcs {
            return;
        }
        let f = from + self.rng.below(self.funcs - from);
        if self.rng.chance(1, 3) {
            let r = self.work();
            self.line(&format!("li {r}, f{f}"));
            self.line(&format!("call {r}"));
        } else {
            self.line(&format!("call f{f}"));
        }
    }

    fn syscall(&mut self) {
        match self.rng.below(5) {
            0 => self.line("sys WRITE_INT"),
            1 => self.line("sys WRITE_CHAR"),
            2 => {
                self.line("li r0, msg");
                self.line("sys WRITE_STR");
            }
            3 => self.line("sys READ_CHAR"),
            _ => self.line("sys RAND"),
        }
    }

    /// A snippet that faults.
    fn fault(&mut self) {
        let lines: &[&str] = match self.rng.below(12) {
            0 => &["mov r1, 0", "ld r0, [r1 + 8]"],
            1 => &["mov r1, 0", "div r0, r0, r1"],
            2 => &["st r0, [0x0100]"],
            3 => &["ld r0, [0x4102]"],
            4 => &["ld r0, [0xC000]"],
            5 => &["li r1, 0xFFFF", "ld r0, [r1 + 4]"],
            6 => &[
                "li r0, 8",
                "sys ALLOC",
                "mov r5, r0",
                "sys FREE",
                "ld r1, [r5]",
            ],
            7 => &[
                "li r0, 8",
                "sys ALLOC",
                "mov r5, r0",
                "sys FREE",
                "mov r0, r5",
                "sys FREE",
            ],
            8 => &["li r0, 8", "sys ALLOC", "st r1, [r0 + 8]"],
            9 => &["li r5, 0x4100", "jmp r5"],
            10 => &["pop r0", "pop r0"],
            _ => &["sys 99"],
        };
        for l in lines {
            self.line(l);
        }
    }

    /// One statement of the outer loop body.
    fn outer_statement(&mut self) {
        match self.rng.below(20) {
            0..=6 => self.alu(),
            7..=9 => self.mem(false),
            10 => self.branch(false),
            11 => self.jump_reg(),
            12 => self.push_pop(false),
            13 | 14 => self.inner_loop(),
            15 | 16 => self.call(0),
            17 => self.syscall(),
            18 => self.heap(),
            _ => self.alu(),
        }
    }

    /// One statement of subroutine `f`.
    fn func_statement(&mut self, f: u64) {
        match self.rng.below(12) {
            0..=4 => self.alu(),
            5..=7 => self.mem(true),
            8 => self.branch(true),
            9 => self.push_pop(true),
            10 => self.heap(),
            _ => self.call(f + 1),
        }
    }
}

/// Source text of program `index` (deterministic in `rng`).
pub fn program_source(rng: &mut SplitMix64, index: u32) -> String {
    let funcs = 1 + rng.below(4);
    let rounds = 10 + rng.below(111);
    let faulty = rng.chance(1, FAULT_ONE_IN);
    let mut g = Gen {
        rng,
        out: format!(
            "; fuzz-gen program {index}\n        .data\nmsg:    .string \"fuzz\\n\"\n        .text\nmain:\n"
        ),
        labels: 0,
        funcs,
    };
    for r in WORK {
        let v = g.rng.next_u32();
        g.line(&format!("li {r}, 0x{v:08X}"));
    }
    g.line(&format!("li r6, {rounds}"));
    g.label("outer");
    let fault_at = if faulty {
        Some(1 + g.rng.below(rounds))
    } else {
        None
    };
    let body = 6 + g.rng.below(15);
    let fault_pos = g.rng.below(body);
    for i in 0..body {
        if i == fault_pos
            && let Some(k) = fault_at
        {
            let skip = g.fresh("F");
            g.line(&format!("cmp r6, {k}"));
            g.line(&format!("jnz {skip}"));
            g.fault();
            g.label(&skip);
        }
        g.outer_statement();
    }
    g.line("sub r6, r6, 1");
    g.line("jnz outer");
    g.line("halt");
    for f in 0..funcs {
        g.label(&format!("f{f}"));
        g.line("push fp");
        g.line("mov fp, sp");
        g.line("sub sp, sp, 16");
        for _ in 0..3 + g.rng.below(10) {
            g.func_statement(f);
        }
        g.line("mov sp, fp");
        g.line("pop fp");
        g.line("ret");
    }
    g.out
}

/// Runs a program and returns (status, steps, digests as (step, digest)).
pub fn run_digests(machine: &mut Machine) -> (Status, u64, Vec<(u64, u64)>) {
    let mut digests = vec![(0, machine.state.digest(&machine.tapes))];
    let status = loop {
        let mut s = Status::Paused;
        for _ in 0..DIGEST_EVERY {
            s = machine.step();
            if s != Status::Paused {
                break;
            }
        }
        if s != Status::Paused {
            break s;
        }
        digests.push((machine.state.steps, machine.state.digest(&machine.tapes)));
    };
    let last = (machine.state.steps, machine.state.digest(&machine.tapes));
    if digests.last() != Some(&last) {
        digests.push(last);
    }
    (status, machine.state.steps, digests)
}

/// Writes `count` programs `fuzz-NNNNN.s` and `digests.txt` into `out`.
pub fn generate(seed: u64, count: u32, out: &Path) -> Result<String, String> {
    std::fs::create_dir_all(out).map_err(|e| format!("cannot create '{}': {e}", out.display()))?;
    let mut rng = SplitMix64::new(seed);
    let mut digests = format!(
        "# stapelwerk fuzz-gen --seed {seed} --count {count}\n\
         # file rand_seed budget status steps step:digest...\n\
         # Digests (FNV-1a-64, MachineState::digest) after every {DIGEST_EVERY} steps and at the\n\
         # end. Run in test mode: input closed and empty, RAND seeded with rand_seed.\n"
    );
    let mut faults = 0u32;
    for i in 0..count {
        let src = program_source(&mut rng, i);
        let rand_seed = rng.next_u32();
        let name = format!("fuzz-{i:05}.s");
        let program = assemble(&src, None, &MnemonicFilter::All).map_err(|d| {
            format!(
                "internal error: generated program {name} does not assemble ({:?})",
                d.first().map(|d| d.key)
            )
        })?;
        let mut m = Machine::new(
            &program,
            MachineConfig {
                seed: rand_seed,
                budget: FUZZ_BUDGET,
                ..MachineConfig::default()
            },
        );
        let (status, steps, list) = run_digests(&mut m);
        if status == Status::Fault {
            faults += 1;
        }
        let _ = write!(
            digests,
            "{name} {rand_seed} {FUZZ_BUDGET} {} {steps}",
            status.code()
        );
        for (step, d) in list {
            let _ = write!(digests, " {step}:{d:016x}");
        }
        digests.push('\n');
        let path = out.join(&name);
        std::fs::write(&path, src)
            .map_err(|e| format!("cannot write '{}': {e}", path.display()))?;
    }
    let path = out.join("digests.txt");
    std::fs::write(&path, digests)
        .map_err(|e| format!("cannot write '{}': {e}", path.display()))?;
    Ok(format!(
        "wrote {count} program(s) and digests.txt to {} ({faults} ended with a fault)",
        out.display()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn programs_assemble_and_are_deterministic() {
        let mut a = SplitMix64::new(5);
        let mut b = SplitMix64::new(5);
        for i in 0..20 {
            let s = program_source(&mut a, i);
            assert_eq!(s, program_source(&mut b, i));
            let p = assemble(&s, None, &MnemonicFilter::All).expect(&s);
            let mut m = Machine::new(
                &p,
                MachineConfig {
                    budget: 5000,
                    ..MachineConfig::default()
                },
            );
            let (_, steps, d) = run_digests(&mut m);
            assert_eq!(d.last().map(|x| x.0), Some(steps));
        }
    }

    /// Most programs run long and only the planted snippets fault.
    #[test]
    fn programs_run_long_and_fault_only_on_purpose() {
        let mut rng = SplitMix64::new(11);
        let mut steps = Vec::new();
        let mut faults = 0;
        for i in 0..100 {
            let src = program_source(&mut rng, i);
            let p = assemble(&src, None, &MnemonicFilter::All).expect(&src);
            let mut m = Machine::new(
                &p,
                MachineConfig {
                    seed: rng.next_u32(),
                    budget: FUZZ_BUDGET,
                    ..MachineConfig::default()
                },
            );
            let (status, n, _) = run_digests(&mut m);
            if status == Status::Fault {
                faults += 1;
                assert!(
                    src.lines().any(|l| l.starts_with('F') && l.ends_with(':')),
                    "unplanned fault {:?} in\n{src}",
                    m.state.fault
                );
            }
            steps.push(n);
        }
        steps.sort_unstable();
        assert!(faults <= 25, "{faults} faults");
        assert!(steps[50] >= 5000, "median {} steps", steps[50]);
    }
}

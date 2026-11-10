//! `fuzz-gen`: random programs for the differential test native vs. WebAssembly.
//!
//! Each program is a register prologue (`li` with data-segment addresses, so memory instructions
//! often hit valid memory) followed by random valid instructions printed by the disassembler.
//! Jumps and calls get a random label inside the program instead of a random offset, `sys` uses a
//! real system call number (no `READ_INT`, the input is empty), and register jumps are left out.
//! Each program is run natively in test mode (closed, empty input); `digests.txt` records the
//! state digest every 1000 steps and at the end.

use stapelwerk_core::asm::{MnemonicFilter, assemble};
use stapelwerk_core::disasm::format_instr;
use stapelwerk_core::isa::{Cond, Instr, decode};
use stapelwerk_core::machine::{Machine, MachineConfig, Status};
use stapelwerk_core::rng::SplitMix64;
use std::fmt::Write as _;
use std::path::Path;

/// Instructions per program (besides prologue and final `halt`).
const BODY_LEN: u64 = 48;
/// Cycle budget per program.
pub const FUZZ_BUDGET: u64 = 100_000;
/// Digest interval in steps.
pub const DIGEST_EVERY: u64 = 1000;

/// System calls used by generated programs: WRITE_INT, READ_CHAR, WRITE_CHAR, WRITE_STR, ALLOC,
/// FREE, RAND.
const SYSCALLS: [u16; 7] = [2, 3, 4, 5, 6, 7, 8];

/// One random body instruction as source text.
fn random_line(rng: &mut SplitMix64) -> String {
    loop {
        let word = rng.next_u32();
        let Ok(instr) = decode(word) else {
            continue;
        };
        return match instr {
            Instr::JumpReg { .. } | Instr::CallReg { .. } => continue,
            Instr::Halt if !rng.chance(1, 8) => continue,
            Instr::Jump { .. } => {
                let cond = Cond::ALL[rng.below(Cond::ALL.len() as u64) as usize];
                format!("{} L{}", cond.mnemonic(), rng.below(BODY_LEN))
            }
            Instr::Call { .. } => format!("call L{}", rng.below(BODY_LEN)),
            Instr::Sys { .. } => {
                let n = SYSCALLS[rng.below(SYSCALLS.len() as u64) as usize];
                format_instr(&Instr::Sys { n }, 0x100)
            }
            other => format_instr(&other, 0x100),
        };
    }
}

/// Source text of program `index` (deterministic in `rng`).
pub fn program_source(rng: &mut SplitMix64, index: u32) -> String {
    let mut s = format!("; fuzz-gen program {index}\n");
    for r in ["r0", "r1", "r2", "r3", "r4", "r5", "r6", "r7"] {
        let addr = 0x4000 + 4 * rng.below(0x800);
        let _ = writeln!(s, "        li {r}, 0x{addr:04X}");
    }
    let _ = writeln!(s, "        li fp, 0xE800");
    for i in 0..BODY_LEN {
        let _ = writeln!(s, "L{i}:\n        {}", random_line(rng));
    }
    s.push_str("        halt\n");
    s
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
}

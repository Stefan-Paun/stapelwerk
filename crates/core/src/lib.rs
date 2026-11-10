//! Stapelwerk core: the teaching CPU, its assembler, the level format and the test runner.
//!
//! Everything in this crate is deterministic and free of `unsafe`. The normative description of the
//! machine is `docs/ISA.md`; the module [`isa`] is the single source of truth for opcodes, encodings,
//! cycle costs and flag effects, and every other module reads from it.
//!
//! Module ownership during the build phases (see `docs/STATUS.md`):
//!
//! | Module | Contents | Status |
//! |---|---|---|
//! | [`isa`], [`consts`], [`fault`], [`diag`], [`json`], [`rng`], [`program`] | shared contract | frozen (phase 0) |
//! | [`mem`], [`heap`], [`cpu`], [`sys`], [`machine`] | the machine | phase 1A |
//! | [`asm`], [`disasm`] | assembler and disassembler | phase 1B |
//! | [`level`], [`runner`], [`session`] | levels, tests, debugger | later phases |

#![forbid(unsafe_code)]

pub mod asm;
pub mod consts;
pub mod cpu;
pub mod diag;
pub mod disasm;
pub mod fault;
pub mod heap;
pub mod isa;
pub mod json;
pub mod level;
pub mod machine;
pub mod mem;
pub mod program;
pub mod rng;
pub mod runner;
pub mod session;
pub mod sys;

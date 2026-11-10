# Status

**Version 1.0.0 (2026-11-10): feature-complete.** The machine, the assembler, the debugger with time
travel, the level system, the command-line tool, the WebAssembly bridge, all 30 levels and the
interface are done and tested. What is still open is listed at the end.

## What exists

| Area | State |
|---|---|
| Workspace | `crates/core` (`stapelwerk-core`), `crates/wasm` (`stapelwerk-wasm`, cdylib), `crates/cli` (`stapelwerk-cli`, binary `stapelwerk`); Rust 1.99.1 pinned; zero third-party crates; `unsafe` only in `crates/wasm` |
| Machine | ISA as the single source in `isa.rs` (51 opcodes, 47 mnemonics), CPU, memory with segments, checked heap, 8 system calls, 23 precise faults ([ISA.md](ISA.md)) |
| Assembler | two passes, expressions, directives, local labels, `li`, short forms, library pre-pass, allow/forbid, diagnostics with UTF-16 columns and suggestions; canonical disassembler |
| Session | debugger (step, back, run in time slices, goto, breakpoints, `set_reg`, back to the last change, line counters, frames, heap list) with time travel through snapshots and replay |
| Levels | 30 levels in 8 chapters ([CURRICULUM.md](CURRICULUM.md)), 439 tests in total (326 hidden); parser, test runner, metrics, packer ([LEVELS.md](LEVELS.md)) |
| CLI | `asm`, `dis`, `run` (also `--level`/`--test`/`--trace`), `check-levels --strict --mutate`, `pack-levels`, `isa-table`, `fuzz-gen` |
| WebAssembly | ABI v1: 42 functions plus `memory`, no imports ([ABI.md](ABI.md), [abi-exports.txt](abi-exports.txt)); `web/js/vm.js` is the only module that touches it |
| Interface | chapter plan, workspace (editor, debugger, registers, stack, screen, console, hex dump, heap map, tests, result), manual, sandbox with share links, about page with progress export/import; German and English; light and dark theme; offline through a service worker. The JavaScript stand-in `vm-mock.js` is only used with `?mock=1` |
| Tooling | `npm run build/start/test/test:browser/check` via `tools/*.mjs`; CI workflows `test` (Rust, site on Linux and macOS with Node 22 and 24, browser, fuzz/differential), `pages`, `codeql`; Dependabot for actions |
| Repository | README (EN/DE), CONTRIBUTING, SECURITY, CHANGELOG, CLAUDE.md, issue and pull request templates, MIT licence |

## Verification (measured 2026-11-10)

| Check | Result |
|---|---|
| `cargo test --workspace` | 221 passed, 0 failed, 3 ignored (two benchmarks, the exhaustive 2³² test that CI runs in release mode) |
| `node --test test/unit test/wasm test/site` | 92 passed |
| `check-levels --strict --mutate levels/` | 30 levels good; 550 of 550 deletions caught, 12 lines skipped with `; mutate:skip` |
| `dist/wasm/stapelwerk.wasm` | 209 893 bytes (limit 300 KiB) |
| Differential native vs. WebAssembly | 50 programs locally (`npm run check`), 300 per CI run |
| Browser tests (`test/browser`) | headless Chrome over CDP: smoke, levels, keyboard-only play, debugger, five viewports, reduced motion, damaged storage, offline, sub-path, sandbox, XSS probes, share links |

## Development log

| Phase | Result |
|---|---|
| 0 | Workspace, `isa.rs` as the single source, frozen specification (ISA, ABI, LEVELS, ERRORS, CURRICULUM), CI |
| 1A | Machine: memory, CPU, heap, system calls; ALU vectors and a random ALU model |
| 1B | Assembler and disassembler; golden files, text round trip, no panic on corrupted input |
| 2A | Session with debugger and time travel; time-travel invariant over about 540 000 steps |
| 2B | Level parser, test runner, packer, CLI, sample level 2.2 |
| 3A | WebAssembly bridge, `vm.js`, differential test; test details in `level_info`, structured `fuzz-gen` |
| Levels | Chapters 0–3 (14 levels) and 4–7 (15 levels), complete manifest |
| CLI | `run --level/--test/--budget/--heap-check/--library`, address expressions in tests |
| Interface | "Reißbrett" design: chapter plan, workspace, debugger, sandbox, manual, about |
| Review | Debugger and runner share the test setup; BOM handling in the assembler |
| Release | Browser tests, offline mode, English page, repository documents |

Clarifications made along the way are part of the specification: see the *clarification* notes in
[ISA.md](ISA.md) and the "Clarifications" sections of [ABI.md](ABI.md).

## Open

- Images for the README: `docs/img/demo.gif`, `docs/img/workspace.png`, `docs/img/chapters.png`,
  `docs/img/mobile.png` and the German screenshots in `docs/img/de/` do not exist yet.
- Publishing: GitHub Pages must be enabled (Settings → Pages → Source: GitHub Actions) before the
  first push to `main`, otherwise the `pages` workflow fails at `configure-pages`. Private
  vulnerability reporting must be switched on for [SECURITY.md](../SECURITY.md).
- Portfolio: no tile on stefanpaun.page yet.

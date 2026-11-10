# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] - 2026-11-10

First public release.

### Added

- **Machine:** a 32-bit teaching CPU with 10 registers, flags Z/N/C/V, 47 mnemonics on 51 opcodes in four fixed 32-bit formats with strict decoding, 64 KiB of memory in eight segments with fixed rights, a 64×64 screen with 16 named colours, 8 system calls, a checked heap with red zones and fill patterns, and 23 precise, named runtime faults ([docs/ISA.md](docs/ISA.md)).
- **Assembler and disassembler:** labels and local labels, expressions, data directives, the `li` pseudo-instruction, two-operand short forms, "did you mean" suggestions, error positions in UTF-16 columns, and a canonical text form that assembles back to the same word.
- **Session and debugger:** step, step back, run in time slices, restart, timeline, breakpoints, editing registers while paused, "back to the last change", line counters, frame chain and heap list, with time travel through snapshots and replay.
- **Levels:** 30 levels in 8 chapters from registers to heap lists, in German and English, each with task, three hints, visible and hidden tests, reference solution and "What you just learned"; seven debugging levels and three bonus levels ([docs/CURRICULUM.md](docs/CURRICULUM.md)).
- **Level tooling:** the `.lvl` format with I/O, memory, register, screen, function and heap tests; `check-levels --strict --mutate` with a mutation test of every reference solution; `pack-levels` for the web build ([docs/LEVELS.md](docs/LEVELS.md)).
- **Command-line tool** `stapelwerk`: `asm`, `dis`, `run` (also against a level's tests, with `--trace`), `check-levels`, `pack-levels`, `isa-table`, `fuzz-gen`.
- **WebAssembly bridge:** a hand-written C ABI with 42 functions and no imports ([docs/ABI.md](docs/ABI.md)), and `web/js/vm.js` as the only JavaScript module that touches it.
- **Interface "Reißbrett":** chapter plan, workspace with editor, debugger, registers, stack with frames, screen (actual, expected, difference), console, hex dump, heap map and tests; result page with metrics, badges and solution slots; manual with eleven topics; sandbox with examples and share links; light and dark theme; German and English pages.
- **Offline and privacy:** service worker for offline play, progress only in local storage with export and import, no cookies, no tracking, no requests to other servers, strict Content-Security-Policy.
- **Accessibility:** keyboard shortcuts for the whole debugger, live region for debugger steps, stack as a table, reduced motion and forced colours respected, changes marked by more than colour.
- **Tests and CI:** Rust tests (including an exhaustive test over all 2³² instruction words in CI), Node tests, browser tests in headless Chrome, a differential test of native against WebAssembly, the level mutation test, CodeQL, and GitHub Pages deployment.

[1.0.0]: https://github.com/Stefan-Paun/stapelwerk/releases/tag/v1.0.0

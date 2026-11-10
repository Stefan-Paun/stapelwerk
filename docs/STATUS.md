# Status

**Phase 0 (foundation and contract): done.** Phase 1A (machine) and phase 1B (assembler and
disassembler): done. Phase 2 (session, levels, runner, CLI) and phase 3A (WebAssembly bridge): done.

## What exists

| Area | State |
|---|---|
| Workspace | `crates/core` (`stapelwerk-core`), `crates/wasm` (`stapelwerk-wasm`, cdylib), `crates/cli` (`stapelwerk-cli`, binary `stapelwerk`); Rust 1.99.1 pinned; zero third-party crates; release profile per spec |
| Instruction set | `crates/core/src/isa.rs` complete: `Reg`, `Cond`, `AluOp`, `Format`, `Shape`, `ImmKind`, `FlagEffect`, `OpInfo`, the opcode table `OPCODES`, `Instr`, `Mem`, `encode`, strict `decode`, `isa_table_markdown` |
| Shared contract | `consts.rs` (memory map, segments, colours, syscalls, limits, reserved names), `fault.rs` (all `fault.*` keys, codes, args, JSON), `diag.rs` (`Diagnostic`, `Arg`, all keys), `program.rs` (`Program`, `Symbol`, `LineEntry`), `json.rs` (writer), `rng.rs` (SplitMix64, xorshift32) |
| Placeholders | `level.rs` (data model done, parser open), `runner.rs` (result types and metrics done), `session.rs` (signatures) |
| Tests | 8 mandatory encoding vectors, random encode/decode round trip, random words, exhaustive 2³² test (`#[ignore]`, CI release), 90 ALU vectors in `tests/vectors/alu.txt` (parsed; evaluation `#[ignore]` until phase 1A), ERRORS.md ↔ key list, CLI tests, Node ABI tests |
| WebAssembly | full ABI v1 (`crates/wasm`, 42 exports + `memory`, no imports, ≈ 201 KiB) and `web/js/vm.js`; see "Phase 3A result" |
| CLI | argument parsing for all commands; `isa-table` works; all others exit 2 with "not implemented yet" |
| Docs | ISA, ABI, LEVELS, ERRORS, CURRICULUM, isa-table (generated), abi-exports.txt, TO-REAL-CPUS |
| Tooling | `npm run build/start/test/check/test:browser` via `tools/*.mjs`; CI workflows test, pages, codeql; Dependabot for actions |

## Interfaces for phase 1A (machine)

Files owned by 1A: `crates/core/src/mem.rs`, `heap.rs`, `cpu.rs`, `sys.rs`, `machine.rs`. Every
placeholder is marked `PHASE 1A` and returns a harmless default.

- `cpu::alu(op: AluOp, a: u32, b: u32) -> Result<(u32 /*result*/, u32 /*new flag word*/), FaultKind>`
  — must satisfy `tests/vectors/alu.txt`; then remove `#[ignore]` from
  `crates/core/tests/alu_vectors.rs::vectors_match_cpu_alu`.
- `cpu::step(&mut MachineState, &mut Tapes, &MachineConfig) -> Status` — contract in its doc comment
  (precise faults, `WAITING_INPUT` without side effects, `halt` keeps `pc`).
- `sys::syscall(n: u16, &mut MachineState, &mut Tapes, &MachineConfig) -> Result<SysOutcome, FaultKind>`.
- `mem::Memory`: `load_program`, `is_code_target`, `check`, `read_u32`, `read_u8`, `write_u32`,
  `write_u8`, `fetch` (implemented: `new`, `bytes`, `bytes_mut`, `screen`, `code_end`, `copy_from`,
  `peek_u32`).
- `heap::Heap`: `alloc`, `free`, `check_access` (implemented: `new`, `shadow`, `blocks`,
  `live_count`, `shadow_at`, `copy_from`; shadow constants `SHADOW_*`; `HeapCheck`).
- `machine::Machine::run(max_cycles)`, `MachineState::digest(&Tapes) -> u64` (FNV-1a-64).
  Implemented: `Status`, `MachineConfig`, `Tapes::push_output`, `MachineState::{new, reg, set_reg,
  restore_from}`, `Machine::{new, step}`.
- Input to the machine is only `program::Program` (see "Interfaces for phase 1B"); tests can build a
  `Program` by hand with `isa::encode`, so 1A does not need the assembler.
- Faults: `fault::FaultKind` (+ `.at(pc, step)` → `Fault`); access kinds `fault::Access`.
- Semantics: [ISA.md](ISA.md), including the clarifications on check order, stack checks, heap
  layout, `WRITE_STR`, `RAND`, `halt` and the budget.

## Interfaces for phase 1B (assembler)

Files owned by 1B: `crates/core/src/asm/` (all files) and `crates/core/src/disasm.rs`. The internal
split of `asm/` (lexer, expr, parser, layout) is only a suggestion.

- `asm::assemble(player: &str, library: Option<&str>, filter: &MnemonicFilter) -> Result<Program, Vec<Diagnostic>>`
  (implemented: `MnemonicFilter::permits`, `canonical_mnemonic`, `lexer::utf16_col`).
- Output: `program::Program { words, text_len, data, symbols, linemap, trap_addr, size }` with
  `Symbol { name, kind: Code|Data|Const, value: i64, file, line }` and
  `LineEntry { addr, file, line }`; layout rules in its doc comment and in ISA.md.
- Diagnostics: `diag::Diagnostic { key, file, line, col, len, args }` built with
  `Diagnostic::new(key, file, line, col, len).arg(name, value)`; keys and argument names must match
  `diag::ALL_KEYS` / [ERRORS.md](ERRORS.md); columns in UTF-16 units.
- Lookups to use (never duplicate tables): `isa::rows_for_mnemonic`, `isa::Cond::from_mnemonic`,
  `isa::Reg::from_name`, `isa::encode`, `ImmKind::range`, `consts::syscall_number`,
  `consts::builtin_constant`, `consts::is_reserved_name`, `consts::DIRECTIVES`,
  `consts::MAX_ASM_ERRORS`, `consts::SUGGESTION_DISTANCE`, `consts::TRAP_LABEL`.
- `disasm::format_instr(&Instr, addr) -> String` (canonical text form, ISA.md) and
  `disasm::disasm_range(mem, program, addr, count) -> Vec<DisasmLine>`; `disasm_word` and
  `jump_target` are implemented. Spec test 5 (`asm(dis(w)) == w`) can use the generator
  `isa::tests::random_instr` (`pub(crate)` in `#[cfg(test)]`).
- Golden tests go to `tests/asm/*.s` with `; expect-word:` / `; expect-error: key line:col`.

### Phase 1B result

- `asm::assemble` complete: lexer (UTF-16 columns), i64 expressions with overflow checks (postfix
  form, nesting limited to 64), parser with error recovery (one diagnostic per line, all lines,
  sorted, at most 50 + `asm.too_many_errors`), library `.equ` pre-pass, two passes, all directives,
  local labels, `li`, short forms, condition aliases, `sys` by name or number, memory operands,
  allow/forbid (player only), line map, symbols, size. "Did you mean" for mnemonics and directives
  (Levenshtein ≤ 2 and smaller than the name's length).
- Layout clarification: one zero word between the player's code and the library/trap (ISA.md,
  "Player and library"); `.align` in `.text` pads with `nop`; library `.data` starts 4-aligned.
- `asm.imm_range` has an optional `hint` (`"li"` for `mov rd, imm`); `asm.operand_type` may also
  say `"string"` (`.string`, `.ascii`). Unknown symbols get no suggestion (ERRORS.md has no such
  argument for `asm.undefined_symbol`).
- `disasm::format_instr` (canonical form), `format_instr_labeled` (jump targets as labels),
  `disasm_range` (fills `label` from the symbol table).
- Tests: golden files `tests/asm/*.s` (every opcode, every `asm.*` key) with the runner
  `crates/core/tests/asm_golden.rs`; text round trip 10⁵ random instructions plus all immediate
  limits (`disasm.rs`) and 10⁵ random decodable words (`asm_fuzz.rs`); no panic on 12 000 corrupted
  sources and 12 000 random byte strings; end-to-end programs on the machine (`asm_e2e.rs`);
  layout and binding rules (`asm_layout.rs`).
- Speed: a 502-line program assembles in about 1.5 ms natively (release, M3).

### Phase 2A result (session, debugger, time travel)

- `crates/core/src/session.rs` (+ `session/{call,inspect,lines}.rs`): `Session::new(SessionConfig)`
  (library, filter, heap_check, seed, budget; `Default` = sandbox with budget 10⁷), `assemble`,
  `load_program`, `reset(Option<TestSetup>)`, `step`, `run` (time slice, breakpoints via
  `RunObserver`), `back`, `goto`, `back_to_change`, `set_reg`, breakpoints, `push_int`/`push_text`/
  `close_input`; queries `view` (`StateView`, 96 bytes), `memory`, `shadow`, `output`, `fault`,
  `symbols`, `linemap`, `line_counts`, `hottest_line`, `frames`, `heap_blocks`, `disasm`, `digest`,
  `call_clobbered`. `TestSetup` is the hand-over from `[test]` to the session (same start order as
  `runner::prepare_test`, checked by a test).
- Time travel per SPEC §14: snapshots every K steps (K = 1024, > 64 → thin, K doubles); a snapshot
  holds the state *before* the edits of its step; `goto` restores in place and replays with the
  edit log; line counters are saved with each snapshot.
- Clarifications: `goto` lands on PAUSED (or HALTED); faults, breakpoints, waiting and the budget
  reappear on the next `step`/`run`. A time slice that ends right before a breakpoint reports
  BREAKPOINT (no breakpoint is skipped at slice boundaries). Register indices 10 = flags (low 4 bits),
  11 = pc for `set_reg`/`back_to_change`. Function tests end at `__trap` without executing it (also
  when the budget is reached exactly there). Breakpoints only on line-map addresses; cleared by
  `assemble`. A failed `assemble` drops the program. Output vectors reserve 8192 entries once, so
  all exported buffers keep their address for the whole session.
- Tests: `session_timetravel.rs` (≈ 540 000 steps, 4 thinnings, digests and line counters at random
  `goto`s, `set_reg` edits, input while WAITING_INPUT incl. back-and-forward without input,
  discarded future), `session_debugger.rs` (16 tests), `session_bench.rs` (`#[ignore]`).
- Speed (release, M3): `back(1)` after 10⁶ steps ≈ 27 µs on average, worst case `goto` to one step
  before a snapshot boundary (K = 16384) ≈ 0.3 ms; 10⁶ steps forward ≈ 28 ms.

### Phase 2B result (levels, test runner, CLI)

- `level::parse` complete (all sections and keys, `[image.NAME]`, exactly 3 hints per language,
  every `lvl.*` key with file line and UTF-16 column, at most 50 diagnostics, never panics);
  `Level::info_json` (`level_info`), `Level::hints`; `level::pack` (`pack_source`,
  `parse_manifest`, `index_json`).
- `runner`: `assemble_level`, `prepare_test`/`run_prepared` (fresh `Machine` per test, no
  session), `run_test`, `run_tests`, `run_tests_with`, `run_reference`, `reference_screen`,
  function tests via `session::prepare_call`/`run_to_end` (same loop and cycle count as the
  debugger), `TestReport::to_json` (`sess_run_tests` shape with `input`), `pack_level`;
  `TestCase::to_setup` builds the session's `TestSetup`. Comparison order and output semantics:
  LEVELS.md.
- CLI: `asm`, `dis`, `run` (`--in`, `--in-text`, `--trace`), `check-levels [--strict] [--mutate]`,
  `pack-levels`, `isa-table`, `fuzz-gen`; English messages for every key; exit codes 0/1/2.
- `levels/manifest.txt` (8 chapters) and the sample level 2.2 (4 visible + 11 hidden tests; the
  redundant `mov r1, 0` carries `; mutate:skip` because registers start at 0).
- `npm run check` and CI now run `check-levels --strict --mutate levels/`; the CI fuzz job
  generates programs and digests; the differential test itself stays guarded until phase 3.
- Tests: `tests/level_parse.rs` (every `lvl.*` key, no panic on 4000 corrupted files incl.
  runner and packer), `tests/runner_tests.rs` (every test kind and `check.*` key, function tests,
  heap, budget, JSON, pack round trip), `crates/cli/tests/cli_commands.rs` (end to end).

### Phase 3A result (WebAssembly bridge)

- `crates/wasm`: every export of `abi-exports.txt` as a hand-written C ABI (`#[unsafe(no_mangle)]
  pub extern "C"`, only `i32`/`u32`, no imports, `panic = "abort"`). One `thread_local!` `RefCell`
  holds a generational handle table (`table.rs`, levels and sessions in one table), the output
  buffer and the registry of input buffers. The only `unsafe` is the `no_mangle` attribute: input
  `(ptr, len)` ranges are looked up in the `buf_alloc` registry and copied (unknown range →
  `abi.bad_arg`), exported pointers are addresses of boxed buffers. `sess_view` points to a boxed
  `StateView` refreshed after every session call. `sess_run_tests`/`sess_check` use the runner;
  for unpacked levels the reference run is computed once per level and cached (packed levels use
  `ref_cycles`/`ref_size`). Behaviour details: ABI.md, "Clarifications made in phase 3A".
- `web/js/vm.js`: `loadVm`, `Level`, `Session` (API of `vm-mock.js` plus the ABI; differences
  listed in ABI.md, "web/js/vm.js contract"), typed-array views recreated after `memory.grow`,
  trap recovery (new instance, levels and sessions reloaded, `VmError('internal')`). Pure ES module,
  runs under Node.
- `tools/build.mjs`: release build with `--remap-path-prefix` for HOME and the repository, refuses a
  module containing `/Users/` or `/home/`, copies `web/`, runs `pack-levels levels dist/levels`
  (packed `.lvl` + `index.json`).
- `tools/differential.mjs`: reads `fuzz-gen` output (or runs it: default seed 1, N = 50), replays
  every program through `vm.js` in test mode with the same seed and budget and compares status,
  steps and the digests every 1000 steps. CI: fuzz job with N = 300; `npm run check` runs N = 50.
- Tests: `crates/wasm` unit tests (handle table, JSON shapes, exports natively), Node tests
  `test/wasm/abi.test.mjs` (exact export list, no imports, size, no local paths, raw handle errors)
  and `test/wasm/vm.test.mjs` (level 2.2 unpacked and packed, diagnostics, debugger, input,
  time slices, JSON shapes, views after `memory.grow`, bad handles, trap recovery).
- Measured (M3, Node 24): `stapelwerk.wasm` 206 217 bytes (201.4 KiB); level 2.2 "assemble the
  reference + run all 15 tests" through `vm.js`: ≈ 5 ms for the very first call of a process (engine
  warm-up), then median 0.15 ms (packed level, browser case; p90 0.24 ms) and 0.11 ms (unpacked,
  reference run cached after its first use); 200 repetitions each.

### Phase 3A follow-up (test details, stronger fuzz-gen)

- `level_info` lists for every visible test `in`, `in_text`, `out`, `out_text`, `regs`, `set`,
  `call`, `expect_reg`, `expect`, `screen_init`, `expect_screen`, `heap_empty`
  (`TestCase::write_details_json`; additive extension of ABI v1, ABI.md "Test details in
  `level_info`"). `vm.js`: `level.image(name)`; `referenceScreen(test)` follows the test's
  `expect_screen`.
- `fuzz-gen` (`crates/cli/src/fuzz.rs`) builds structured programs: outer loop with counter,
  inner loops, subroutines with prologue/epilogue and locals, direct and register calls and jumps,
  forward branches, balanced push/pop, memory accesses only at valid aligned addresses (data,
  bottom of the stack, screen, frame locals, fresh heap blocks), `ALLOC`/`FREE`, output system
  calls, divisors made odd before division; one program in eight faults on purpose (12 snippets).
  N = 300, seed 1: 262 HALTED, 34 FAULT (all planted), 4 BUDGET_EXCEEDED; steps median 6642,
  p10 1289, p90 22555, max 54652; differential 2.86 million steps, 3306 digests in 0.84 s.
  A unit test keeps the median ≥ 5000 steps and only planted faults.

## Clarifications made in phase 0

These fill gaps in the original design; they are documented in ISA.md and ABI.md.

1. `halt` counts as a step and a cycle; `pc` stays on the `halt` word.
2. Budget: before each instruction, `cycles ≥ budget` → BUDGET_EXCEEDED. Faults and waiting cost
   nothing.
3. Order of data-access checks: range, alignment, segment, heap. `call` checks the target before the
   stack; `ret` the stack before the target. Exact stack-check table in ISA.md.
4. Heap block span: round_up(n, 8) + 8 bytes; the first round_up(n, 4) bytes allocated, the rest red
   zone; `FREE` marks the whole span freed; first fit over free and freed granules.
5. `WRITE_STR`: at most 4096 bytes before the terminating 0, every byte checked like `ldb`, the
   terminator is not written; `READ_CHAR` at the end of closed input returns −1 without a fault.
6. `RAND`: xorshift32 with shifts (13, 17, 5); r0 = new state.
7. Canonical disassembly: decimal for signed immediates and shift widths, `0x` upper-case hex for
   unsigned immediates, absolute jump targets (`0x` + 4 hex digits, decimal outside 0…0xFFFF).
8. Labels bind to the next emitted item after its alignment; `.align n` takes a power of two
   1…256; `.byte` −128…255; `.word` −2³¹…2³²−1.
9. `MnemonicFilter` compares canonical names (`je` counts as `jz`); `li` is filtered as `li`.
10. Fault code = 1-based position in the `fault.*` list; `StateView.fault_addr` = first numeric
    argument; `lvl.*` diagnostics use `file` = 2.
11. Handles: low 16 bits slot + 1, high 16 bits generation; invalid handles → 0xFFFFFFFF or
    `{"error":"abi.bad_handle"}`.
12. `test/index.js` exists so that `node --test test/` works on Node ≥ 21 (a directory argument is
    loaded as a module there).

## Test plan (no third-party crates)

| # | Test | State |
|---|---|---|
| 1 | `isa.rs` is the single source (assembler, disassembler, CPU, `isa-table` read from it) | table + generated doc + CI diff in place |
| 2 | ALU vectors `tests/vectors/alu.txt` incl. edge cases | 90 vectors; evaluation enabled in 1A |
| 3 | naive i64 ALU model vs. `cpu::alu`, 10⁶ random cases (SplitMix64) | 1A |
| 4 | exhaustive encoding over 2³² words (`#[ignore]`, release/CI) | done |
| 5 | text round trip `asm(dis(w)) == w` for random valid instructions and immediate limits | done (1B) |
| 6 | assembler golden tests `tests/asm/*.s` (`; expect-word:`, `; expect-error: key line:col`) | done (1B) |
| 7 | no panic on random bytes / corrupted sources (assembler, level parser, machine) | assembler and machine done; levels open |
| 8 | time-travel invariant `digest(goto(k)) == d_k` in random order, thinning ≥ 3×, with input while WAITING_INPUT and `set_reg` | done (2A) |
| 9 | every `fault.*` key has a triggering test (reads ERRORS.md via `include_str!`) | 1A (key list ↔ ERRORS.md already checked) |
| 10 | differential test native vs. WASM via `fuzz-gen` | done (3A): `tools/differential.mjs`, N = 50 locally, 300 in CI; long-running structured programs |
| 11 | `check-levels --strict --mutate levels/` | levels phase |
| 12 | Node: export list == `abi-exports.txt`, no imports, ≤ 300 KiB, JSON shapes, `bad_handle`, views after `memory.grow`, time slices | done (3A): `test/wasm/abi.test.mjs`, `test/wasm/vm.test.mjs` |

## Placeholders to remove later

- `crates/core/tests/alu_vectors.rs`: `#[ignore]` on `vectors_match_cpu_alu` (phase 1A).
- `.github/workflows/test.yml`: the `exhaustive` name filter on the `--ignored` run.
- `tools/build.mjs`: generating `dist/en/index.html` and `dist/sw.js` (web phase).
- GitHub Pages must be enabled (Settings → Pages → Source: GitHub Actions) before the first push
  to `main`, otherwise the pages workflow fails at `configure-pages`.

## How to continue

```sh
export PATH=/opt/homebrew/opt/rustup/bin:$PATH   # only with Homebrew's keg-only rustup
export CARGO_BUILD_JOBS=2                        # on small machines
cargo test --workspace
npm run check            # everything CI runs, with a summary
npm run build && npm start   # http://127.0.0.1:4860/stapelwerk/
```

Rules: no third-party crates, no npm packages, no `HashMap`/`HashSet`, no floats in the machine, no
panics from user input, code comments in English. After changing `isa.rs`, regenerate the table:
`cargo run -q -p stapelwerk-cli -- isa-table > docs/isa-table.md`.

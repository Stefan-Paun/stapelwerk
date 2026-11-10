# Status

**Phase 0 (foundation and contract): done.** Phase 1A (machine) and phase 1B (assembler and
disassembler): done.

## What exists

| Area | State |
|---|---|
| Workspace | `crates/core` (`stapelwerk-core`), `crates/wasm` (`stapelwerk-wasm`, cdylib), `crates/cli` (`stapelwerk-cli`, binary `stapelwerk`); Rust 1.99.1 pinned; zero third-party crates; release profile per spec |
| Instruction set | `crates/core/src/isa.rs` complete: `Reg`, `Cond`, `AluOp`, `Format`, `Shape`, `ImmKind`, `FlagEffect`, `OpInfo`, the opcode table `OPCODES`, `Instr`, `Mem`, `encode`, strict `decode`, `isa_table_markdown` |
| Shared contract | `consts.rs` (memory map, segments, colours, syscalls, limits, reserved names), `fault.rs` (all `fault.*` keys, codes, args, JSON), `diag.rs` (`Diagnostic`, `Arg`, all keys), `program.rs` (`Program`, `Symbol`, `LineEntry`), `json.rs` (writer), `rng.rs` (SplitMix64, xorshift32) |
| Placeholders | `level.rs` (data model done, parser open), `runner.rs` (result types and metrics done), `session.rs` (signatures) |
| Tests | 8 mandatory encoding vectors, random encode/decode round trip, random words, exhaustive 2³² test (`#[ignore]`, CI release), 90 ALU vectors in `tests/vectors/alu.txt` (parsed; evaluation `#[ignore]` until phase 1A), ERRORS.md ↔ key list, CLI tests, Node ABI tests |
| WebAssembly | exports only `abi_version()` = 1 and `memory`; no imports |
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
| 8 | time-travel invariant `digest(goto(k)) == d_k` in random order, thinning ≥ 3×, with input while WAITING_INPUT and `set_reg` | session phase |
| 9 | every `fault.*` key has a triggering test (reads ERRORS.md via `include_str!`) | 1A (key list ↔ ERRORS.md already checked) |
| 10 | differential test native vs. WASM via `fuzz-gen` | WASM ABI phase |
| 11 | `check-levels --strict --mutate levels/` | levels phase |
| 12 | Node: export list == `abi-exports.txt`, no imports, ≤ 300 KiB, JSON shapes, `bad_handle`, views after `memory.grow`, time slices | partly done (imports, size, version, exports ⊆ list) |

## Placeholders to remove later

- `crates/core/tests/alu_vectors.rs`: `#[ignore]` on `vectors_match_cpu_alu` (phase 1A).
- `.github/workflows/test.yml`: the `exhaustive` name filter on the `--ignored` run; the guards that
  skip `check-levels` (until `levels/manifest.txt` exists) and the fuzz job (until
  `test/fuzz/differential.mjs` exists).
- `test/wasm/abi.test.mjs`: `skip` on "exports match docs/abi-exports.txt exactly" (WASM ABI phase).
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

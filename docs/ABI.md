# WebAssembly ABI v1

The browser talks to the Rust core through a small, stable WebAssembly interface. This document is
the contract between `crates/wasm` (the only crate with `unsafe` code) and `web/js/vm.js` (the only
JavaScript module that touches WebAssembly). The complete list of exports is
[abi-exports.txt](abi-exports.txt); a Node test compares it with the built module.

## Module

- Target `wasm32-unknown-unknown`, crate type `cdylib`, **no imports** (no WASI, no JS glue).
- Exports are plain functions declared `#[unsafe(no_mangle)] pub extern "C"` and take and return only
  `i32`/`u32`. The module also exports its `memory`. Linker-generated globals whose names start with
  `__` (if any) are not part of the ABI.
- Built with `opt-level = "s"`, LTO, `panic = "abort"`; the `.wasm` must stay at or below 300 KiB.
  A Rust panic becomes a WebAssembly trap; `vm.js` then discards the instance (see below).
- `abi_version()` returns `1`. `vm.js` refuses any other value.

## Conventions

**Handles.** Levels and sessions are referred to by handles: opaque non-zero `u32` values, never
`0xFFFFFFFF`. Internally they live in a table in a `thread_local!` `RefCell` and combine a slot index
with a generation counter (low 16 bits: slot + 1, high 16 bits: generation), so a freed handle stays
invalid even if its slot is reused. With an invalid handle, every function that returns a status,
pointer, count or flag returns `0xFFFFFFFF` (−1 as i32); functions that return a JSON length write
`{"error":"abi.bad_handle"}` to the output buffer and return its length; functions without a result
do nothing. Out-of-range arguments are handled the same way with `abi.bad_arg`, and session
functions that need a program before `sess_assemble` succeeded report `abi.not_assembled` (JSON) or
status 6 (NOT_ASSEMBLED).

**Input buffers.** To pass a string, JavaScript calls `buf_alloc(len)` to get a pointer, writes the
UTF-8 bytes into `memory` at that pointer, calls the function with `(ptr, len)`, then calls
`buf_free(ptr, len)`. Invalid UTF-8 is replaced by U+FFFD before parsing.

**Output buffer.** Functions with variable-size results return the result's **length in bytes**; the
bytes are in a global output buffer at `out_ptr()` (length also in `out_len()`), valid until the
next call into the module. Results are UTF-8 JSON unless stated otherwise.

**Stable views.** `sess_view`, `sess_mem`, `sess_shadow`, `sess_out_kinds` and `sess_out_vals`
return pointers that stay valid for the lifetime of the session: restoring snapshots copies into
the existing buffers. They can still be invalidated by `memory.grow` (the `ArrayBuffer` of
`memory.buffer` is replaced), so `vm.js` recreates its typed-array views whenever `memory.buffer`
changed after an export call.

## Exports

| Export | Signature → result | Meaning |
|---|---|---|
| `abi_version` | () → u32 | always 1 |
| `buf_alloc` | (len) → ptr | allocate an input buffer (0 if `len` is 0 or allocation fails) |
| `buf_free` | (ptr, len) | free an input buffer |
| `out_ptr` | () → ptr | address of the output buffer |
| `out_len` | () → u32 | length of the last result in the output buffer |
| `segments` | () → len | JSON memory map |
| `level_load` | (ptr, len) → lv | parse a `.lvl` file; 0 on error, then the output buffer holds `{"ok":false,"diagnostics":[…]}` |
| `level_free` | (lv) | release a level |
| `level_info` | (lv) → len | JSON level summary |
| `level_section` | (lv, name_ptr, name_len) → i32 | raw text of a section (`start`, `text.de`, `image.NAME`, …) in the output buffer; −1 if missing |
| `sess_new` | (lv) → s | new session for a level; `lv = 0` creates a sandbox session (no tests, no library) |
| `sess_free` | (s) | release a session |
| `sess_assemble` | (s, ptr, len) → u32 | assemble the player's code; returns the number of errors; JSON `{ok, size, diagnostics}` |
| `sess_reset` | (s, test: i32) → status | fresh machine for test `test`, or −1 for an interactive run |
| `sess_step` | (s, n) → status | execute up to `n` instructions, ignoring breakpoints |
| `sess_run` | (s, max_cycles) → status | run one time slice of at most `max_cycles` cycles; stops at breakpoints |
| `sess_back` | (s, n) → status | go back `n` steps (= `goto(steps − n)`, not below 0) |
| `sess_goto` | (s, step) → status | restore the state after exactly `step` steps |
| `sess_back_to_change` | (s, kind, target) → status | go to just before the last change of a register/word/byte |
| `sess_set_reg` | (s, reg, value) → status | edit a register while paused |
| `sess_bp` | (s, addr, on) → 1/0 | set (`on = 1`) or clear a breakpoint; 1 on success, 0 if `addr` is not an instruction address |
| `sess_bp_clear` | (s) | clear all breakpoints |
| `sess_in_int` | (s, v: i32) → status | append a number to the interactive input |
| `sess_in_text` | (s, ptr, len) → status | append bytes to the interactive text input |
| `sess_in_close` | (s) → status | close the interactive input |
| `sess_view` | (s) → ptr | pointer to the `StateView` (stable) |
| `sess_mem` | (s) → ptr | pointer to the 65536 memory bytes (stable) |
| `sess_shadow` | (s) → ptr | pointer to the 4096-byte heap shadow (stable) |
| `sess_out_kinds` | (s) → ptr | pointer to `u8[8192]` output kinds (stable) |
| `sess_out_vals` | (s) → ptr | pointer to `i32[8192]` output values (stable) |
| `sess_fault` | (s) → len | `{key, pc, step, args}` or `null` |
| `sess_symbols` | (s) → len | `[{name, kind, value, file, line}]` |
| `sess_linemap` | (s) → len | `[{a, f, l}]` (address, file, line) |
| `sess_line_counts` | (s) → len | `[[line, count], …]` for file 0 |
| `sess_frames` | (s) → len | `[{fp, ret, func, size}]`, innermost first |
| `sess_heap` | (s) → len | `[{addr, size, live, line}]` |
| `sess_disasm` | (s, addr, count) → len | `[{addr, word, text, label}]` |
| `sess_check` | (s) → len | evaluate the current test; same shape as one entry of `run_tests` |
| `sess_run_tests` | (s) → len | run all tests (see below) |
| `sess_reference_screen` | (s, test) → ptr | 4096 bytes: the reference solution's screen after `test` (native/unpacked levels only; 0 if unavailable) |
| `sess_digest` | (s) → len | 16 lower-case hex characters (raw ASCII, not JSON) |

Functions without a listed result return nothing. `sess_in_int`, `sess_in_text` and
`sess_in_close` return the current status so the UI can see whether a waiting program can continue.

## Status codes

| Code | Name | Meaning |
|---|---|---|
| 0 | PAUSED | ready; the next instruction is at `pc` |
| 1 | HALTED | `halt` executed (function tests: `__trap` reached) |
| 2 | FAULT | stopped by a fault; see `sess_fault` |
| 3 | BREAKPOINT | stopped before an instruction with a breakpoint |
| 4 | WAITING_INPUT | waiting for interactive input |
| 5 | BUDGET_EXCEEDED | the cycle budget is used up |
| 6 | NOT_ASSEMBLED | no successfully assembled program |
| 0xFFFFFFFF | – | handle or argument error |

## StateView

`#[repr(C)]`, all fields `u32` little-endian, exactly 96 bytes (checked at compile time with
`const _: () = assert!(size_of::<StateView>() == 96)`). Updated after every export call that changes
the session.

| Offset | Field | Meaning |
|---|---|---|
| 0 | `status` | status code |
| 4 | `pc` | program counter |
| 8–47 | `regs[10]` | `r0`–`r7`, `fp`, `sp` |
| 48 | `flags` | Z, N, C, V in bits 0–3 |
| 52 | `steps` | completed instructions (saturating at 2³²−1) |
| 56 | `cycles` | consumed cycles (saturating) |
| 60 | `fault_code` | 0 = none, else the fault code below |
| 64 | `fault_addr` | first numeric fault argument (`word`, `target`, `addr`, `sp` or `n`), else 0 |
| 68 | `out_len` | visible length of the output tape |
| 72 | `in_int_pos` | numbers consumed |
| 76 | `in_int_len` | numbers available |
| 80 | `in_text_pos` | text bytes consumed |
| 84 | `in_text_len` | text bytes available |
| 88 | `test_index` | current test, −1 = interactive (as i32) |
| 92 | `budget` | cycle budget of the current run |

**Fault codes** (`fault_code`): the 1-based position of the key in the `fault.*` list of
[ERRORS.md](ERRORS.md):

| Code | Key | Code | Key |
|---|---|---|---|
| 1 | `fault.illegal_instruction` | 13 | `fault.stack_misaligned` |
| 2 | `fault.zero_instruction` | 14 | `fault.div_zero` |
| 3 | `fault.exec_outside_code` | 15 | `fault.input_end` |
| 4 | `fault.bad_jump_target` | 16 | `fault.heap_unallocated` |
| 5 | `fault.bad_return_address` | 17 | `fault.heap_overflow` |
| 6 | `fault.null_access` | 18 | `fault.use_after_free` |
| 7 | `fault.out_of_range` | 19 | `fault.invalid_free` |
| 8 | `fault.guard_page` | 20 | `fault.double_free` |
| 9 | `fault.write_to_code` | 21 | `fault.unknown_syscall` |
| 10 | `fault.misaligned` | 22 | `fault.string_too_long` |
| 11 | `fault.stack_overflow` | 23 | `fault.output_limit` |
| 12 | `fault.stack_underflow` | | |

**Heap shadow** values (`sess_shadow`, one byte per 4-byte granule of 0x8000…0xBFFF): 0 free,
1 allocated, 2 freed, 3 red zone.

**Output kinds** (`sess_out_kinds`): 0 number, 1 byte. Only the first `out_len` entries are valid.

## JSON shapes

All numbers are JSON integers (addresses as unsigned decimal). Arguments are objects keyed by the
argument names of [ERRORS.md](ERRORS.md).

```jsonc
// segments
[{"name":"null","start":0,"end":256,"r":false,"w":false,"x":false}, …]

// sess_assemble
{"ok":false,"size":0,"diagnostics":[{"key":"asm.unknown_mnemonic","file":0,"line":3,"col":9,"len":2,"args":{"name":"ad","suggestion":"add"}}]}

// level_load on error
{"ok":false,"diagnostics":[{"key":"lvl.missing_key","file":2,"line":1,"col":1,"len":0,"args":{"key":"id"}}]}

// level_info
{"id":"2-2-bis-zur-null","version":1,"chapter":2,"order":2,"kind":"normal",
 "title":{"de":"Bis zur Null","en":"Until Zero"},"card":{"de":"…","en":"…"},
 "budget":100000,"seed":2202,"mobile":false,"heap_check":"strict",
 "allow":null,"forbid":null,
 "tests":[{"index":0,"name":"grundfall","in":[3,4,5,0],"in_text":"","out":[12],"out_text":null,
           "regs":[],"set":[],"call":null,"expect_reg":[],"expect":[],
           "screen_init":null,"expect_screen":null,"heap_empty":false}, …],   // visible tests only
 "hidden_tests":1,"ref_cycles":null,"ref_size":null,
 "sections":["text.de","text.en","hint.de","hint.en","learned.de","learned.en","start"]}

// sess_fault
{"key":"fault.null_access","pc":260,"step":7,"args":{"addr":8,"access":"w"}}   // or null

// sess_symbols
[{"name":"schleife","kind":"code","value":260,"file":0,"line":3}]   // kind: code | data | const

// sess_linemap
[{"a":256,"f":0,"l":2}]

// sess_line_counts
[[2,1],[3,5]]

// sess_frames
[{"fp":61424,"ret":284,"func":"fak","size":16}]   // func may be null

// sess_heap
[{"addr":32768,"size":8,"live":true,"line":12}]   // line of the ALLOC in file 0, 0 if unknown

// sess_disasm
[{"addr":256,"word":285212673,"text":"mov r1, 0","label":"main"}]   // label may be null; words that do not decode: text ".word 0x…"

// sess_check: one test entry (below)

// sess_run_tests
{"passed":false,
 "tests":[{"index":0,"name":"grundfall","hidden":false,"passed":true,"status":1,"cycles":31,"steps":12,"fault":null,"mismatch":null,
           "input":{"in":[3,4,5,0],"in_text":"","out":[12],"out_text":null}}],
 "metrics":{"cycles_avg":31,"cycles_max":40,"steps_avg":12,"size":9},
 "reference":{"cycles":31,"size":9}}   // reference may be null
```

`mismatch` is `{"key":"check.…","args":{…}}` or `null`.

**Budget end** (*extension of v1*, additive, `abi_version()` stays 1). A test that ran out of
cycles has `"status":5` (BUDGET_EXCEEDED), `"fault":null`, `cycles` ≥ the test's budget, and a
`check.not_halted` mismatch with three extra arguments, so the UI can say "cycle budget used up –
line 7 ran 9 999 times" without another call:

```jsonc
{"index":1,"name":"ungerade","hidden":false,"passed":false,"status":5,"cycles":10000,"steps":4999,"fault":null,
 "mismatch":{"key":"check.not_halted","args":{"status":"BUDGET_EXCEEDED","cycles":10000,"hottest_line":8,"hottest_count":1666}}}
```

`cycles` repeats the test's cycle count, `hottest_line` is the most executed line of the player's
file (file 0; 0 if no player line ran, e.g. a loop in the library; smallest line on ties) and
`hottest_count` how often it ran (the same numbers as `sess_line_counts` after running that test to
its end). Other non-`HALTED` endings keep `{"status":"…"}` only. `sess_check` returns the same
entry. The optional `input` object (`in`,
`in_text`, `out`, `out_text`; the last two `null` when the test does not check them) is always
present for visible tests; for hidden tests it is present only when the test failed, so the player
can reproduce the failure.

**Packed levels.** The web build packs levels with `stapelwerk pack-levels`: `[reference]` is
removed, `ref_cycles`/`ref_size` are written into `[level]`, and `expect_screen = reference` is
turned into an `[image.__ref_TESTNAME]` section with the test switched to that image. Therefore
`sess_reference_screen` returns 0 in the browser.

**Test details in `level_info`** (*extension of v1*, additive, `abi_version()` stays 1). Every
entry of `tests` (visible tests only; hidden tests are only counted in `hidden_tests`) carries what
goes in and what is expected, so the UI can show it before anything runs:

| Field | Value |
|---|---|
| `in` | numbers of `in` (`[]` if none) |
| `in_text` | `in_text` as a string (`""` if none) |
| `out` / `out_text` | expected number output / text output; `null` = not checked |
| `regs` / `expect_reg` | `[{"reg":"r1","value":-2}]` (`reg rX = v` / `expect_reg rX = v`; values signed) |
| `set` / `expect` | `[{"addr":16384,"words":[1,-1]}]` for `set`/`expect`, `[{"addr":"DATA","bytes":[97,98]}]` for `setb`/`expectb`; `addr` is a number (also for built-in constants with an optional offset, e.g. `INPUT + 4` → `24580`, which need no program), otherwise the address as text when it names a program symbol: `"DATA"`, `"DATA + 4"`, `"DATA - 4"` (the level alone cannot resolve program symbols); words signed, bytes 0…255 |
| `call` | label of a function test or `null` |
| `screen_init` | image name or `null` |
| `expect_screen` | `"reference"`, an image name or `null`; packed levels name the image `__ref_TESTNAME` |
| `heap_empty` | `expect_heap_empty` |

Images are read with `level_section("image.NAME")` (rows of `.`/hex digits; `vm.js`:
`level.image(name)` → `Uint8Array(4096)`).

## Session semantics

A session holds the level, the assembled program, a `MachineState` and the tapes, an edit log,
snapshots, breakpoints and line counters.

- **`MachineState`** (cloned into snapshots): registers, flags, pc, 64 KiB memory, heap manager and
  shadow, RNG, counters, input read positions and the **length** of the output tape. The tapes
  themselves only grow.
- **Snapshots** every K steps (K starts at 1024). With more than 64 snapshots every second one is
  dropped and K doubles.
- **`goto(t)`** restores the snapshot with the largest step ≤ t and replays deterministically up to
  t (without breakpoints), applying the edit log at its steps. `back(n)` = `goto(steps − n)`, not
  below 0.
- Restoring **copies into the existing buffers** (stable pointers for JavaScript).
- Going back truncates the visible output to the saved length; input is only ever appended.
- **`set_reg(reg, value)`** only while paused: the future is discarded (snapshots after now, later
  edits) and the edit is logged at the current step.
- **`back_to_change(kind, target)`**: kind 0 = register (index), 1 = memory word (address),
  2 = memory byte. Jumps to the state **directly before** the last step < now that changed the value;
  if there is none, to step 0.
- **Breakpoints:** a run stops **before** an instruction with a breakpoint, except for the first
  instruction of the call. `step(1)` ignores breakpoints.
- **Line counters:** executions per source line of file 0 since the reset (recomputed or saved
  across `goto`).
- **`digest()`:** FNV-1a-64 over the whole state (registers, flags, pc, memory, heap, counters,
  output up to its length, read positions), printed as 16 hex characters. Native and WebAssembly
  builds must produce identical digests (differential test).
- **Function tests** (`call = LABEL`): fresh machine; `r0`–`r3` from the test's `reg` lines (others
  0); `r4`–`r7` and `fp` get random values from the test seed (`fp ≠ 0`); `__trap` is pushed as the
  return address; `pc = LABEL`; the run ends when `pc` reaches `__trap` (status HALTED). Then
  `r4`–`r7`, `fp` and `sp` must equal their values before the call (`check.abi_clobbered`), followed
  by the test's other expectations.

## `web/js/vm.js` contract

`vm.js` is the only JavaScript module that touches WebAssembly.

- `loadVm(urlOrBytes)` instantiates the module (in the browser with
  `WebAssembly.instantiateStreaming`, falling back to `arrayBuffer()`; under Node from bytes),
  checks `abi_version() === 1`, and returns the VM object.
- Class `Session` with the methods `assemble`, `reset`, `step`, `run`, `back`, `goto`,
  `backToChange`, `setReg`, `setBreakpoint`, `clearBreakpoints`, `pushInt`, `pushText`,
  `closeInput`, `state`, `memory`, `shadow`, `output`, `fault`, `symbols`, `lineMap`, `lineCounts`,
  `frames`, `heap`, `disasm`, `check`, `runTests`, `digest`, `free`.
- After every export call it recreates its typed-array views if `memory.buffer` changed.
- If an export traps, the instance is unusable: `vm.js` creates a new instance and reports an
  "internal error" to the UI.

*Clarification* (phase 3A, as implemented in `web/js/vm.js`):

- `loadVm` also accepts an `ArrayBuffer`, a typed array or a `WebAssembly.Module`; a `file:` URL is
  read with `node:fs` (Node's `fetch` cannot read files). A wrong version rejects with
  `VmError('abi.version')`.
- The VM object: `mock` (`false`), `abiVersion`, `segments()`, `loadLevel(text)` → `Level` or
  `null` (then `levelDiagnostics` holds the `lvl.*` diagnostics), `newSession(level | null)`
  (`null` = sandbox), `onInternalError` (optional callback). Class `Level`: `info()` (the
  `level_info` object), `section(name)` (text or `null`), `image(name)` (`Uint8Array(4096)` of
  colour indices or `null`), `free()`.
- Additional `Session` methods: `horizon()` (furthest step reached on the current timeline, kept in
  JavaScript) and `referenceScreen(test)` (`Uint8Array(4096)` or `null`): for a visible test the
  image named by its `expect_screen` (in packed levels `__ref_NAME`), for `reference` the screen
  from `sess_reference_screen` (unpacked levels), `null` without a screen expectation; for hidden
  tests `sess_reference_screen` if available.
- Status results are unsigned numbers (`0xFFFFFFFF` for handle/argument errors). `memory()` and
  `shadow()` return live views; fetch them again after every call. `output()` returns
  `[{kind, value}]` for the visible entries. `state()` decodes the `StateView` with the field
  names `status, pc, regs, flags, steps, cycles, faultCode, faultAddr, outLen, inIntPos, inIntLen,
  inTextPos, inTextLen, testIndex, budget`.
- JSON results `{"error":"abi.…"}` are thrown as `VmError` with `key` set to the error key, except
  `abi.not_assembled` from `check()` (→ `null`) and `runTests()` (→
  `{passed: false, tests: [], metrics: null, reference: null}`). Using a freed `Session` or `Level`
  throws `VmError('abi.bad_handle')`.
- After a trap `vm.js` instantiates the module again, reloads every live level from its text and
  every session (new handle, last successfully assembled source, test index, breakpoints; the run
  starts over at step 0, interactive input is lost), calls `onInternalError(error)` and throws
  `VmError('internal')`. A level or source that traps again during the reload is dropped.

## Clarifications made in phase 3A

These describe the behaviour of `crates/wasm` where the sections above leave room.

1. **One handle table** holds levels and sessions, so passing a level handle to a `sess_*` function
   (or a session handle to a `level_*` function) is `abi.bad_handle`. At most 65534 handles are live
   at a time; `sess_new` returns `0xFFFFFFFF` for an invalid level handle or a full table, and
   `level_load` returns 0 with `{"error":"abi.bad_arg"}` when the table is full.
2. **Input buffers:** a `(ptr, len)` argument must lie inside a buffer from `buf_alloc` that has not
   been freed (`len = 0` is always valid). Otherwise the function reports `abi.bad_arg` (JSON
   functions), `0xFFFFFFFF` (`sess_assemble`, `sess_in_text`), −1 (`level_section`) or 0
   (`level_load`). `buf_free` with a pair that `buf_alloc` did not return is ignored. The bytes are
   copied before use, so the buffer can be freed right after the call.
3. `level_load` leaves the output buffer unchanged on success. `level_section` never returns
   `reference`; `image.NAME` returns the image as rows (`.` or a hex digit per pixel, trailing `.`
   and empty trailing rows omitted, joined with `\n`).
4. A level freed with `level_free` stays alive inside every session created from it.
5. `sess_assemble` returns `0xFFFFFFFF` for an invalid handle or buffer (the output buffer then
   holds the error object). On success the session is reset for an interactive run (test −1, input
   open) and its breakpoints are cleared; on failure the old program is dropped.
6. `sess_reset` returns `0xFFFFFFFF` for a test index ≥ the number of tests, < −1, ≥ 0 in a sandbox
   session, and when the test cannot be set up (a `set`/`call` symbol the program does not define,
   a `set` beyond 0xFFFF); the session is then unchanged. Without a program it returns 6.
7. `sess_set_reg` returns `0xFFFFFFFF` for `reg > 11` and when the run is over (HALTED, FAULT,
   BUDGET_EXCEEDED); `sess_back_to_change` for `kind > 2`, a register index above 11, a word address that is not 4-aligned or
   above 0xFFFC, or a byte address above 0xFFFF; `sess_bp` for `on > 1`.
8. `sess_disasm` reports `abi.bad_arg` for `addr ≥ 0x10000` or `count > 4096`.
9. `sess_view` is refreshed after **every** `sess_*` call, so it is current whenever JavaScript
   reads it.
10. `sess_check` runs the current test (the one of the last `sess_reset`) through the test runner
    on a fresh machine, exactly like its entry in `sess_run_tests`; register edits and the debugger
    position do not influence it. It writes `null` for an interactive run and in a sandbox session,
    and `abi.not_assembled` without a program.
11. `sess_run_tests` in a sandbox session writes
    `{"passed":false,"tests":[],"metrics":{…},"reference":null}`. For unpacked levels the reference
    solution is run once per level (on first use) and cached for `reference`, for
    `expect_screen = reference` and for `sess_reference_screen`.
12. `sess_reference_screen` returns `0xFFFFFFFF` for a test index outside the level; the pointer stays
    valid as long as the session exists.
13. `sess_digest` with an invalid handle writes `{"error":"abi.bad_handle"}` (not 16 characters).

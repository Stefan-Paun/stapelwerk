# Error keys

Every error and status message in Stapelwerk is identified by a **key** with named **arguments**.
The core (Rust) only produces keys and arguments; all human-readable texts live in the UI's
translation files (`web/js/texts/de.js`, `web/js/texts/en.js`), so the same key can be shown in
German and English.

Format of the list below (parsed by a test in `crates/core/src/diag.rs`; keep it exact):

```
- `family.name` {arg1, arg2?} – description
```

Arguments are listed in output order; `?` marks an optional argument; the braces are omitted when
a key has no arguments. Numbers are JSON integers, strings are JSON strings. The list must contain
exactly the keys of `diag::ALL_KEYS`, in the same order.

## Runtime faults (`fault.*`)

A fault stops the machine. Faults are **precise**: registers, memory, flags, counters and output are
unchanged, and `pc` points at the faulting instruction. Every fault additionally carries `pc` and
`step` (number of instructions completed before it) outside `args`:
`{"key":"fault.div_zero","pc":268,"step":41,"args":{}}`. The fault code in `StateView.fault_code` is
the 1-based position in this list.

- `fault.illegal_instruction` {word} – The fetched word is not a valid instruction: unassigned opcode, a non-zero unused field, register code 10–15 where not allowed, shift immediate above 31, or condition 15. `word` is the raw word.
- `fault.zero_instruction` – The fetched word is 0. The unused part of the text segment is zero, so this usually means a missing `halt` ("forgot halt?").
- `fault.exec_outside_code` – `pc` lies outside the occupied part of the text segment.
- `fault.bad_jump_target` {target} – A jump or call target (`jmp`, `j<cc>`, `call`, `jmp rs`, `call rs`) is not a multiple of 4 or lies outside the code. Reported at the jumping instruction.
- `fault.bad_return_address` {target} – `ret` popped an address that is not a multiple of 4 or lies outside the code (typically an unbalanced stack). Reported at the `ret`.
- `fault.null_access` {addr, access} – Data access to 0x0000…0x00FF. `access` is `"r"` or `"w"`.
- `fault.out_of_range` {addr, access} – The effective address is 0x10000 or higher (base + offset is computed modulo 2³²).
- `fault.guard_page` {addr, access} – Data access to the guard segment 0xC000…0xCFFF between heap and stack.
- `fault.write_to_code` {addr} – Store into the text segment (code is never writable).
- `fault.misaligned` {addr, size} – Word access (`ld`, `st`) at an address that is not a multiple of 4; `size` is 4.
- `fault.stack_overflow` {sp} – `push` or `call` would store below 0xD000. `sp` is the value before the instruction.
- `fault.stack_underflow` {sp} – `pop` or `ret` would read at or above 0xF000. `sp` is the value before the instruction.
- `fault.stack_misaligned` {sp} – `sp` is not a multiple of 4 at `push`, `pop`, `call` or `ret`.
- `fault.div_zero` – `div`, `divu`, `rem` or `remu` with divisor 0.
- `fault.input_end` – `READ_INT` found no number left (test mode; input is closed).
- `fault.heap_unallocated` {addr} – Access to heap memory that was never allocated (`heap_check = strict`).
- `fault.heap_overflow` {addr} – Access to the red zone behind a heap block (`heap_check = strict`).
- `fault.use_after_free` {addr} – Access to a freed heap block (`heap_check = strict`).
- `fault.invalid_free` {addr} – `FREE` with an address that is not the start of a heap block.
- `fault.double_free` {addr} – `FREE` of a block that is already freed.
- `fault.unknown_syscall` {n} – `sys n` with a number that is not a system call.
- `fault.string_too_long` {addr} – `WRITE_STR` found no terminating 0 within 4096 bytes from `addr`.
- `fault.output_limit` – The output tape already holds 8192 entries.

## Run status (`run.*`)

Not a fault: the program did not finish within its cycle budget (status `BUDGET_EXCEEDED`).

- `run.budget_exceeded` {cycles, hottest_line} – The cycle budget is used up after `cycles` cycles. `hottest_line` is the line of the player's file executed most often (0 if none), usually the loop that does not end.

## Assembler (`asm.*`)

Each diagnostic has `{key, file, line, col, len, args}`: `file` 0 = player, 1 = level library;
`line` and `col` are 1-based; `col` and `len` count UTF-16 code units.

- `asm.unknown_mnemonic` {name, suggestion?} – `name` is not an instruction, pseudo-instruction or directive. `suggestion` is the closest known mnemonic within edit distance 2 (Damerau-Levenshtein, a swap of two neighbouring letters counts as 1), if any.
- `asm.unknown_register` {name} – A register operand is expected but `name` is not `r0`…`r7`, `fp` or `sp`.
- `asm.operand_count` {expected, got} – Wrong number of operands; `expected` describes the accepted forms (e.g. `"2-3"`).
- `asm.operand_type` {expected} – An operand has the wrong kind or is missing; `expected` is `"register"`, `"immediate"`, `"memory"`, `"label"` (a jump target, also for a target that is not a multiple of 4 away, or the name of `.equ`) or `"string"` (`.string`, `.ascii`).
- `asm.imm_range` {value, min, max, hint?} – An immediate, offset, data value, `.space`/`.align` argument or jump target is outside its range (for jumps, `value`, `min` and `max` are absolute addresses). `hint` is `"li"` for `mov rd, imm`, so the UI can suggest `li`.
- `asm.undefined_symbol` {name} – A label or constant is used but never defined.
- `asm.duplicate_label` {name, first_line} – A label or constant is defined twice (player and library share one namespace).
- `asm.reserved_name` {name} – A register name, mnemonic, syscall name, built-in constant or a name starting with `__` is used as a label or constant.
- `asm.bad_number` – A malformed number literal (e.g. `0x`, `0b2`, a misplaced `_`, or a value beyond i64).
- `asm.bad_string` – A malformed string or character literal (unknown escape, missing closing quote, character literal with not exactly one character).
- `asm.unexpected_char` {ch} – A character that cannot start a token.
- `asm.local_label_without_global` – A local label `.name` appears before any global label.
- `asm.needs_constant` – The expression must be known at this point (`.equ`, `.space`, `.align`, data directives) but uses a label or a constant defined later.
- `asm.expr_div_zero` – Division or remainder by zero in a constant expression.
- `asm.expr_overflow` – A constant expression overflows i64.
- `asm.data_in_text` – A data directive (`.word`, `.byte`, `.string`, `.ascii`, `.space`) in the `.text` section.
- `asm.code_in_data` – An instruction in the `.data` section.
- `asm.section_overflow` {section, size, max} – The section is larger than its segment (`text`: 16128 bytes including library and trap; `data`: 8192 bytes).
- `asm.forbidden_instruction` {mnemonic} – The level does not allow this mnemonic (`allow` / `forbid`).
- `asm.too_many_errors` – More than 50 diagnostics; the assembler stopped reporting.

## Level files (`lvl.*`)

Reported for `.lvl` files with `file` = 2.

- `lvl.unknown_section` {name} – A section header names an unknown section.
- `lvl.unknown_key` {key} – Unknown key in `[level]` or `[test]`.
- `lvl.missing_key` {key} – A required key is missing.
- `lvl.bad_value` {key} – The value of `key` cannot be parsed or is out of range.
- `lvl.duplicate_section` – A section that may appear only once appears twice.
- `lvl.no_tests` – The level has no `[test]` section.
- `lvl.allow_and_forbid` – `allow` and `forbid` are both given.
- `lvl.bad_image` {name, line} – Image `name` has more than 64 rows or columns or an invalid character in row `line`.

## Test comparison (`check.*`)

The `mismatch` of a test result that ran without a fault but did not pass.

- `check.out_mismatch` {index, expected, actual} – Output entry `index` (0-based) differs.
- `check.out_missing` {index, expected} – The output ended before entry `index`.
- `check.out_extra` {index, actual} – The output has more entries than expected, starting at `index`.
- `check.text_mismatch` {offset, expected, actual} – Text output differs at byte `offset` (values are byte values, −1 for "missing").
- `check.mem_mismatch` {addr, expected, actual} – Memory differs at `addr` (word or byte, as in the test line).
- `check.reg_mismatch` {reg, expected, actual} – Register `reg` (name) has the wrong value.
- `check.screen_mismatch` {x, y, expected, actual, count} – The screen differs; first wrong pixel at (`x`, `y`), `count` wrong pixels in total (low 4 bits compared).
- `check.heap_not_empty` {count, alloc_line} – `count` heap blocks are still allocated; `alloc_line` is the player's line of the first leaked `ALLOC` (0 if unknown).
- `check.abi_clobbered` {reg, before, after} – A function test changed a callee-saved register (`r4`–`r7`, `fp`) or did not restore `sp`.
- `check.not_halted` {status, cycles?, hottest_line?, hottest_count?} – The run ended with a status other than `HALTED` (e.g. `"WAITING_INPUT"`). For `"BUDGET_EXCEEDED"` the test runner adds `cycles` (cycles used), `hottest_line` (line of the player's file executed most often, 0 if none) and `hottest_count` (how often that line ran).

## WASM ABI (`abi.*`)

Returned as `{"error":"abi.bad_handle"}` by JSON-returning exports.

- `abi.bad_handle` – Unknown or stale level or session handle.
- `abi.bad_arg` – An argument is out of range (e.g. a buffer outside memory, an unknown kind).
- `abi.not_assembled` – The session has no successfully assembled program.

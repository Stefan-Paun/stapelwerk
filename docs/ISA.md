# The Stapelwerk machine

This is the normative description of the Stapelwerk teaching CPU and its assembly language.
The opcode table in `crates/core/src/isa.rs` is the single source of truth for opcodes, formats,
immediate kinds, cycle costs and flag effects; [isa-table.md](isa-table.md) is generated from it.
Where this document says *clarification*, it pins down a detail that the original design left
open; implementations must follow it.

Contents: [Overview](#overview) · [Registers](#registers) · [Flags](#flags) ·
[Encoding](#encoding) · [Instructions](#instructions) · [Conditions](#jump-conditions) ·
[Semantics](#semantics) · [Canonical text form](#canonical-text-form) ·
[Memory map](#memory-map) · [Screen](#screen) · [System calls](#system-calls) · [Heap](#heap) ·
[Calling convention](#calling-convention) · [Cycles and budget](#cycles-and-budget) ·
[Assembly language](#assembly-language)

## Overview

- 32-bit words, two's complement, little-endian memory.
- 10 registers (`r0`–`r7`, `fp`, `sp`), a program counter that is not a register operand, and four
  flags Z, N, C, V.
- Fixed 32-bit instructions in four formats, with strict decoding: every valid word has exactly one
  meaning and one canonical text form.
- 64 KiB of memory in eight segments with fixed rights (null, text, data, input, heap, guard,
  stack, screen). Code is never writable; data is never executable.
- A 64×64 pixel screen mapped into memory, eight system calls for I/O, heap and random numbers.
- Every fault is precise and has a named key (see [ERRORS.md](ERRORS.md)).
- Everything is deterministic: no floating point, no clocks, randomness only from seeds.

## Registers

| Code (4 bit) | Name | Role |
|---|---|---|
| 0–3 | `r0`–`r3` | arguments and return values, **caller-saved** |
| 4–7 | `r4`–`r7` | **callee-saved** |
| 8 | `fp` | frame pointer, callee-saved |
| 9 | `sp` | stack pointer |
| 10–14 | – | invalid → `fault.illegal_instruction` |
| 15 | (abs) | only in field B of memory instructions and `lea`: base 0 with an unsigned offset (absolute address 0…0xFFFF) |

The program counter `pc` cannot be named as an operand.

**Initial state:** all registers 0, `sp = 0xF000`, `fp = 0` (end of the frame chain),
`pc = 0x0100`, flags 0.

## Flags

The flag word holds Z, N, C, V in bits 0, 1, 2, 3.

| Instruction | Z, N | C | V |
|---|---|---|---|
| `add` | from result | carry out of bit 31 | signed overflow |
| `sub`, `cmp`, `neg` (= 0 − x) | from result | **borrow**: a <ᵤ b (x86 convention) | signed overflow |
| `mul` | from result | product does not fit in u32 | product does not fit in i32 |
| `div`, `rem` | from result | 0 | 1 exactly for INT_MIN / −1 |
| `divu`, `remu`, `and`, `or`, `xor`, `tst`, `not` | from result | 0 | 0 |
| `shl`, `shr`, `sar` | from result | last bit shifted out (0 for width 0) | 0 |
| all others (`mov`, `movhi`, `lea`, `ld*`, `st*`, `push`, `pop`, jumps, `call`, `ret`, `sys`, `nop`, `halt`) | unchanged | unchanged | unchanged |

Z is set when the 32-bit result is 0; N is bit 31 of the result. Instructions that set flags always
write all four. For `cmp` the "result" is a − b and for `tst` it is a & b; neither writes a register.

For `mul`, "product" means the exact mathematical product: C compares the unsigned product of both
operands with 2³² − 1, V compares the signed product with the i32 range.

For shifts by k (1…31): `shl` shifts out bit 32 − k last, `shr` and `sar` shift out bit k − 1 last.

## Encoding

Every instruction is one 32-bit word, nibble-aligned, stored little-endian.

| Format | 31–24 | 23–20 | 19–16 | 15–12 | 11–0 |
|---|---|---|---|---|---|
| N | op | 0 | 0 | 0 | 0 |
| R | op | A | B | C | 0 |
| I | op | A | B | imm16 (bits 15–0) ||
| J | op | A (condition) | imm20 (bits 19–0, signed, in words relative to the address of this instruction) |||

### Strict decoding

Every valid word has exactly one textual form, so `encode(decode(w)) = w` for every valid word w.
A word is invalid (`fault.illegal_instruction {word}`) if any of these holds:

- the opcode is not assigned (see the table below; e.g. 0x00 with non-zero bits, 0x03, 0x2F, 0x3F);
- an unused field or unused bits are not 0 (for format N: bits 23–0; format R: bits 11–0 and, for
  instructions with fewer operands, the unused register fields; `sys`: fields A and B; `mov`
  immediate and `movhi`: field B; `call L`: field A; `cmp`/`tst`: field A);
- a register field holds 10–15, except 15 in field B of `lea`, `ld`, `ldb`, `ldbs`, `st`, `stb`;
- the immediate of `shl`, `shr`, `sar` (immediate form) is greater than 31;
- the condition field of opcode 0x60 is 15.

The word `0x00000000` is not "illegal" but `fault.zero_instruction` (the unused text segment is
zero, so this usually means a missing `halt`).

### Mandatory encoding test vectors

| Instruction | Word |
|---|---|
| `add r1, r2, r3` | `0x20123000` |
| `add r1, r2, -1` | `0x3012FFFF` |
| `ld r0, [fp - 8]` | `0x5008FFF8` |
| `ld r0, [0x4000]` | `0x500F4000` |
| `push fp` | `0x58800000` |
| `jz` three words back | `0x601FFFFD` |
| `sys WRITE_INT` | `0x04000002` |
| `halt` | `0x01000000` |

## Instructions

| Op | Assembly | Format | Semantics | Cycles |
|---|---|---|---|---|
| 01 | `halt` | N | status HALTED | 1 |
| 02 | `nop` | N | – | 1 |
| 04 | `sys NAME\|n` | I (A = B = 0, imm = n) | system call n ([System calls](#system-calls)) | 4 |
| 08 | `ret` | N | pc = pop() (target checked) | 2 |
| 10 | `mov rd, rs` | R (C = 0) | rd = rs | 1 |
| 11 | `mov rd, simm16` | I (B = 0) | rd = sext(imm) | 1 |
| 12 | `movhi rd, uimm16` | I (B = 0) | rd = (imm << 16) \| (rd & 0xFFFF) | 1 |
| 13 | `lea rd, [m]` | I | rd = effective address | 1 |
| 20–2E | `op rd, rs1, rs2` | R | ALU (table below) | 1; `mul` 3; `div`, `divu`, `rem`, `remu` 8 |
| 30–3E | `op rd, rs1, imm` | I | ALU with immediate | as above |
| 40 | `neg rd, rs` | R (C = 0) | rd = 0 − rs | 1 |
| 41 | `not rd, rs` | R (C = 0) | rd = ~rs | 1 |
| 50 | `ld rd, [m]` | I | load 32-bit word (address multiple of 4) | 2 |
| 51 | `ldb rd, [m]` | I | load byte, zero-extended | 2 |
| 52 | `ldbs rd, [m]` | I | load byte, sign-extended | 2 |
| 54 | `st rs, [m]` | I (A = source) | store 32-bit word | 2 |
| 55 | `stb rs, [m]` | I | store lowest byte | 2 |
| 58 | `push rs` | R (A only) | sp −= 4; mem[sp] = rs (for `push sp`: the old value) | 2 |
| 59 | `pop rd` | R (A only) | v = mem[sp]; sp += 4; rd = v | 2 |
| 60 | `jmp L` / `j<cc> L` | J | if the condition holds: pc = pc_this + 4 · sext(imm20) | 1 |
| 61 | `call L` | J (A = 0) | push(pc + 4); pc = target | 2 |
| 62 | `jmp rs` | R (A only) | pc = rs | 1 |
| 63 | `call rs` | R (A only) | push(pc + 4); pc = rs | 2 |

A conditional jump costs 1 cycle whether or not it is taken.

**ALU operations.** The low nibble selects the operation, identically for 0x2_ (register form) and
0x3_ (immediate form). Immediate kinds: s = −32768…32767 (sign-extended), u = 0…65535
(zero-extended), sh = 0…31.

| Nibble | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | A | B | C | D | E |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Instruction | `add` | `sub` | `mul` | `div` | `divu` | `rem` | `remu` | `and` | `or` | `xor` | `shl` | `shr` | `sar` | `cmp` | `tst` |
| Immediate | s | s | s | s | u | s | u | u | u | u | sh | sh | sh | s | u |

`cmp` and `tst` have A = 0 and only set flags: `cmp a, b` computes a − b, `tst a, b` computes a & b.
Their assembly form has two operands (`cmp rs1, rs2`, `cmp rs1, imm`).

**Memory operand `[m]`:** `[reg]`, `[reg + expr]`, `[reg - expr]` (field B = reg, offset
−32768…32767) or `[expr]` (field B = 15, absolute address 0…0xFFFF).
The effective address is base + sext(offset) modulo 2³²; an address of 0x10000 or more gives
`fault.out_of_range`. `lea` computes the same address without accessing memory and without the
range check.

## Jump conditions

Field A of opcode 0x60. Names follow x86.

| Code | Mnemonic (aliases) | Condition | Meaning after `cmp a, b` |
|---|---|---|---|
| 0 | `jmp` | always | |
| 1 | `jz` (`je`) | Z | a = b |
| 2 | `jnz` (`jne`) | !Z | a ≠ b |
| 3 | `jlt` | N ≠ V | a < b signed |
| 4 | `jge` | N = V | a ≥ b signed |
| 5 | `jle` | Z ∨ N ≠ V | a ≤ b signed |
| 6 | `jgt` | !Z ∧ N = V | a > b signed |
| 7 | `jb` | C | a < b unsigned |
| 8 | `jae` | !C | a ≥ b unsigned |
| 9 | `jbe` | C ∨ Z | a ≤ b unsigned |
| A | `ja` | !C ∧ !Z | a > b unsigned |
| B | `js` | N | negative |
| C | `jns` | !N | not negative |
| D | `jo` | V | overflow |
| E | `jno` | !V | no overflow |

Code 15 is invalid. The disassembler always prints the first name (`jz`, not `je`).

## Semantics

- **Overflow** is always modulo 2³² and never a fault.
- **Division:** division or remainder by 0 → `fault.div_zero`. Signed division rounds toward 0; the
  remainder has the sign of the dividend. INT_MIN / −1 = INT_MIN with remainder 0 and V = 1.
- **Shifts:** the width from a register is taken `& 31` (so 32 shifts by 0 and 33 by 1). A width of
  0 leaves the value unchanged and sets C = 0.
- **Precise faults:** when an instruction faults, registers, memory, flags, counters and output do
  not change, and `pc` points at the faulting instruction.
- **Control-flow check:** every target of a jump, `call` or `ret` must be a multiple of 4 and lie in
  the code region (the occupied part of the text segment, `0x0100` up to the end of the last
  instruction word including the trap word). Otherwise the jumping instruction faults with
  `fault.bad_jump_target {target}` (`jmp`, `j<cc>`, `call`, `jmp rs`, `call rs`) or
  `fault.bad_return_address {target}` (`ret`). A conditional jump that is not taken does not check
  its target. *Clarification:* `call` checks its target before the stack; `ret` checks the stack
  (alignment, underflow) before the popped target.
- **Instruction fetch:** `pc` outside the code region → `fault.exec_outside_code`; a zero word →
  `fault.zero_instruction`; an invalid word → `fault.illegal_instruction {word}`.
- **`halt`** (*clarification*): counts as one step and one cycle and sets status HALTED; `pc` stays
  on the `halt` word.
- **Pseudo-instructions** (assembler only): `li rd, imm32` always produces exactly two words,
  `mov rd, lo16` and `movhi rd, hi16` (the `mov` sign-extends, the `movhi` then replaces the upper
  half, so every value in −2³¹…2³²−1 works). `mov rd, imm` outside −32768…32767 →
  `asm.imm_range` (the UI suggests `li`).
- **Two-operand short forms** for all ALU instructions: `add r1, r2` ≡ `add r1, r1, r2`,
  `add r1, 5` ≡ `add r1, r1, 5`; `neg r1` ≡ `neg r1, r1`, `not r1` ≡ `not r1, r1`. (`cmp` and `tst`
  always have two operands.) The disassembler always prints the full form.

### Memory access checks

*Clarification* of the order in which a data access at effective address `a` is checked (the first
failing check reports):

1. `a ≥ 0x10000` → `fault.out_of_range {addr, access}`
2. word access and `a` not a multiple of 4 → `fault.misaligned {addr, size: 4}`
3. segment: null → `fault.null_access`, guard → `fault.guard_page`, write to text →
   `fault.write_to_code`
4. heap segment with `heap_check = strict` → heap checks ([Heap](#heap))

Byte accesses may use any address. There are no half-word accesses. Reading the text segment is
allowed (code is readable), executing anything outside it is not.

### Stack checks

`push`, `pop`, `call` and `ret` (including the implicit push of `call rs` and pop of `ret`) must
stay inside the stack segment 0xD000…0xEFFF with `sp` a multiple of 4. *Clarification* of the
checks, performed on the value of `sp` before the instruction:

| Instruction | Fault if |
|---|---|
| `push`, `call` | `sp % 4 ≠ 0` → `fault.stack_misaligned {sp}`; else `sp − 4 < 0xD000` → `fault.stack_overflow {sp}`; else `sp > 0xF000` → `fault.stack_underflow {sp}` |
| `pop`, `ret` | `sp % 4 ≠ 0` → `fault.stack_misaligned {sp}`; else `sp ≥ 0xF000` → `fault.stack_underflow {sp}`; else `sp < 0xD000` → `fault.stack_overflow {sp}` |

Ordinary `ld`/`st` through `fp` or `sp` are subject only to the segment rights.

## Canonical text form

The disassembler prints every valid word in exactly one way, and the assembler must turn that text
back into the same word (`asm(dis(w)) = w`). *Clarification* of the format:

- lower-case mnemonic, one space, operands separated by `, `; the full form, never a short form;
  conditional jumps use their first name (`jz`, never `je`);
- registers `r0`…`r7`, `fp`, `sp`;
- signed immediates (s16) and shift widths in decimal (`add r1, r2, -1`, `shl r1, r1, 3`);
- unsigned immediates (u16: `movhi`, `divu`, `remu`, `and`, `or`, `xor`, `tst`) in hexadecimal with
  `0x` and upper-case digits, without leading zeros (`and r1, r2, 0xFF`, `tst r1, 0x0`);
- `sys` with the system call name if the number is known (`sys WRITE_INT`), else decimal (`sys 99`);
- memory operands `[fp]`, `[fp + 8]`, `[fp - 8]` (decimal offset, `+`/`-` with spaces) and
  `[0x4000]` (absolute: `0x` and exactly four upper-case hex digits);
- jump and call targets as absolute addresses: `0x` and four upper-case hex digits when the target
  is in 0…0xFFFF (`jz 0x0104`), otherwise as a signed decimal number (`jmp -2096896`); the
  assembler accepts both because a jump operand is an address expression.

Debugger views may show a label instead of a numeric jump target; that text assembles to the same
word too.

## Memory map

64 KiB, page table with 256 pages of 256 bytes. Each page has the rights of its segment.

| Segment | From | To (excl.) | Size | Rights | Contents |
|---|---|---|---|---|---|
| null | 0x0000 | 0x0100 | 256 B | – | `fault.null_access` |
| text | 0x0100 | 0x4000 | 16128 B | R X | code: player, then library, then the test trap word |
| data | 0x4000 | 0x6000 | 8 KiB | R W | `.data`: player, then library |
| input | 0x6000 | 0x8000 | 8 KiB | R W | level input data (`set` in tests), constant `INPUT` |
| heap | 0x8000 | 0xC000 | 16 KiB | R W* | only through `ALLOC`; *heap checks ([Heap](#heap)) |
| guard | 0xC000 | 0xD000 | 4 KiB | – | `fault.guard_page` |
| stack | 0xD000 | 0xF000 | 8 KiB | R W | `sp` starts at 0xF000 and grows down |
| screen | 0xF000 | 0x10000 | 4 KiB | R W | screen |

- Writing to text → `fault.write_to_code`; executing outside text → `fault.exec_outside_code`.
  Self-modifying code is impossible (W^X).
- Word accesses only at addresses that are multiples of 4 (`fault.misaligned {addr, size}`); byte
  accesses anywhere; no half-words.
- The unused part of the text segment is 0, so running into it gives `fault.zero_instruction`.
- Memory outside the loaded program and data is 0 at the start of every run.

## Screen

64×64 pixels, one byte per pixel, row by row; pixel (x, y) is at `SCREEN + y*64 + x`; (0, 0) is the
top-left corner. The colour is `byte & 0x0F`; the upper bits are stored but ignored for display and
for test comparison. Word accesses are allowed (4 pixels, the lowest address is the leftmost pixel).
The screen is readable. A test can set an initial image.

The 16 colours have fixed **names** (assembler constants):

| Value | Name | Value | Name |
|---|---|---|---|
| 0 | `BLACK` | 8 | `DARKGRAY` |
| 1 | `BLUE` | 9 | `LIGHTBLUE` |
| 2 | `GREEN` | 10 | `LIGHTGREEN` |
| 3 | `CYAN` | 11 | `LIGHTCYAN` |
| 4 | `RED` | 12 | `LIGHTRED` |
| 5 | `MAGENTA` | 13 | `PINK` |
| 6 | `BROWN` | 14 | `YELLOW` |
| 7 | `LIGHTGRAY` | 15 | `WHITE` |

The RGB values are a design decision of the web UI (`web/js/palette.js`); the core knows only
indices.

## System calls

`sys n` changes only `r0` (and the tapes, heap and random state); flags are unchanged; each call
costs 4 cycles. The name or the number may be written (`sys WRITE_INT` = `sys 2`).

| No. | Name | Input | Effect |
|---|---|---|---|
| 1 | `READ_INT` | – | r0 = next number. Input exhausted: in test mode `fault.input_end`; interactively (sandbox, input still open) status WAITING_INPUT, `pc` stays, the call is repeated after input arrives |
| 2 | `WRITE_INT` | r0 | appends the number (signed decimal) to the output tape |
| 3 | `READ_CHAR` | – | r0 = next byte of the text input; at the end r0 = −1 (interactively with open input: WAITING_INPUT) |
| 4 | `WRITE_CHAR` | r0 & 0xFF | appends the byte to the output tape |
| 5 | `WRITE_STR` | r0 = address | writes the zero-terminated string; at most 4096 bytes; checked completely before anything is written (`fault.string_too_long`, segment and heap faults) |
| 6 | `ALLOC` | r0 = size | r0 = address (multiple of 8) or 0 (size 0 or no space) |
| 7 | `FREE` | r0 = address | frees the block; `FREE` of 0 does nothing |
| 8 | `RAND` | – | r0 = next xorshift32 value; the state starts from the run's seed (seed 0 → 0x9E3779B9) |

Any other number → `fault.unknown_syscall {n}`.

**Output tape:** entries `(kind, value)` with kind 0 = number, 1 = byte, value an i32. At most 8192
entries; a call that would exceed this faults with `fault.output_limit` and writes nothing.

**Input:** two channels, numbers (`READ_INT`) and text bytes (`READ_CHAR`). In test mode the input is
closed from the start. In interactive mode, an empty but open channel makes the machine wait
(status WAITING_INPUT, nothing changes, no step and no cycles are counted); after more input or
after closing the input the same `sys` executes again.

*Clarifications:*

- `READ_CHAR` at the end of a closed text input returns −1 (0xFFFFFFFF) and is not a fault.
- `WRITE_STR` writes one byte entry per character; the terminating 0 is not written. The string may
  have at most 4096 bytes before the 0; otherwise `fault.string_too_long {addr}` with the start
  address. Every byte read is checked like an `ldb` (segment rights, heap checks).
- `RAND` is Marsaglia's xorshift32 with shifts (13, 17, 5): `x ^= x << 13; x ^= x >> 17;
  x ^= x << 5`. The new state is stored and returned in r0. The initial state is the seed of the
  run (test `seed`, else level `seed`), or 0x9E3779B9 if that seed is 0.
- `ALLOC` treats r0 as unsigned; sizes that cannot fit return 0.

## Heap

The heap is managed in Rust, not in VM memory: first fit over 0x8000…0xBFFF, blocks aligned to 8,
followed by an 8-byte **red zone**. A shadow map holds one byte per 4-byte granule with the states
0 free, 1 allocated, 2 freed, 3 red zone. `ALLOC` fills the block with 0xCD and `FREE` with 0xDD
(visible in the hex dump). Freed space is reused.

*Clarification* of the layout: a request of n bytes occupies a span of round_up(n, 8) + 8 bytes
starting at an 8-aligned address; the first round_up(n, 4) bytes are *allocated*, the rest of the
span is *red zone*. First fit chooses the lowest 8-aligned address whose span consists only of free
or freed granules. `FREE` marks the whole span *freed*.

Level switch `heap_check`:

- `strict` (default): every data access to a free, freed or red-zone granule faults:
  `fault.heap_unallocated`, `fault.use_after_free`, `fault.heap_overflow` (all with `addr`).
- `off`: these three checks are skipped (the access just reads or writes; after `FREE` the block
  contains 0xDD…).

Always active: `fault.invalid_free` (the address is not the start of a block) and
`fault.double_free`.

For tests the heap reports the number of live blocks (`expect_heap_empty`) and a list
`{addr, size, live, alloc_pc}`.

## Calling convention

- Arguments 1–4 in `r0`–`r3`; further arguments on the stack, pushed from right to left (the 5th
  argument is at `[fp + 8]` inside the callee).
- Return value in `r0` (a second value in `r1`).
- `r0`–`r3` are saved by the caller; `r4`–`r7` and `fp` are saved by the callee; `sp` must have its
  old value after `ret`; flags are not preserved.
- Prologue: `push fp` / `mov fp, sp` / `sub sp, N`. Epilogue: `mov sp, fp` / `pop fp` / `ret`.
- Frame: `[fp + 0]` old fp, `[fp + 4]` return address, `[fp + 8 …]` stack arguments,
  `[fp - 4 …]` local variables.

**Debugger frame chain:** follow the fp chain until fp = 0, fp leaves the stack segment, fp does not
increase, or 64 frames. The function name of a frame is the nearest global label at or below the
call target of its return site (from the symbol table); for frames without a known target, the label
at or below `pc` (innermost frame) or the child's return address.

## Cycles and budget

| Instructions | Cycles |
|---|---|
| `halt`, `nop`, `mov`, `movhi`, `lea`, ALU except below, `neg`, `not`, `jmp`/`j<cc>` (taken or not), `jmp rs` | 1 |
| `ret`, `ld`, `ldb`, `ldbs`, `st`, `stb`, `push`, `pop`, `call L`, `call rs` | 2 |
| `mul` | 3 |
| `sys` | 4 |
| `div`, `divu`, `rem`, `remu` | 8 |

Every test runs with a cycle **budget** (level `budget`, default 200000, at most 10000000; a test
may override it). *Clarification:* before each instruction the machine checks `cycles ≥ budget`;
if so, the run stops with status BUDGET_EXCEEDED (`run.budget_exceeded {cycles, hottest_line}`).
A fault does not consume cycles.

## Assembly language

### Lines

- One statement per line, optionally preceded by labels: `loop: add r1, 1`. Comments start with `;`
  and run to the end of the line.
- Mnemonics, registers, directives and system call names are case-insensitive. Labels and constants
  are case-sensitive.
- Immediates have no `#`. Identifiers match `[A-Za-z_][A-Za-z0-9_]*`. Reserved names (registers,
  mnemonics including `li` and condition aliases, system call names — all case-insensitive —,
  built-in constants and colour names, and names starting with `__`) cannot be defined →
  `asm.reserved_name`.
- **Local labels** `.name:` belong to the most recent global label (internally `global.name`) and are
  referenced as `.name` inside that scope or as `global.name` anywhere. A local label before any
  global label → `asm.local_label_without_global`.
- *Clarification:* a label refers to the address of the next emitted item **after** that item's
  automatic alignment (so `values: .word 1` and a label on the line before `.word` both point at the
  aligned word). A label at the end of a section refers to the current position.

### Numbers, strings, expressions

- Numbers: `42`, `-7`, `0x2A`, `0b1010`, `1_000` (underscores between digits), character literals
  `'A'`, `'\n'`.
- Strings are UTF-8 with the escapes `\n \t \0 \\ \" \' \xHH`.
- Expressions are evaluated in i64 with overflow checks (`asm.expr_overflow`); precedence as in C:
  unary `-` `~`, then `* / %`, `+ -`, `<< >>`, `&`, `^`, `|`; parentheses. Division by zero →
  `asm.expr_div_zero`.
- `.equ NAME, expr` must be defined **before** it is used; labels may be used before their
  definition. Where a value must be known immediately (`.equ`, `.space`, `.align`) a later label or
  constant gives `asm.needs_constant`.

### Sections and data

- Sections `.text` (default) and `.data`. Data directives in `.text` → `asm.data_in_text`;
  instructions in `.data` → `asm.code_in_data`.
- `.word a, b, …` (aligns to 4; each value −2³¹…2³²−1), `.byte …` (each −128…255), `.string "x"`
  (with terminating 0), `.ascii "x"` (without), `.space n` (n zero bytes), `.align n`.
  *Clarification:* `.align n` takes the alignment in bytes, a power of two from 1 to 256; other
  values → `asm.imm_range`.
- Instructions are automatically aligned to 4.
- Size limits: text ≤ 16128 bytes (player + library + trap word), data ≤ 8192 bytes, otherwise
  `asm.section_overflow {section, size, max}`.

### Operands

- `sys` takes a name from [System calls](#system-calls) or a number 0…65535.
- Immediate ranges follow the immediate kind (s16, u16, sh); for example `and r1, -1` is out of
  range because `and` takes 0…65535. Out of range → `asm.imm_range {value, min, max}`.
- Memory offsets: `[reg ± expr]` −32768…32767, `[expr]` 0…0xFFFF.
- Jump and call operands are address expressions (usually a label); the distance must be a multiple
  of 4 and fit in 20 signed bits of words.
- *Clarification* of diagnostics for jump targets: a target outside the 20-bit range gives
  `asm.imm_range` with absolute addresses as `value`, `min` and `max`; a target that is not a
  multiple of 4 away gives `asm.operand_type {expected: "label"}`. `mov rd, imm` out of range gives
  `asm.imm_range` with `hint: "li"`.
- Built-in constants: `SCREEN` = 0xF000, `SCREEN_W` = 64, `SCREEN_H` = 64, `INPUT` = 0x6000,
  `HEAP` = 0x8000, `STACK_TOP` = 0xF000, the 16 colour names, and the system call names (only as
  the operand of `sys`).

### Player and library

- A pre-pass collects the library's `.equ` constants (they may use only built-in and earlier
  library constants) and makes them visible to the player.
- Layout: text = player code from 0x0100, then the library, then one invisible `halt` word, the test
  trap `__trap` (used by function tests); data = player `.data` from 0x4000, then library `.data`.
- *Clarification:* exactly one zero word (0x00000000) separates the player's code from the library
  (or from the trap word if there is no library). A program that forgets `halt` and runs past its
  end stops with `fault.zero_instruction` instead of falling into library code. The zero word has no
  line-map entry and does not count towards the size. The library's `.data` starts at the next
  multiple of 4 after the player's.
- Labels of both files share one namespace (duplicate → `asm.duplicate_label`). Entry is always
  0x0100.
- The level options `allow` / `forbid` filter the player's mnemonics (not the library's) →
  `asm.forbidden_instruction`. Aliases count as their canonical name (`je` = `jz`); `li` is filtered
  by its own name.

### Diagnostics

Every error is `{key, file, line, col, len, args}` with file 0 = player and 1 = library; line and
column are 1-based, and **columns count UTF-16 code units** (as the browser editor does). For an
unknown mnemonic the assembler suggests the closest known one within Levenshtein distance 2
(`args.suggestion`). After 50 errors it stops with `asm.too_many_errors`. All keys:
[ERRORS.md](ERRORS.md).

### Line map and size

- Every instruction address maps to `(file, line)`; the zero word and the trap word have no entry.
  *Clarification:* `.align n` in `.text` pads with `nop` words that map to the `.align` line but
  do not count towards the size.
- The **size** metric is the number of instruction words from file 0 (`li` counts 2).

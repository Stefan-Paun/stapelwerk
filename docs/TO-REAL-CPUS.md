# From Stapelwerk to real CPUs

Stapelwerk is an invented teaching machine, but every concept in it exists in real processors. This
page maps it to **x86-64**, **ARM64** (AArch64) and **RISC-V** (RV32/RV64 with the M extension).
It sticks to well-established facts; ABI details refer to the usual Linux/macOS conventions
(System V AMD64, AAPCS64, the RISC-V psABI) unless stated otherwise. Assembly syntax is Intel syntax
for x86, GNU syntax for ARM64 and RISC-V.

## Registers

| | Stapelwerk | x86-64 | ARM64 | RISC-V |
|---|---|---|---|---|
| General-purpose registers | 10 × 32 bit (`r0`–`r7`, `fp`, `sp`) | 16 × 64 bit (`rax` … `r15`; 32-bit halves `eax` …) | 31 × 64 bit (`x0`–`x30`; 32-bit views `w0`–`w30`) plus a separate `sp` | 32 registers (`x0`–`x31`), `x0` is always 0 |
| Stack pointer | `sp` | `rsp` | `sp` | `sp` = `x2` |
| Frame pointer | `fp` | `rbp` (by convention) | `x29` | `s0`/`fp` = `x8` |
| Return address | on the stack | on the stack | link register `x30` (`lr`) | `ra` = `x1` |
| Program counter | not addressable | `rip` (only for RIP-relative addressing) | not a general register | not a general register |
| Flags | Z, N, C, V | ZF, SF, CF, OF (plus others) in `RFLAGS` | Z, N, C, V in `NZCV` | none |

## Calling conventions

| | Stapelwerk | x86-64 System V | x86-64 Windows | ARM64 (AAPCS64) | RISC-V |
|---|---|---|---|---|---|
| Integer arguments | `r0`–`r3`, then stack | `rdi rsi rdx rcx r8 r9`, then stack | `rcx rdx r8 r9`, then stack (plus 32 bytes of shadow space) | `x0`–`x7`, then stack | `a0`–`a7`, then stack |
| Return value | `r0` (`r1`) | `rax` (`rdx`) | `rax` | `x0` (`x1`) | `a0` (`a1`) |
| Callee-saved | `r4`–`r7`, `fp` | `rbx rbp r12–r15` | `rbx rbp rdi rsi r12–r15` (and `xmm6`–`xmm15`) | `x19`–`x28`, `x29` | `s0`–`s11` |
| Stack alignment | 4 bytes | 16 bytes at each `call` | 16 bytes | 16 bytes | 16 bytes |

The closest real relative of Stapelwerk's frame layout is 32-bit x86 with `cdecl` (arguments on the
stack pushed from right to left, first argument at `[ebp+8]`, old `ebp` at `[ebp]`, return address at
`[ebp+4]`, result in `eax`). In x86-64 the same layout uses 8-byte slots: old `rbp` at `[rbp]`,
return address at `[rbp+8]`, stack arguments from `[rbp+16]`.

**Prologue and epilogue:**

| | Prologue | Epilogue |
|---|---|---|
| Stapelwerk | `push fp` · `mov fp, sp` · `sub sp, N` | `mov sp, fp` · `pop fp` · `ret` |
| x86-64 | `push rbp` · `mov rbp, rsp` · `sub rsp, N` | `mov rsp, rbp` · `pop rbp` · `ret` (or `leave` · `ret`) |
| ARM64 | `stp x29, x30, [sp, #-16]!` · `mov x29, sp` | `ldp x29, x30, [sp], #16` · `ret` |
| RISC-V (RV32) | `addi sp, sp, -16` · `sw ra, 12(sp)` · `sw s0, 8(sp)` · `addi s0, sp, 16` | `lw ra, 12(sp)` · `lw s0, 8(sp)` · `addi sp, sp, 16` · `ret` |

On ARM64 and RISC-V the return address arrives in a register, so a leaf function that calls nothing
does not need to touch the stack at all.

## Flags and conditions

Stapelwerk's flags follow x86, including the meaning of C after a subtraction: **C = borrow**
(C = 1 when a < b unsigned). **ARM uses the opposite convention:** after `subs`/`cmp`, C = 1 means
*no* borrow. That is why ARM's "unsigned lower" condition is `lo` (= `cc`, carry clear) while x86's
is `jb` (= `jc`, carry set). RISC-V has no flags; its branches compare two registers directly.

| Stapelwerk | Meaning after `cmp a, b` | x86-64 | ARM64 | RISC-V |
|---|---|---|---|---|
| `jz` (`je`) | a = b | `je` / `jz` | `b.eq` | `beq a, b` |
| `jnz` (`jne`) | a ≠ b | `jne` / `jnz` | `b.ne` | `bne a, b` |
| `jlt` | a < b signed | `jl` | `b.lt` | `blt a, b` |
| `jge` | a ≥ b signed | `jge` | `b.ge` | `bge a, b` |
| `jle` | a ≤ b signed | `jle` | `b.le` | `bge b, a` (pseudo `ble`) |
| `jgt` | a > b signed | `jg` | `b.gt` | `blt b, a` (pseudo `bgt`) |
| `jb` | a < b unsigned | `jb` / `jc` | `b.lo` / `b.cc` | `bltu a, b` |
| `jae` | a ≥ b unsigned | `jae` / `jnc` | `b.hs` / `b.cs` | `bgeu a, b` |
| `jbe` | a ≤ b unsigned | `jbe` | `b.ls` | `bgeu b, a` (pseudo `bleu`) |
| `ja` | a > b unsigned | `ja` | `b.hi` | `bltu b, a` (pseudo `bgtu`) |
| `js` | negative | `js` | `b.mi` | `bltz a` (pseudo) |
| `jns` | not negative | `jns` | `b.pl` | `bgez a` (pseudo) |
| `jo` | signed overflow | `jo` | `b.vs` | – (no overflow flag) |
| `jno` | no signed overflow | `jno` | `b.vc` | – |

On ARM64 only instructions with an `s` suffix (`adds`, `subs`, `ands`, …) and `cmp`/`cmn`/`tst` set
the flags; on x86 most arithmetic and logic instructions do.

## Typical instructions

| Stapelwerk | x86-64 | ARM64 | RISC-V |
|---|---|---|---|
| `mov r0, r1` | `mov eax, ecx` | `mov w0, w1` | `mv a0, a1` (= `addi a0, a1, 0`) |
| `li r0, 0x12345678` (`mov` + `movhi`) | `mov eax, 0x12345678` (one instruction) | `movz w0, #0x5678` · `movk w0, #0x1234, lsl #16` | `lui a0, 0x12345` · `addi a0, a0, 0x678` (pseudo `li`) |
| `add r0, r1, r2` | `lea eax, [rcx+rdx]` or `mov` + `add` (two-operand form) | `add w0, w1, w2` | `add a0, a1, a2` |
| `cmp r0, 5` | `cmp eax, 5` | `cmp w0, #5` | – (compare inside the branch) |
| `lea r0, [fp - 8]` | `lea rax, [rbp-8]` | `sub x0, x29, #8` | `addi a0, s0, -8` |
| `ld r0, [r1 + 4]` | `mov eax, [rcx+4]` | `ldr w0, [x1, #4]` | `lw a0, 4(a1)` |
| `ldb` / `ldbs` | `movzx` / `movsx` | `ldrb` / `ldrsb` | `lbu` / `lb` |
| `st r0, [r1]` / `stb` | `mov [rcx], eax` / `mov [rcx], al` | `str w0, [x1]` / `strb` | `sw a0, 0(a1)` / `sb` |
| `push r4` / `pop r4` | `push rbx` / `pop rbx` | no push/pop: `str`/`ldr` with pre/post-index, usually in pairs (`stp`/`ldp`) | `addi sp, sp, -16` + `sw`/`sd` |
| `call f` / `ret` | `call f` / `ret` | `bl f` / `ret` | `call f` (`jal ra, f` or `auipc` + `jalr`) / `ret` (= `jalr x0, 0(ra)`) |
| `jmp r0` / `call r0` | `jmp rax` / `call rax` | `br x0` / `blr x0` | `jr a0` / `jalr a0` |
| `sys WRITE_INT` | `syscall` (number in `rax`) | `svc #0` (number in `x8`) | `ecall` (number in `a7`) |

The system call numbers above refer to Linux. Real system calls receive their number in a register
and return their result in `rax` / `x0` / `a0`; Stapelwerk encodes the number in the instruction and
returns the result in `r0`.

## Arithmetic corner cases

| Case | Stapelwerk | x86-64 | ARM64 | RISC-V |
|---|---|---|---|---|
| Integer overflow in `add`/`sub`/`mul` | wraps, sets C/V | wraps, sets CF/OF | wraps (flags only with `adds`/`subs`) | wraps silently |
| Division by zero | `fault.div_zero` | `div`/`idiv` raise a divide error (#DE; SIGFPE on Linux) | `sdiv`/`udiv` return 0, no exception | quotient = all ones (−1 signed), remainder = dividend, no exception |
| INT_MIN / −1 | INT_MIN, remainder 0, V = 1 | `idiv` raises a divide error | returns INT_MIN | quotient INT_MIN, remainder 0 |
| Remainder instruction | `rem`, `remu` | part of `div`/`idiv` (in `edx`) | none; computed with `msub` | `rem`, `remu` |
| Shift width from a register | `& 31` | masked to 5 bits (32-bit operands) or 6 bits (64-bit) | modulo the register width | low 5 bits (RV32) or 6 bits (RV64) |
| Carry from a shift | last bit shifted out | CF = last bit shifted out (flags unchanged for width 0) | shifts do not set flags | no flags |
| Overflow flags of `mul` | C: unsigned product too big, V: signed product too big | `mul`: CF = OF = upper half ≠ 0; `imul`: CF = OF = signed result truncated | none (use `umulh`/`smulh` for the upper half) | none (`mulh`/`mulhu` for the upper half) |

## Encoding

| | Stapelwerk | x86-64 | ARM64 | RISC-V |
|---|---|---|---|---|
| Instruction length | always 4 bytes | 1–15 bytes | always 4 bytes | 4 bytes; 2 bytes with the C extension |
| Byte order | little-endian | little-endian | little-endian in practice (big-endian possible) | little-endian (standard ABIs) |
| Relative branch range | 20 bits of words (±2 MiB) | 8- or 32-bit byte offset | `b`: 26 bits of words (±128 MiB); `b.cond`: 19 bits (±1 MiB) | `jal`: ±1 MiB; conditional branches: ±4 KiB |

## Memory

- **Alignment.** x86 allows misaligned accesses (with few exceptions). ARM64 allows them for ordinary
  loads and stores to normal memory. On RISC-V, support for misaligned accesses depends on the
  implementation (they may be slow or trap). Stapelwerk forbids misaligned word accesses so that
  alignment becomes visible.
- **Null page.** Operating systems usually leave the lowest pages of an address space unmapped so
  that a null-pointer access traps; Stapelwerk's null segment does the same.
- **W^X.** Modern systems map code pages read-only and data pages non-executable (the NX bit on
  x86-64, XN bits on ARM). Stapelwerk enforces this for every access.
- **Guard pages.** Thread stacks usually have an unmapped guard page next to them, so that a stack
  overflow traps instead of overwriting other data; Stapelwerk's guard segment between heap and
  stack plays this role.
- **Memory-mapped I/O.** Devices are controlled by reading and writing addresses. A classic example
  of a byte-per-pixel framebuffer is the PC's VGA mode 13h (320×200, 256 colours, at physical
  address 0xA0000). Stapelwerk's 16 colour names follow the order of the classic 16-colour CGA/EGA
  palette.

## Heap debugging

- The fill bytes come from Microsoft's debug C runtime: 0xCD marks freshly allocated memory and
  0xDD freed memory (it also uses 0xFD for guard bytes around blocks).
- Red zones and a shadow map are how **AddressSanitizer** (Clang, GCC) detects heap overflows and
  use-after-free. ASan uses one shadow byte per 8 bytes of memory; Stapelwerk uses one per 4 bytes.

## Random numbers

`RAND` uses xorshift32 (George Marsaglia, 2003): three shifts and XORs, fast and small, but not
suitable for cryptography.

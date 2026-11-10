# Curriculum v1 (30 levels, 8 chapters)

Binding for everyone who writes levels. The machine is defined in [ISA.md](ISA.md); instruction and
system call names there take precedence. The level file format is in [LEVELS.md](LEVELS.md).

Markers after the ID: [T] tutorial, [D] debugging level (start code contains a bug), [F] chapter
final, [B] bonus (not required to progress). These four are the level's `kind` key in its `.lvl`
file; a level without one of them is `kind = normal`. [S] marks a screen level (at least one test
with `expect_screen`) and comes first when combined with a kind ([S][T], [S][B], [S][F]). "Diff"
in the tables is the difficulty (1–5). "Ref" is the estimated length of the reference solution in
instructions (a guideline).

## Positioning (for the README and the start page)

Stapelwerk closes the gap between the model computers used in schools (Johnny, Bonsai, Little Man
Computer) and university tools (MARS, RARS, Ripes). The machine is small enough to understand in an
afternoon, yet it has everything that a first semester examines: two's complement, flags, stack
frames, calling conventions, the heap and memory-mapped I/O. Unlike commercial programming puzzle
games, every level teaches what a real x86, ARM or RISC-V processor and a C compiler do, and
debugging is a game mechanic of its own.

For upper secondary students, first-year students and teachers: no account, no data collection
(GDPR), works offline, openly licensed.

## Chapter 0 – Power On (Einschalten)

| ID | Title DE / EN | Learning goal | Task | Test | New | Diff | Ref |
|---|---|---|---|---|---|---|---|
| 0.1 [S][T] | Erster Schritt / First Step | Operate the debugger; read registers and the PC. | A finished program draws a face. Step through it, set a breakpoint on `auge:`, take one step back. (Checklist in the UI: step, breakpoint, run, step back.) | screen | registers, PC, step, step back, breakpoint, `halt` | 1 | given, ~12 |
| 0.2 | Ein Register ändern / Change a Register | Registers hold numbers; `mov` sets them, `sys WRITE_INT` prints r0. | The program prints 7. Stop before the output, set r0 to 42 in the debugger, then make the change permanent in the code (the test expects 42). | I/O | `mov`, `sys WRITE_INT`, editing registers in the debugger | 1 | 3 |

## Chapter 1 – Registers & Arithmetic (Register und Rechnen)

| ID | Title DE / EN | Learning goal | Task | Test | New | Diff | Ref |
|---|---|---|---|---|---|---|---|
| 1.1 | Plus und Minus / Plus and Minus | Read values, keep them, compute with two operands. | Read a and b; print a+b, then a−b. | I/O | `sys READ_INT`, `add`, `sub`, short form | 1 | 10 |
| 1.2 [S] | Erster Pixel / First Pixel | The screen is memory. | Read x, y, colour; set exactly that pixel (SCREEN + y·64 + x). | screen | `stb`, `mul`, `SCREEN`, address, byte, MMIO | 2 | 9 |
| 1.3 [D] | Überschrieben / Overwritten | System calls write to r0. | Should print width·height, prints height². Find where the width gets lost and repair it. | I/O | "last changed" marker, r0 convention | 1 | 8 |
| 1.4 [F] | Sekundenuhr / Stopwatch | Integer division and remainder. | Read seconds (0–359999), print hours, minutes, seconds. | I/O | `div`, `rem` | 2 | 13 |

## Chapter 2 – Jumps & Loops (Sprünge und Schleifen)

| ID | Title DE / EN | Learning goal | Task | Test | New | Diff | Ref |
|---|---|---|---|---|---|---|---|
| 2.1 | Die größere Zahl / The Larger One | `cmp` sets flags, conditional jumps read them. | Read a and b; print the larger one. | I/O | `cmp`, Z/N/V, `jz jnz jlt jle jgt jge jmp`, labels | 2 | 8 |
| 2.2 | Bis zur Null / Until Zero | A loop with a sentinel. | Read numbers until 0, print the sum. **Sample level, see [LEVELS.md](LEVELS.md#sample-level).** | I/O | loop, sentinel | 2 | 9 |
| 2.3 [S] | Rechteck / Rectangle | Nested loops; next row = +64. | Read x, y, width, height, colour; fill the rectangle. | screen | nested loop | 3 | 18 |
| 2.4 [D] | Endlosschleife / Never Ending | "not zero" ≠ "greater than zero"; read the budget message. | Should print n, n−2, … while > 0; loops forever for odd n (`jnz` instead of `jgt`). | I/O | cycle budget, line counter | 2 | 8 |
| 2.5 [F] | Collatz | Loop plus branch. | Read n > 0, print the Collatz sequence down to 1, then the number of steps. | I/O | combination | 3 | 16 |

## Chapter 3 – Bits & Numbers (Bits und Zahlen)

| ID | Title DE / EN | Learning goal | Task | Test | New | Diff | Ref |
|---|---|---|---|---|---|---|---|
| 3.1 | Einsen zählen / Count the Ones | Masks, logical shifts; negative numbers are bits too. | Print the number of 1 bits (−1 → 32). | I/O | `and or xor not shl shr`, `0b`, `0x` | 3 | 8 |
| 3.2 [S][B] | 8×8-Zeichen / 8×8 Glyph | Test bits and draw them. | Read x, y, colour and 8 row bytes (bit 7 on the left); draw the glyph. | screen | bit test with a mask, `tst` | 3 | 22 |
| 3.3 [D] | Halbe Wahrheit / Half Truth | Two's complement, logical vs. arithmetic shift. | Halves measurements with `shr`; negative values turn into billions. Repair it (round down). | I/O | `sar`, sign bit, N | 3 | 9 |
| 3.4 [F] | Hex-Ausgabe / Hex Dump | Nibbles → ASCII. | Print a number as exactly 8 hex digits (−1 → `FFFFFFFF`). | text | `sys WRITE_CHAR`, ASCII, `'A'` | 3 | 15 |

## Chapter 4 – Memory (Speicher)

| ID | Title DE / EN | Learning goal | Task | Test | New | Diff | Ref |
|---|---|---|---|---|---|---|---|
| 4.1 | Größter Wert / Largest Value | Arrays, base + index·4, alignment. | `anzahl` and `werte` lie at `INPUT` (the library defines `ANZAHL` and `WERTE` as constants). Write the maximum to `MAX` (library: an address in the data segment). | memory | `.data .word .space`, `ld st`, `[reg+imm]`, little-endian | 3 | 13 |
| 4.2 [S][B] | Spiegelbild / Mirror | Two pointers, swapping. | Mirror the given picture horizontally. | screen (with initial image) | `ldb`, swapping | 4 | 20 |
| 4.3 [D] | Einer zu viel / One Too Many | Arrays have no bounds. | The sum reads one word too far (`jle` instead of `jlt`) and adds the word behind the array → wrong sum. | memory | memory view, off-by-one | 3 | 11 |
| 4.4 [F] | Sortieren / Sorting | Nested loops over memory. | Insertion sort in place (C code in the task text). | memory | combination | 4 | 22 |

## Chapter 5 – Stack & Subroutines (Stack und Unterprogramme)

| ID | Title DE / EN | Learning goal | Task | Test | New | Diff | Ref |
|---|---|---|---|---|---|---|---|
| 5.1 | Rückwärts / Backwards | The stack is LIFO; sp grows downwards. | Read numbers until 0, print them in reverse. | I/O | `push pop`, sp, stack view | 3 | 12 |
| 5.2 [S] | Pixel als Funktion / Pixel Function | A subroutine following the calling convention. | Write `pixel` (r0 = x, r1 = y, r2 = colour); ignore pixels outside the screen. The checker calls `pixel` directly. | function + screen | `call ret`, return address, caller/callee-saved, `jae` (unsigned!) | 3 | 11 |
| 5.3 [D] | Der vergessene Pop / The Forgotten Pop | Every path must balance the stack. | `max3` does `push r4`; one branch forgets the `pop` → `ret` jumps into nowhere (`fault.bad_return_address`). | I/O | reading the fault message, stack view | 3 | 16 |
| 5.4 [S][F] | Linie / Line | Translate an algorithm from C; register pressure. | Read x0, y0, x1, y1, colour; draw the line with Bresenham's algorithm using the given `pixel` from the library. | screen | library function, spilling values to the stack | 5 | 35 |

## Chapter 6 – Recursion (Rekursion)

| ID | Title DE / EN | Learning goal | Task | Test | New | Diff | Ref |
|---|---|---|---|---|---|---|---|
| 6.1 | Fakultät / Factorial | Stack frames with fp, prologue/epilogue. | `fak(n)` recursively (n ≤ 12); the checker calls `fak`. | function | fp, frame, local variable | 3 | 14 |
| 6.2 [D] | Falscher Fibonacci / Fibonacci Gone Wrong | Caller- vs. callee-saved. | `fib(n)` keeps an intermediate value in r1; the recursive call destroys it. | function | frame chain, "back to the last change" | 4 | 20 |
| 6.3 [S][B] | Flutfüllung / Flood Fill | Recursion made visible; depth costs stack. | Fill the connected area of the same colour starting at (x, y) with a new colour (test images: areas ≤ 150 pixels). | screen (initial image) | recursion depth, stack overflow | 4 | 28 |
| 6.4 [F] | Türme von Hanoi / Towers of Hanoi | Recursion with several arguments. | Read n (1–10), print the moves as pairs (from, to). | I/O | combination | 4 | 24 |

## Chapter 7 – Heap & Lists (Heap und Listen)

| ID | Title DE / EN | Learning goal | Task | Test | New | Diff | Ref |
|---|---|---|---|---|---|---|---|
| 7.1 | Kettenglieder / Chain Links | Requesting and freeing memory, pointers, struct layout. | Read numbers until 0, prepend each to a list (node: `next` at 0, `wert` at 4), print the list, free everything. | I/O + heap empty | `sys ALLOC`, `sys FREE`, null pointer | 4 | 24 |
| 7.2 [D] | Nach dem Freigeben / After Free | Access after free silently returns garbage. | Removes negative nodes but reads `next` only after `FREE` (which now holds 0xDDDDDDDD) → later crash with `fault.out_of_range`. `heap_check = off`. | I/O + heap empty | fill patterns, heap map, back to the last change | 4 | 26 |
| 7.3 [F] | Sortiert einfügen / Sorted Insert | Pointer work including the head special case. | Read numbers until 0, insert them sorted, print, free. | I/O + heap empty | combination | 5 | 32 |

## Didactic curve (manual terms introduced before each chapter)

| Ch. | Terms | Steep point | Countermeasure |
|---|---|---|---|
| 0 | register, PC, instruction, cycle, breakpoint | – | guided checklist |
| 1 | two-operand form, system call, input stream, r0 convention, byte, address, MMIO, integer division | 1.2: addresses early | formula and picture in the text |
| 2 | flags Z/N/C/V, `cmp` = subtraction without result, jump table, loop patterns, sentinel, budget | 2.3: first nesting | hint 2 with a scaffold |
| 3 | binary/hex, bit numbers, masks, two's complement, overflow (C vs. V), logical/arithmetic shift, ASCII | 3.1: negative input | visible test with −1 |
| 4 | segments/rights, word/byte, little-endian, alignment, base + index·4, data directives | 4.1: biggest jump | memory view with labels, alignment message "·4?" |
| 5 | stack, push/pop, call/ret, return address, calling convention, register pressure | 5.4: Bresenham | C code in the text, `pixel` given |
| 6 | frame, fp, prologue/epilogue, local variables, recursion tree, stack overflow | 6.1: frames | stack view labels frames |
| 7 | heap vs. stack, lifetime, pointers, NULL, memory leak, use after free, double free | 7.3: head | C template; "pointer to pointer" only in hint 3 |

Principle for chapter finals: what is tested is the **translation** into assembly, not inventing the
algorithm, so the C code is part of the task text (4.4, 5.4, 6.4, 7.3).

Levels that play well on a phone (`mobile = yes`): 0.1, 0.2, 1.1, 1.3, 2.1, 2.4, 3.3, 4.3, 5.3,
6.2, 7.2.

## Content guidelines

**Tone.** Address the player informally (German "du", English "you"), matter-of-fact and warm. No
chains of exclamation marks, no diminutives, no "super!". Technical terms are **bold** on first use
and link to the manual (`[Stack](handbuch:stack)`). Use the English terms that are common at
universities (stack, heap, register, flag) and give a German gloss on first use in German texts:
"Stack (Stapel)". Debugger terms in German texts are German: Haltepunkt (breakpoint), Einzelschritt
(single step). English texts carry the same content in natural English.

**Lengths.** Card: 1 sentence. Task: at most 80 words plus an example. Hint: at most 40 words
(level 3 plus a scaffold). "What you just learned": at most 150 words plus code.

**Code style.** Lower-case instructions; labels in German start code are German without umlauts
(`schleife:`, `fertig:`); constants in upper case; comments with `;` (start code and reference in
German, which is fine because the code is monolingual); aligned columns.

**Hints.** Level 1 gives the direction (the idea, no instruction names). Level 2 names the tools
(instructions, debugger features). Level 3 is almost the solution (a scaffold with `; ???` gaps).

**"What you just learned".** One summary sentence · how it looks in C · how x86, ARM and RISC-V do
it · optionally a "pro view".

**Markdown subset** in text sections: `#`/`##` headings, paragraphs, `**bold**`, `*italic*`,
`` `code` ``, fenced code blocks, lists with `- `; no tables; links only to `handbuch:…` (internal).

**Honesty.** Statements about real processors must be correct; when in doubt, leave them out. See
[TO-REAL-CPUS.md](TO-REAL-CPUS.md) for checked facts.

## Sample level

The complete level 2.2 is in [LEVELS.md](LEVELS.md#sample-level). It is the template for all other
levels; real levels add 8–20 hidden tests whose expectations are worked out by hand or checked
against the reference solution by `check-levels`.

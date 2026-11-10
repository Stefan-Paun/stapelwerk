# Writing levels

A level is one plain-text `.lvl` file in `levels/`, listed in `levels/manifest.txt`. This page
describes the file format, how tests run, how levels are checked and packed, and the command-line
tool. Content rules (tone, lengths, hint levels) are in [CURRICULUM.md](CURRICULUM.md); the machine
is described in [ISA.md](ISA.md); all error keys are in [ERRORS.md](ERRORS.md).

## Checklist for a new level

1. Copy the [sample level](#sample-level) to `levels/<id>.lvl` and add it to `levels/manifest.txt`.
2. Write both languages completely: title, card, task, three hints, "what you just learned".
3. Write the reference solution and at least five visible tests plus 8–20 hidden tests.
4. Run `stapelwerk check-levels --strict --mutate levels/` (or `npm run check`) until it passes.
5. No third-party crates or npm packages.

## File format

UTF-8, LF line endings. A **section header** is a known section name in square brackets alone on
a line; any other line, including one that merely looks like a header, is content of the current
section.

| Section | Count | Contents |
|---|---|---|
| `[level]` | 1 | metadata (key = value) |
| `[test]` | 1 or more | one test (key = value) |
| `[start]` | 0–1 | start code shown in the editor |
| `[library]` | 0–1 | library code assembled after the player's code (file 1) |
| `[reference]` | 1 in source files | reference solution (removed when packing) |
| `[text.de]`, `[text.en]` | 1 each | the task (Markdown subset) |
| `[hint.de]`, `[hint.en]` | 1 each | exactly 3 hints separated by a line `---` |
| `[learned.de]`, `[learned.en]` | 1 each | "what you just learned" |
| `[image.NAME]` | any | a 64×64 picture (see below) |

In `[level]` and `[test]` every non-empty line has the form `key [argument] = value`; `;` starts a
comment. Unknown sections → `lvl.unknown_section`, unknown keys → `lvl.unknown_key`, a section that
may appear only once appearing twice → `lvl.duplicate_section`.

### `[level]` keys

| Key | Value |
|---|---|
| `id` | unique id, e.g. `2-2-bis-zur-null` (required) |
| `version` | format version, `1` |
| `chapter` | 0–7 |
| `order` | position in the chapter, 1… |
| `title.de`, `title.en` | title |
| `card.de`, `card.en` | one sentence for the level map |
| `kind` | `normal`, `tutorial`, `debug`, `final` or `bonus` |
| `budget` | cycles per test, default 200000, maximum 10000000 |
| `seed` | u32 seed for `RAND` and function-test registers |
| `allow` **or** `forbid` | mnemonics separated by spaces (both → `lvl.allow_and_forbid`) |
| `heap_check` | `strict` (default) or `off` |
| `mobile` | `yes` or `no`: playable on a phone |
| `ref_cycles`, `ref_size` | only in packed files (written by `pack-levels`) |

### `[test]` keys

| Key | Meaning |
|---|---|
| `name` | unique name within the level |
| `hidden = yes` | hidden test (input and expectation shown only when it fails) |
| `in = 3 4 5 0` | numbers for `READ_INT` |
| `in_text = "…"` | bytes for `READ_CHAR` |
| `reg rX = v` | initial register value (function tests: `r0`–`r3`) |
| `set ADR = w1 w2 …` | words written to memory before the run |
| `setb ADR = b1 b2 …` or `setb ADR = "…"` | bytes written to memory before the run |
| `screen_init = NAME` | initial screen from `[image.NAME]` |
| `call = LABEL` | function test (see below) |
| `out = …` | expected numbers on the output tape |
| `out_text = "…"` | expected bytes on the output tape |
| `expect ADR = …` | expected words in memory after the run |
| `expectb ADR = …` | expected bytes in memory after the run |
| `expect_reg rX = v` | expected register value |
| `expect_screen = reference` or `= NAME` | expected screen (low 4 bits per pixel) |
| `expect_heap_empty = yes` | no live heap blocks at the end |
| `seed` | overrides the level seed |
| `budget` | overrides the level budget |

`ADR` is a number or a symbol of the assembled program (e.g. a label from the library).
`expect_screen = reference` means the screen after the run must equal the screen of the reference
solution after the same test. A test without `out` does not check the output; `out =` with an empty
value requires the output to be empty. A test passes when the program halts (`HALTED`) and every
expectation holds; the first failing expectation is reported as a `check.*` mismatch.

**Function tests** (`call = LABEL`): the machine starts fresh; `r0`–`r3` come from the `reg` lines
(unset ones are 0); `r4`–`r7` and `fp` get random values from the test seed (`fp ≠ 0`); the address
of `__trap` is pushed as the return address; `pc = LABEL`; the run ends when `pc` reaches `__trap`
(status HALTED). Then `r4`–`r7`, `fp` and `sp` must equal their values before the call, otherwise
`check.abi_clobbered`. Then the other expectations (`expect_reg`, …) are checked.

### Images

`[image.NAME]` contains up to 64 rows of up to 64 characters. `0`–`9` and `A`–`F` are colour
indices, `.` is 0; missing pixels are 0. Anything else, or more than 64 rows or columns →
`lvl.bad_image {name, line}`.

### Kinds

- `tutorial`: the start code may pass all tests (the UI additionally requires a checklist: a step, a
  breakpoint, a run to the breakpoint, a step back).
- All other kinds: the start code and the empty program must **not** pass.
- `debug` levels start with buggy code to repair; `final` levels close a chapter; `bonus` levels are
  optional.

### Text sections

Markdown subset: `#`/`##` headings, paragraphs, `**bold**`, `*italic*`, `` `code` ``, fenced code
blocks with ```` ``` ````, lists with `- `; no tables; links only to the manual
(`[Stack](handbuch:stack)`). The UI renders this into DOM nodes, never into HTML strings.

### `levels/manifest.txt`

One file name per line in playing order. Chapter lines look like
`# chapter N: German title | English title`.

## Test runs and metrics

`run_tests` runs every test on a fresh machine with its budget. Each result has `passed`,
`status`, `cycles`, `steps`, `fault` (or null) and `mismatch` (or null).

Metrics: `cycles_avg` (average over all tests, rounded up), `cycles_max`, `steps_avg`, and `size`
(instruction words in the player's file; `li` counts 2). Reference values `ref_cycles` and
`ref_size` come from `[reference]` (or from the packed file). Badges in the UI: **Compact**
(size ≤ ref_size) and **Swift** (cycles_avg ≤ ref_cycles). Cycle costs: [ISA.md](ISA.md#cycles-and-budget).

## Checking and packing

`stapelwerk check-levels --strict <dir|file>` verifies:

- the file parses;
- both languages are complete (`text`, `hint` with exactly 3 parts, `learned`, `title`, `card`);
- the reference solution passes every test using at most 50 % of the budget;
- the empty program and the start code do **not** pass (except `kind = tutorial`);
- test names are unique;
- `set` ranges are allowed (inside writable segments).

`--mutate` additionally deletes each instruction of the reference solution in turn; every such
mutant must fail at least one test, otherwise the level fails with "tests too weak". Exceptions:
`nop`, and instructions whose line carries the comment `; mutate:skip` (at most 2 per level, e.g. pure
optimisations).

`stapelwerk pack-levels <dir> <target>` writes the web version of every level: `[reference]` is
removed, `ref_cycles`/`ref_size` are written into `[level]`, reference screens become
`[image.__ref_TESTNAME]` sections (and the test is switched to that image), and `index.json` lists
`id`, `chapter`, `order`, `kind`, titles, cards and `mobile` for every level.

## Command-line tool

`cargo run -q -p stapelwerk-cli -- <command>` (binary name `stapelwerk`). Exit codes: 0 success,
1 failure (diagnostics, failed checks), 2 usage error or not yet implemented.

| Command | Purpose |
|---|---|
| `asm <file.s> [-o out.img]` | assemble a source file; print diagnostics or write the memory image |
| `dis <file.img\|file.s>` | disassemble an image or an assembled source file |
| `run <file.s> [--in "3 4"] [--in-text "…"] [--trace]` | assemble and run; print output, final status and optionally a trace |
| `check-levels [--strict] [--mutate] <dir\|file>` | check level files (see above) |
| `pack-levels <dir> <target-dir>` | pack levels for the web build |
| `isa-table` | print the generated instruction table; CI compares it with [isa-table.md](isa-table.md) |
| `fuzz-gen --seed S --count N --out <dir>` | generate random programs plus `digests.txt` for the native-vs-WebAssembly differential test |

## Sample level

Level 2.2 is the template for all others. (The hidden tests listed here are only a start; real
levels have 8–20 hidden tests whose expectations are computed by hand or checked against the
reference by `check-levels`.)

````
[level]
id = 2-2-bis-zur-null
version = 1
chapter = 2
order = 2
kind = normal
title.de = Bis zur Null
title.en = Until Zero
card.de = Eine Schleife, die so lange läuft, wie Daten kommen.
card.en = A loop that runs for as long as data keeps coming.
budget = 100000
seed = 2202
mobile = no

[text.de]
Die Eingabe ist eine Folge ganzer Zahlen. Wie viele es sind, weißt du vorher nicht. Das Ende erkennst du an einer **0**. Sie zählt nicht mit, sie ist nur die **Endmarke**.

Gib die Summe aller Zahlen vor der 0 aus.

Beispiel: Eingabe `3 4 5 0` → Ausgabe `12`.

[text.en]
The input is a sequence of integers. You don't know in advance how many there are. You recognise the end by a **0**. It doesn't count, it is just the **sentinel**.

Output the sum of all numbers before the 0.

Example: input `3 4 5 0` → output `12`.

[hint.de]
Du brauchst eine Schleife: lesen, prüfen, ob es die 0 ist, sonst dazuzählen, wieder von vorn. Überleg zuerst, in welchem Register die Summe die ganze Zeit liegen bleibt.
---
`cmp r0, 0` rechnet r0 − 0, verwirft das Ergebnis und setzt nur die Flags. Ist r0 null, ist Z gesetzt und `jz fertig` springt. Vorsicht: `sys READ_INT` schreibt jedes Mal nach r0. Setz einen Haltepunkt hinter das Lesen und beobachte die Flags.
---
```
        mov r1, 0          ; Summe
schleife:
        sys READ_INT
        cmp r0, 0
        ; ??? springe zu fertig, wenn Z gesetzt ist
        ; ??? zähle r0 zur Summe dazu
        jmp schleife
fertig:
        ; ??? Summe nach r0, ausgeben
        halt
```

[hint.en]
You need a loop: read, check whether it is the 0, otherwise add it, start over. First think about which register keeps the sum the whole time.
---
`cmp r0, 0` computes r0 − 0, throws the result away and only sets the flags. If r0 is zero, Z is set and `jz fertig` jumps. Careful: `sys READ_INT` writes to r0 every time. Set a breakpoint after the read and watch the flags.
---
```
        mov r1, 0          ; Summe (sum)
schleife:
        sys READ_INT
        cmp r0, 0
        ; ??? jump to fertig if Z is set
        ; ??? add r0 to the sum
        jmp schleife
fertig:
        ; ??? move the sum to r0, output it
        halt
```

[learned.de]
Du hast eine Schleife gebaut, deren Länge erst die Daten bestimmen. In C:

```c
int summe = 0, x;
while ((x = read_int()) != 0) summe += x;
```

Die 0 am Ende ist eine **Endmarke**. Zeichenketten in C enden genauso, mit dem Byte 0.

Echte Prozessoren: x86 schreibt `cmp eax, 0` und `je fertig`, ARM `cmp r0, #0` und `beq fertig`. **RISC-V hat gar keine Flags:** `beqz a0, fertig` vergleicht und springt in einem Befehl.

**Profi-Blick:** Liest du die erste Zahl vor der Schleife und prüfst am Ende mit `jnz`, fällt das `jmp` weg: mehr Befehle, aber weniger Zyklen pro Zahl. Compiler machen das automatisch („Schleifenumkehr“). Kurz und schnell sind nicht dasselbe.

[learned.en]
You built a loop whose length is decided by the data. In C:

```c
int sum = 0, x;
while ((x = read_int()) != 0) sum += x;
```

The 0 at the end is a **sentinel**. Strings in C end the same way, with the byte 0.

Real processors: x86 writes `cmp eax, 0` and `je fertig`, ARM `cmp r0, #0` and `beq fertig`. **RISC-V has no flags at all:** `beqz a0, fertig` compares and branches in a single instruction.

**Pro view:** If you read the first number before the loop and test with `jnz` at the end, the `jmp` disappears: more instructions, but fewer cycles per number. Compilers do this automatically ("loop inversion"). Short and fast are not the same thing.

[start]
; Bis zur Null
; Lies Zahlen bis zur 0 und gib ihre Summe aus.

        halt

[reference]
        mov r1, 0          ; r1 = Summe
schleife:
        sys READ_INT       ; r0 = nächste Zahl
        cmp r0, 0          ; Endmarke?
        jz fertig
        add r1, r0         ; Summe += Zahl
        jmp schleife
fertig:
        mov r0, r1
        sys WRITE_INT
        halt

[test]
name = grundfall
in = 3 4 5 0
out = 12

[test]
name = leere-folge
in = 0
out = 0

[test]
name = negativ
in = -7 10 0
out = 3

[test]
name = zwischensumme-null
in = 100 -100 42 0
out = 42

[test]
name = verdeckt-1
hidden = yes
in = 17 -3 999 -1000 5 0
out = 18
````

Labels in the start code and in the hint scaffolds stay the same in both languages (the code is
monolingual); comments in German start code are fine.

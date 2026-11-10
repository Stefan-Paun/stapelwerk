// Demo data for the mock VM: the real chapter plan with all 30 titles from the
// curriculum, and four fully playable levels (0.1, 2.2, 2.3, 6.1). The real
// levels come from levels/*.lvl once the Rust core is built.

export const MOCK_CHAPTERS = [
  { chapter: 0, title: { de: 'Einschalten', en: 'Power On' } },
  { chapter: 1, title: { de: 'Register und Rechnen', en: 'Registers & Arithmetic' } },
  { chapter: 2, title: { de: 'Sprünge und Schleifen', en: 'Jumps & Loops' } },
  { chapter: 3, title: { de: 'Bits und Zahlen', en: 'Bits & Numbers' } },
  { chapter: 4, title: { de: 'Speicher', en: 'Memory' } },
  { chapter: 5, title: { de: 'Stack und Unterprogramme', en: 'Stack & Subroutines' } },
  { chapter: 6, title: { de: 'Rekursion', en: 'Recursion' } },
  { chapter: 7, title: { de: 'Heap und Listen', en: 'Heap & Lists' } },
];

// [id, chapter, order, kind, tags, mobile, title de, title en, card de, card en]
const INDEX = [
  ['0-1-erster-schritt', 0, 1, 'tutorial', ['screen'], true, 'Erster Schritt', 'First Step', 'Ein fertiges Programm malt ein Gesicht; du gehst Schritt für Schritt hindurch.', 'A finished program paints a face; you walk through it step by step.'],
  ['0-2-ein-register-aendern', 0, 2, 'normal', [], true, 'Ein Register ändern', 'Change a Register', 'Register halten Zahlen, und du kannst sie im Debugger ändern.', 'Registers hold numbers, and you can change them in the debugger.'],
  ['1-1-plus-und-minus', 1, 1, 'normal', [], true, 'Plus und Minus', 'Plus and Minus', 'Zwei Zahlen einlesen, festhalten und mit ihnen rechnen.', 'Read two numbers, keep them and calculate with them.'],
  ['1-2-erster-pixel', 1, 2, 'normal', ['screen'], false, 'Erster Pixel', 'First Pixel', 'Der Bildschirm ist Speicher: eine Adresse, ein Byte, ein Pixel.', 'The screen is memory: one address, one byte, one pixel.'],
  ['1-3-ueberschrieben', 1, 3, 'debug', [], true, 'Überschrieben', 'Overwritten', 'Ein Systemaufruf schreibt nach r0, und die Breite geht verloren.', 'A system call writes to r0, and the width gets lost.'],
  ['1-4-sekundenuhr', 1, 4, 'final', [], false, 'Sekundenuhr', 'Stopwatch', 'Ganzzahldivision und Rest machen aus Sekunden eine Uhrzeit.', 'Integer division and remainder turn seconds into a clock time.'],
  ['2-1-die-groessere-zahl', 2, 1, 'normal', [], true, 'Die größere Zahl', 'The Larger One', 'cmp setzt Flags, und bedingte Sprünge lesen sie.', 'cmp sets flags, and conditional jumps read them.'],
  ['2-2-bis-zur-null', 2, 2, 'normal', [], false, 'Bis zur Null', 'Until Zero', 'Eine Schleife, die so lange läuft, wie Daten kommen.', 'A loop that runs for as long as data keeps coming.'],
  ['2-3-rechteck', 2, 3, 'normal', ['screen'], false, 'Rechteck', 'Rectangle', 'Zwei verschachtelte Schleifen füllen ein Rechteck, Zeile für Zeile.', 'Two nested loops fill a rectangle, row by row.'],
  ['2-4-endlosschleife', 2, 4, 'debug', [], true, 'Endlosschleife', 'Never Ending', '„Ungleich null“ ist nicht „größer als null“.', '“Not zero” is not “greater than zero”.'],
  ['2-5-collatz', 2, 5, 'final', [], false, 'Collatz', 'Collatz', 'Schleife und Verzweigung zusammen: eine Folge bis zur 1.', 'Loop and branch together: a sequence down to 1.'],
  ['3-1-einsen-zaehlen', 3, 1, 'normal', [], false, 'Einsen zählen', 'Count the Ones', 'Masken und Schieben zählen die gesetzten Bits.', 'Masks and shifts count the bits that are set.'],
  ['3-2-8x8-zeichen', 3, 2, 'bonus', ['screen'], false, '8×8-Zeichen', '8×8 Glyph', 'Bits prüfen und als Zeichen auf den Bildschirm bringen.', 'Test bits and draw them as a glyph on the screen.'],
  ['3-3-halbe-wahrheit', 3, 3, 'debug', [], true, 'Halbe Wahrheit', 'Half Truth', 'Logisch oder arithmetisch schieben: negative Zahlen merken den Unterschied.', 'Logical or arithmetic shift: negative numbers notice the difference.'],
  ['3-4-hex-ausgabe', 3, 4, 'final', [], false, 'Hex-Ausgabe', 'Hex Dump', 'Aus Nibbles werden ASCII-Zeichen.', 'Nibbles become ASCII characters.'],
  ['4-1-groesster-wert', 4, 1, 'normal', [], false, 'Größter Wert', 'Largest Value', 'Ein Array im Speicher: Basis plus Index mal vier.', 'An array in memory: base plus index times four.'],
  ['4-2-spiegelbild', 4, 2, 'bonus', ['screen'], false, 'Spiegelbild', 'Mirror', 'Zwei Zeiger laufen aufeinander zu und tauschen.', 'Two pointers walk towards each other and swap.'],
  ['4-3-einer-zu-viel', 4, 3, 'debug', [], true, 'Einer zu viel', 'One Too Many', 'Arrays haben keine Grenze, die dich aufhält.', 'Arrays have no boundary that stops you.'],
  ['4-4-sortieren', 4, 4, 'final', [], false, 'Sortieren', 'Sorting', 'Insertion Sort an Ort und Stelle, übersetzt aus C.', 'Insertion sort in place, translated from C.'],
  ['5-1-rueckwaerts', 5, 1, 'normal', [], false, 'Rückwärts', 'Backwards', 'Der Stack ist ein Stapel: was zuletzt kam, geht zuerst.', 'The stack is a pile: last in, first out.'],
  ['5-2-pixel-als-funktion', 5, 2, 'normal', ['screen'], false, 'Pixel als Funktion', 'Pixel Function', 'Ein Unterprogramm nach Aufrufkonvention.', 'A subroutine that follows the calling convention.'],
  ['5-3-der-vergessene-pop', 5, 3, 'debug', [], true, 'Der vergessene Pop', 'The Forgotten Pop', 'Jeder Pfad muss den Stack ausgleichen.', 'Every path has to balance the stack.'],
  ['5-4-linie', 5, 4, 'final', ['screen'], false, 'Linie', 'Line', 'Bresenham aus C übersetzen, mit einer fertigen pixel-Funktion.', 'Translate Bresenham from C, with a ready-made pixel function.'],
  ['6-1-fakultaet', 6, 1, 'normal', [], false, 'Fakultät', 'Factorial', 'Ein Stack-Rahmen pro Aufruf: Prolog, Epilog, lokale Variable.', 'One stack frame per call: prologue, epilogue, local variable.'],
  ['6-2-falscher-fibonacci', 6, 2, 'debug', [], true, 'Falscher Fibonacci', 'Fibonacci Gone Wrong', 'Der rekursive Aufruf zerstört einen Zwischenwert.', 'The recursive call destroys an intermediate value.'],
  ['6-3-flutfuellung', 6, 3, 'bonus', ['screen'], false, 'Flutfüllung', 'Flood Fill', 'Rekursion wird sichtbar, und Tiefe kostet Stack.', 'Recursion becomes visible, and depth costs stack.'],
  ['6-4-tuerme-von-hanoi', 6, 4, 'final', [], false, 'Türme von Hanoi', 'Towers of Hanoi', 'Rekursion mit mehreren Argumenten.', 'Recursion with several arguments.'],
  ['7-1-kettenglieder', 7, 1, 'normal', [], false, 'Kettenglieder', 'Chain Links', 'Speicher anfordern, verketten und wieder freigeben.', 'Request memory, link it up and free it again.'],
  ['7-2-nach-dem-freigeben', 7, 2, 'debug', [], true, 'Nach dem Freigeben', 'After Free', 'Ein Zugriff nach der Freigabe liefert still Müll.', 'Access after free quietly returns garbage.'],
  ['7-3-sortiert-einfuegen', 7, 3, 'final', [], false, 'Sortiert einfügen', 'Sorted Insert', 'Zeigerarbeit mit dem Sonderfall Kopf.', 'Pointer work, including the special case of the head.'],
];

const FULL = {
  '0-1-erster-schritt': {
    budget: 100000,
    text: {
      de: `Dieses Programm ist fertig. Es malt ein Gesicht auf den **Bildschirm**. Du musst nichts schreiben, sondern zusehen, wie die Maschine arbeitet.

- Drück **Schritt** (Taste S) und beobachte, wie sich der **PC** und die Register ändern.
- Setz einen **Haltepunkt** auf die Zeile mit \`auge:\` (Klick in die Zeilenspalte).
- Starte den **Lauf** (L). Die Maschine hält am Haltepunkt.
- Geh einen Schritt **zurück** (Z).

Danach: **Assemblieren & testen**.`,
      en: `This program is finished. It paints a face on the **screen**. You don't have to write anything; watch how the machine works.

- Press **Step** (key S) and watch how the **PC** and the registers change.
- Set a **breakpoint** on the line with \`auge:\` (click the line gutter).
- Start a **run** (L). The machine stops at the breakpoint.
- Go one step **back** (Z).

Then: **Assemble & test**.`,
    },
    hint: {
      de: ['Jeder Schritt führt genau einen Befehl aus. Achte darauf, welche Zeile danach markiert ist.', 'Der Haltepunkt wirkt beim Lauf, nicht beim Einzelschritt. Klick links neben die Zeilennummer von `auge:`.', 'Reihenfolge: S, Haltepunkt setzen, L, Z. Dann Strg+Enter.'],
      en: ['Each step executes exactly one instruction. Watch which line is marked afterwards.', 'A breakpoint acts during a run, not during a single step. Click left of the line number of `auge:`.', 'Order: S, set the breakpoint, L, Z. Then Ctrl+Enter.'],
    },
    learned: {
      de: `Ein Programm ist eine Folge von Befehlen im Speicher. Der **PC** (Programmzähler) zeigt auf den nächsten. Jeder Schritt liest den Befehl, führt ihn aus und rückt den PC um 4 Bytes weiter, weil jeder Befehl 4 Bytes lang ist.

\`stb r2, [r1 + 1300]\` schreibt ein Byte an die Adresse r1 + 1300. Weil der Bildschirm ab \`SCREEN\` im Speicher liegt, erscheint dort ein Pixel.

Echte Debugger (gdb, lldb) können genauso Schritte machen und Haltepunkte setzen. Rückwärts gehen können nur wenige, etwa rr unter Linux.`,
      en: `A program is a sequence of instructions in memory. The **PC** (program counter) points at the next one. Each step reads the instruction, executes it and moves the PC on by 4 bytes, because every instruction is 4 bytes long.

\`stb r2, [r1 + 1300]\` writes one byte to address r1 + 1300. Since the screen lives in memory from \`SCREEN\` on, a pixel appears there.

Real debuggers (gdb, lldb) can step and set breakpoints the same way. Only a few can go backwards, rr on Linux for example.`,
    },
    start: `; Erster Schritt
; Dieses Programm malt ein Gesicht. Geh Schritt für Schritt hindurch.

        li r1, SCREEN          ; r1 = Adresse des Bildschirms
        mov r2, YELLOW         ; r2 = Farbe
auge:
        stb r2, [r1 + 1300]    ; linkes Auge
        stb r2, [r1 + 1301]
        stb r2, [r1 + 1364]
        stb r2, [r1 + 1365]
        stb r2, [r1 + 1322]    ; rechtes Auge
        stb r2, [r1 + 1323]
        stb r2, [r1 + 1386]
        stb r2, [r1 + 1387]
        stb r2, [r1 + 2581]    ; Mundwinkel
        stb r2, [r1 + 2602]
        add r1, 2646           ; Mund ab Spalte 22, Zeile 41
        mov r3, 20             ; 20 Pixel breit
mund:
        stb r2, [r1]
        add r1, 1
        sub r3, 1
        jnz mund
        halt
`,
    tests: [{ name: 'gesicht', expect_screen: 'reference' }],
  },
  '2-2-bis-zur-null': {
    budget: 100000,
    text: {
      de: `Die Eingabe ist eine Folge ganzer Zahlen. Wie viele es sind, weißt du vorher nicht. Das Ende erkennst du an einer **0**. Sie zählt nicht mit, sie ist nur die **Endmarke**.

Gib die Summe aller Zahlen vor der 0 aus.

Beispiel: Eingabe \`3 4 5 0\` → Ausgabe \`12\`.`,
      en: `The input is a sequence of integers. You don't know in advance how many there are. You recognise the end by a **0**. It doesn't count, it is just the **sentinel**.

Output the sum of all numbers before the 0.

Example: input \`3 4 5 0\` → output \`12\`.`,
    },
    hint: {
      de: [
        'Du brauchst eine Schleife: lesen, prüfen, ob es die 0 ist, sonst dazuzählen, wieder von vorn. Überleg zuerst, in welchem Register die Summe die ganze Zeit liegen bleibt.',
        '`cmp r0, 0` rechnet r0 − 0, verwirft das Ergebnis und setzt nur die Flags. Ist r0 null, ist Z gesetzt und `jz fertig` springt. Vorsicht: `sys READ_INT` schreibt jedes Mal nach r0.',
        '```\n        mov r1, 0          ; Summe\nschleife:\n        sys READ_INT\n        cmp r0, 0\n        ; ??? springe zu fertig, wenn Z gesetzt ist\n        ; ??? zähle r0 zur Summe dazu\n        jmp schleife\nfertig:\n        ; ??? Summe nach r0, ausgeben\n        halt\n```',
      ],
      en: [
        "You need a loop: read, check whether it is the 0, otherwise add it, start over. First decide which register keeps the sum the whole time.",
        '`cmp r0, 0` computes r0 − 0, throws the result away and only sets the flags. If r0 is zero, Z is set and `jz fertig` jumps. Careful: `sys READ_INT` writes to r0 every time.',
        '```\n        mov r1, 0          ; sum\nschleife:\n        sys READ_INT\n        cmp r0, 0\n        ; ??? jump to fertig if Z is set\n        ; ??? add r0 to the sum\n        jmp schleife\nfertig:\n        ; ??? sum to r0, output it\n        halt\n```',
      ],
    },
    learned: {
      de: `Du hast eine Schleife gebaut, deren Länge erst die Daten bestimmen. In C:

\`\`\`c
int summe = 0, x;
while ((x = read_int()) != 0) summe += x;
\`\`\`

Die 0 am Ende ist eine **Endmarke**. Zeichenketten in C enden genauso, mit dem Byte 0.

Echte Prozessoren: x86 schreibt \`cmp eax, 0\` und \`je fertig\`, ARM \`cmp r0, #0\` und \`beq fertig\`. **RISC-V hat gar keine Flags:** \`beqz a0, fertig\` vergleicht und springt in einem Befehl.

**Profi-Blick:** Liest du die erste Zahl vor der Schleife und prüfst am Ende mit \`jnz\`, fällt das \`jmp\` weg: mehr Befehle, aber weniger Zyklen pro Zahl. Kurz und schnell sind nicht dasselbe.`,
      en: `You built a loop whose length is decided by the data. In C:

\`\`\`c
int sum = 0, x;
while ((x = read_int()) != 0) sum += x;
\`\`\`

The 0 at the end is a **sentinel**. Strings in C end the same way, with the byte 0.

Real processors: x86 writes \`cmp eax, 0\` and \`je done\`, ARM \`cmp r0, #0\` and \`beq done\`. **RISC-V has no flags at all:** \`beqz a0, done\` compares and jumps in one instruction.

**Pro view:** If you read the first number before the loop and test with \`jnz\` at the end, the \`jmp\` disappears: more instructions, but fewer cycles per number. Short and fast are not the same thing.`,
    },
    start: `; Bis zur Null
; Lies Zahlen bis zur 0 und gib ihre Summe aus.

        halt
`,
    ref: `        mov r1, 0          ; r1 = Summe
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
`,
    tests: [
      { name: 'grundfall', in: [3, 4, 5, 0], out: [12] },
      { name: 'leere-folge', in: [0], out: [0] },
      { name: 'negativ', in: [-7, 10, 0], out: [3] },
      { name: 'zwischensumme-null', in: [100, -100, 42, 0], out: [42] },
      { name: 'verdeckt-1', hidden: true, in: [17, -3, 999, -1000, 5, 0], out: [18] },
      { name: 'verdeckt-2', hidden: true, in: [1, 1, 1, 1, 1, 1, 1, 0], out: [7] },
      { name: 'verdeckt-3', hidden: true, in: [2147483647, 1, 0], out: [-2147483648] },
    ],
  },
  '2-3-rechteck': {
    budget: 200000,
    text: {
      de: `Lies fünf Zahlen: **x**, **y**, **Breite**, **Höhe** und **Farbe**. Füll das Rechteck, dessen linke obere Ecke bei (x, y) liegt.

Ein Pixel (x, y) liegt an der Adresse \`SCREEN + y·64 + x\`. Eine Zeile tiefer heißt also: **+64**.

Beispiel: \`10 8 20 12 YELLOW\` füllt 20 × 12 Pixel ab (10, 8).`,
      en: `Read five numbers: **x**, **y**, **width**, **height** and **colour**. Fill the rectangle whose top left corner is at (x, y).

A pixel (x, y) lives at address \`SCREEN + y·64 + x\`. One row further down therefore means **+64**.

Example: \`10 8 20 12 YELLOW\` fills 20 × 12 pixels from (10, 8).`,
    },
    hint: {
      de: ['Zwei Schleifen ineinander: die äußere zählt Zeilen, die innere Pixel in einer Zeile. Merk dir den Anfang der Zeile getrennt vom laufenden Zeiger.', '`mul r1, r5, 64` rechnet y·64. `stb` schreibt ein Byte. `SCREEN` ist zu groß für `mov`, nimm `li`.', '```\nzeile:\n        cmp r7, 0\n        jz fertig\n        mov r2, r6          ; Spalten\n        mov r0, r1          ; Zeiger\nspalte:\n        ; ??? Pixel setzen, weiter, zählen\n        jnz spalte\n        ; ??? nächste Zeile: +64\n        jmp zeile\n```'],
      en: ['Two loops inside each other: the outer one counts rows, the inner one pixels in a row. Keep the start of the row separate from the moving pointer.', '`mul r1, r5, 64` computes y·64. `stb` writes one byte. `SCREEN` is too large for `mov`, use `li`.', '```\nzeile:\n        cmp r7, 0\n        jz fertig\n        mov r2, r6          ; columns\n        mov r0, r1          ; pointer\nspalte:\n        ; ??? set pixel, advance, count\n        jnz spalte\n        ; ??? next row: +64\n        jmp zeile\n```'],
    },
    learned: {
      de: 'Ein Bild ist eine Tabelle im Speicher, Zeile hinter Zeile. Die Breite einer Zeile ist der **Zeilenabstand** (engl. stride). Grafikkarten und Bildformate arbeiten genauso.',
      en: 'An image is a table in memory, row after row. The width of a row is the **stride**. Graphics cards and image formats work the same way.',
    },
    start: `; Rechteck
; Lies x, y, Breite, Höhe, Farbe und füll das Rechteck.

        halt
`,
    ref: `        sys READ_INT
        mov r4, r0          ; x
        sys READ_INT
        mov r5, r0          ; y
        sys READ_INT
        mov r6, r0          ; Breite
        sys READ_INT
        mov r7, r0          ; Höhe
        sys READ_INT
        mov r3, r0          ; Farbe
        mul r1, r5, 64      ; y*64
        add r1, r4          ; + x
        li r2, SCREEN
        add r1, r2          ; Adresse links oben
zeile:
        cmp r7, 0
        jz fertig
        mov r2, r6          ; Spalten
        mov r0, r1
spalte:
        stb r3, [r0]
        add r0, 1
        sub r2, 1
        jnz spalte
        add r1, 64
        sub r7, 1
        jmp zeile
fertig:
        halt
`,
    tests: [
      { name: 'mitte', in: [10, 8, 20, 12, 14], expect_screen: 'reference' },
      { name: 'ecke', in: [0, 0, 5, 3, 9], expect_screen: 'reference' },
      { name: 'eine-zeile', in: [3, 60, 58, 1, 4], expect_screen: 'reference' },
      { name: 'verdeckt-1', hidden: true, in: [40, 40, 24, 24, 2], expect_screen: 'reference' },
    ],
  },
  '6-1-fakultaet': {
    budget: 200000,
    text: {
      de: `Schreib die Funktion \`fak\`: Sie bekommt **n** in r0 (0 ≤ n ≤ 12) und gibt **n!** in r0 zurück. Löse es **rekursiv**: fak(n) = n · fak(n − 1), fak(0) = fak(1) = 1.

Der Prüfer ruft \`fak\` direkt auf und kontrolliert die **Aufrufkonvention**: r4–r7, fp und sp müssen danach dieselben Werte haben wie vorher.

Jeder Aufruf bekommt einen eigenen **Stack-Rahmen** mit Prolog und Epilog.`,
      en: `Write the function \`fak\`: it receives **n** in r0 (0 ≤ n ≤ 12) and returns **n!** in r0. Solve it **recursively**: fak(n) = n · fak(n − 1), fak(0) = fak(1) = 1.

The checker calls \`fak\` directly and verifies the **calling convention**: r4–r7, fp and sp must hold the same values afterwards as before.

Every call gets its own **stack frame** with prologue and epilogue.`,
    },
    hint: {
      de: ['n muss den rekursiven Aufruf überleben. Wo kann ein Wert liegen, den der Aufgerufene nicht anfassen darf oder nicht findet?', 'Prolog: `push fp` / `mov fp, sp` / `sub sp, 4`. Dann liegt n sicher bei `[fp - 4]`. Epilog: `mov sp, fp` / `pop fp` / `ret`.', '```\nfak:\n        push fp\n        mov fp, sp\n        sub sp, 4\n        st r0, [fp - 4]\n        cmp r0, 1\n        ; ??? Basisfall\n        sub r0, 1\n        call fak\n        ; ??? n holen, multiplizieren\n.ende:\n        mov sp, fp\n        pop fp\n        ret\n```'],
      en: ['n has to survive the recursive call. Where can a value live that the callee must not touch or cannot find?', 'Prologue: `push fp` / `mov fp, sp` / `sub sp, 4`. Then n sits safely at `[fp - 4]`. Epilogue: `mov sp, fp` / `pop fp` / `ret`.', '```\nfak:\n        push fp\n        mov fp, sp\n        sub sp, 4\n        st r0, [fp - 4]\n        cmp r0, 1\n        ; ??? base case\n        sub r0, 1\n        call fak\n        ; ??? fetch n, multiply\n.ende:\n        mov sp, fp\n        pop fp\n        ret\n```'],
    },
    learned: {
      de: `Jeder Aufruf hat seinen eigenen **Rahmen**: alter fp bei \`[fp+0]\`, Rücksprungadresse bei \`[fp+4]\`, lokale Variablen darunter. Die Rahmen bilden eine Kette, die der Debugger abläuft.

So sieht es in C aus:

\`\`\`c
int fak(int n) { return n <= 1 ? 1 : n * fak(n - 1); }
\`\`\`

x86-64 baut denselben Rahmen mit \`push rbp\` / \`mov rbp, rsp\`, ARM64 sichert x29 (Rahmenzeiger) und x30 (Rücksprung) mit einem \`stp\`.`,
      en: `Every call has its own **frame**: old fp at \`[fp+0]\`, return address at \`[fp+4]\`, local variables below. The frames form a chain the debugger walks.

In C:

\`\`\`c
int fak(int n) { return n <= 1 ? 1 : n * fak(n - 1); }
\`\`\`

x86-64 builds the same frame with \`push rbp\` / \`mov rbp, rsp\`, ARM64 saves x29 (frame pointer) and x30 (return address) with one \`stp\`.`,
    },
    start: `; Fakultät
; fak(n): n in r0, Ergebnis in r0. Rekursiv, mit Stack-Rahmen.

fak:
        mov r0, 1
        ret
`,
    ref: `fak:
        push fp
        mov fp, sp
        sub sp, 4
        st r0, [fp - 4]     ; n merken
        cmp r0, 1
        jle .basis
        sub r0, 1
        call fak            ; r0 = fak(n - 1)
        ld r1, [fp - 4]
        mul r0, r1
        jmp .ende
.basis:
        mov r0, 1
.ende:
        mov sp, fp
        pop fp
        ret
`,
    tests: [
      { name: 'fuenf', call: 'fak', regs: { r0: 5 }, expect_reg: { r0: 120 } },
      { name: 'eins', call: 'fak', regs: { r0: 1 }, expect_reg: { r0: 1 } },
      { name: 'null', call: 'fak', regs: { r0: 0 }, expect_reg: { r0: 1 } },
      { name: 'verdeckt-1', hidden: true, call: 'fak', regs: { r0: 12 }, expect_reg: { r0: 479001600 }, seed: 61 },
    ],
  },
};

export const MOCK_LEVELS = INDEX.map(([id, chapter, order, kind, tags, mobile, tde, ten, cde, cen]) => ({
  id, chapter, order, kind, tags, mobile,
  title: { de: tde, en: ten },
  card: { de: cde, en: cen },
  tests: [],
  ...(FULL[id] ?? {}),
  // The tutorial's start code is its own reference solution.
  ...(kind === 'tutorial' && FULL[id] ? { ref: FULL[id].start } : {}),
}));

// Deutsche Oberflächentexte. Flache Schlüssel, Platzhalter in {geschweiften Klammern}.
// en.js muss dieselben Schlüssel und Platzhalter haben (test/unit/i18n.test.mjs).
export default {
  'meta.tagline': 'Assembler-Rätsel auf einer eigenen CPU',

  'nav.main': 'Hauptnavigation',
  'nav.skip': 'Zum Inhalt',
  'nav.home': 'Kapitelplan',
  'nav.plan': 'Kapitelplan',
  'nav.sandbox': 'Sandkasten',
  'nav.manual': 'Handbuch',
  'nav.about': 'Über',
  'nav.crumbs': 'Pfad',

  'theme.toDark': 'Dunkles Thema (Blaupause)',
  'theme.toLight': 'Helles Thema (Reißbrett)',

  'boot.failed': 'Die Maschine ließ sich nicht starten.',
  'boot.failedText': 'Lade die Seite neu. Wenn es wieder passiert, unterstützt dein Browser vermutlich kein WebAssembly.',
  'boot.internal': 'Interner Fehler in der Maschine. Sie wurde neu gestartet; dein Code ist noch da.',
  'boot.viewFailed': 'Diese Ansicht ist abgestürzt.',
  'boot.progressReset': 'Der gespeicherte Fortschritt war beschädigt und wurde nicht geladen.',

  'notfound.title': 'Diese Seite gibt es nicht.',
  'notfound.text': 'Der Link zeigt auf keine bekannte Ansicht. Im Kapitelplan findest du alle Levels.',

  'common.changed': 'geändert',
  'common.cancel': 'Abbrechen',
  'common.close': 'Schließen',

  'chapter.0': 'Einschalten',
  'chapter.1': 'Register und Rechnen',
  'chapter.2': 'Sprünge und Schleifen',
  'chapter.3': 'Bits und Zahlen',
  'chapter.4': 'Speicher',
  'chapter.5': 'Stack und Unterprogramme',
  'chapter.6': 'Rekursion',
  'chapter.7': 'Heap und Listen',

  'plan.title': 'Kapitelplan',
  'plan.lede': 'Dreißig Rätsel auf einer kleinen 32-Bit-Maschine, von Registern bis zum Heap. Jedes Level lehrt, was ein echter Prozessor und ein C-Compiler tun.',
  'plan.continue': 'Weiter mit {num} {title}',
  'plan.solved': '{done} von {total} gelöst',
  'plan.unlockAll': 'Alles freischalten',
  'plan.chapter': 'Kapitel {n} · {title}',
  'plan.chapterShort': 'Kapitel',
  'plan.progress': '{done}/{total}',
  'plan.state.solved': 'gelöst',
  'plan.state.open': 'offen',
  'plan.state.locked': 'gesperrt',
  'plan.mockNote': 'Demo-Modus: Die Maschine läuft als JavaScript-Nachbau, vier Levels sind spielbar. Mit dem echten Kern kommen alle dreißig.',

  'kind.tutorial': 'Einführung',
  'kind.debug': 'Fehlersuche',
  'kind.final': 'Abschluss',
  'kind.bonus': 'Kür',
  'kind.screen': 'Bild',
  'kind.mobile': 'Handy',

  'level.notPlayable': 'Dieses Level ist im Demo-Modus noch nicht enthalten. Aufgabe, Tests und Startcode kommen mit dem echten Kern.',

  'ws.views': 'Ansichten',
  'ws.tab.task': 'Level',
  'ws.tab.code': 'Code',
  'ws.tab.regs': 'Register',
  'ws.tab.stack': 'Stack',
  'ws.tab.screen': 'Bild',
  'ws.tab.console': 'Konsole',
  'ws.tab.mem': 'Speicher',

  'dbg.label': 'Debugger',
  'dbg.assembleTest': 'Assemblieren & testen',
  'dbg.assemble': 'Assemblieren',
  'dbg.assembleShort': 'Bauen',
  'dbg.testShort': 'Testen',
  'dbg.modEnter': 'Strg ↵',
  'dbg.restart': 'Neustart',
  'dbg.back': 'Zurück',
  'dbg.step': 'Schritt',
  'dbg.run': 'Lauf',
  'dbg.pause': 'Pause',
  'dbg.timeline': 'Zeitleiste',
  'dbg.stepOf': 'Schritt {step} von {total}',
  'dbg.cycles': '{cycles} Zyklen',
  'dbg.testcase': 'Testfall',

  'status.not_assembled': 'Nicht assembliert',
  'status.paused': 'Pausiert',
  'status.halted': 'Angehalten (halt)',
  'status.fault': 'Laufzeitfehler',
  'status.breakpoint': 'Am Haltepunkt',
  'status.waiting': 'Wartet auf Eingabe',
  'status.budget': 'Zyklenbudget erschöpft',
  'status.running': 'Läuft',
  'status.dirty': 'Code geändert',
  'status.tests': 'Tests {passed}/{total}',

  'task.tabs': 'Aufgabe',
  'task.task': 'Aufgabe',
  'task.hints': 'Hinweise',
  'task.tests': 'Tests',
  'task.result': 'Ergebnis',

  'tutorial.title': 'Checkliste',
  'tutorial.step': 'Einen Schritt gemacht',
  'tutorial.breakpoint': 'Einen Haltepunkt gesetzt',
  'tutorial.run': 'Bis zum Haltepunkt gelaufen',
  'tutorial.back': 'Einen Schritt zurückgegangen',
  'tutorial.done': 'erledigt',
  'tutorial.open': 'offen',

  'hints.intro': 'Drei Hinweise, vom Gedanken bis fast zur Lösung. Nimm nur so viel, wie du brauchst.',
  'hints.n': 'Hinweis {n} von {total}',
  'hints.first': 'Ersten Hinweis zeigen',
  'hints.next': 'Hinweis {n} zeigen',

  'tests.notRun': 'Noch nicht getestet. Assemblieren & testen startet alle Tests.',
  'tests.allPassed': 'Alle Tests bestanden.',
  'tests.someFailed': '{passed} von {total} Tests bestanden.',
  'tests.passed': 'bestanden',
  'tests.failed': 'nicht bestanden',
  'tests.pending': 'noch nicht gelaufen',
  'tests.cycles': '{n} Zyklen',
  'tests.screen': 'Bild',
  'tests.open': 'Im Debugger',
  'tests.openLabel': 'Test {name} im Debugger öffnen',
  'tests.hidden': '{count} verdeckte Tests',
  'tests.hiddenNote': 'Eingaben bleiben verborgen, bis einer scheitert.',
  'tests.hiddenResult': '{passed} von {total} bestanden',

  'editor.title': 'Programm',
  'editor.label': 'Programm (Assembler)',
  'editor.size': '{n} Befehlswörter',
  'editor.linecol': 'Z. {line}:{col}',
  'editor.toggleBp': 'Haltepunkt setzen oder entfernen',
  'editor.palette': 'Befehle einfügen',

  'notice.fault': 'Zeile {line}: {text}',
  'notice.budget': 'Nach {cycles} Zyklen abgebrochen. Am häufigsten lief Zeile {line}; vermutlich endet eine Schleife nie.',
  'notice.solved': 'Gelöst. Alle Tests bestanden.',
  'notice.testsFailed': '{failed} von {total} Tests nicht bestanden.',
  'notice.assembled': 'Assembliert. Schritt oder Lauf startet das Programm.',

  'regs.title': 'Register',
  'regs.flags': 'Flags',
  'regs.pcNote': 'pc {pc} · Zeile {line}',
  'regs.callerSaved': 'r0–r3 · Aufrufer sichert',
  'regs.calleeSaved': 'r4–r7 · Aufgerufener sichert',
  'regs.pointers': 'Rahmen- und Stackzeiger',
  'regs.value': '{reg} = {value} (hex {hex}), bearbeiten',
  'regs.valueChanged': '{reg} = {value} (hex {hex}), geändert, vorher {before}, bearbeiten',
  'regs.editLabel': 'Neuer Wert für {reg} (dezimal, 0x… oder 0b…)',
  'regs.editHint': 'Klicken zum Bearbeiten (nur pausiert)',
  'regs.pausedOnly': 'Bearbeiten geht nur, wenn die Maschine pausiert',
  'regs.backToChange': 'Zurück bis zur letzten Änderung von {reg}',
  'regs.flagLabel': 'Flag {flag} ({meaning}) = {value}',
  'regs.flag.Z': 'Null: Ergebnis war 0',
  'regs.flag.N': 'Negativ: Bit 31 des Ergebnisses',
  'regs.flag.C': 'Übertrag oder Borgen (vorzeichenlos)',
  'regs.flag.V': 'Überlauf (vorzeichenbehaftet)',

  'stack.title': 'Stack',
  'stack.caption': 'Stack von der höchsten Adresse bis sp, mit Rahmen',
  'stack.pointer': 'Zeiger',
  'stack.addr': 'Adresse',
  'stack.value': 'Wert',
  'stack.meaning': 'Bedeutung',
  'stack.note': 'sp {sp} · {frames} Rahmen',
  'stack.oldFp': 'alter fp',
  'stack.chainEnd': 'Ende der Kette',
  'stack.ret': 'Rücksprung',
  'stack.line': 'Z. {line}',
  'stack.checker': 'Prüfer',
  'stack.local': 'lokal',
  'stack.arg': 'Argument',
  'stack.pushed': 'gepusht',
  'stack.trap': 'Rücksprung zum Prüfer',
  'stack.frameCurrent': 'aktueller Rahmen',
  'stack.frameN': 'Rahmen {n}',
  'stack.empty': 'Der Stack ist leer: sp steht ganz oben bei F000.',
  'stack.notAssembled': 'Erst assemblieren, dann zeigt der Stack Rahmen und Werte.',
  'stack.truncated': 'Weitere Einträge weiter oben ausgeblendet.',

  'screen.title': 'Bildschirm',
  'screen.modeLabel': 'Anzeige',
  'screen.mode.ist': 'Ist',
  'screen.mode.soll': 'Soll',
  'screen.mode.diff': 'Unterschied',
  'screen.aria': 'Bildschirm 64 × 64, Ansicht {mode}, {count} Pixel nicht schwarz',
  'screen.ariaDiff': 'Unterschied zum Soll: {count} Pixel falsch',
  'screen.hoverHint': 'Zeig auf ein Pixel: Koordinate, Adresse, Wert und Farbe',

  'console.title': 'Konsole',
  'console.input': 'Eingabe',
  'console.output': 'Ausgabe',
  'console.expected': 'Erwartet',
  'console.empty': 'noch keine Ausgabe',
  'console.noInput': 'keine Eingabe',
  'console.inputInteractive': 'Eingaben tippst du unten ein.',
  'console.note': '{read}/{total} gelesen · {out} ausgegeben',
  'console.waiting': 'Das Programm wartet auf eine Eingabe.',
  'console.inputField': 'Eingabe für das Programm',
  'console.inputKind': 'Art der Eingabe',
  'console.kind.int': 'Zahlen',
  'console.kind.text': 'Text',
  'console.send': 'Senden',
  'console.close': 'Eingabe schließen',
  'console.cut': '{count} ältere Einträge ausgeblendet',

  'mem.tabs': 'Speicher und Nachschlagen',
  'mem.hex': 'Hexdump',
  'mem.heap': 'Heap',
  'mem.manual': 'Handbuch',

  'hex.title': 'Hexdump',
  'hex.segments': 'Segment',
  'hex.unit': 'Einheit',
  'hex.byte': 'Byte',
  'hex.word': 'Wort',
  'hex.scrollLabel': 'Hexdump, mit Pfeiltasten scrollen',

  'seg.null': 'null',
  'seg.text': 'text',
  'seg.data': 'data',
  'seg.input': 'input',
  'seg.heap': 'heap',
  'seg.guard': 'guard',
  'seg.stack': 'stack',
  'seg.screen': 'screen',
  'seg.rights.text': 'lesen, ausführen',
  'seg.rights.data': 'lesen, schreiben',
  'seg.rights.input': 'lesen, schreiben · Testeingaben',
  'seg.rights.heap': 'nur über ALLOC',
  'seg.rights.stack': 'lesen, schreiben · wächst nach unten',
  'seg.rights.screen': 'lesen, schreiben · 64 × 64 Pixel',

  'heap.title': 'Heap-Karte',
  'heap.aria': 'Heap-Karte: {live} lebende von {total} Blöcken',
  'heap.state.free': 'frei',
  'heap.state.live': 'belegt',
  'heap.state.freed': 'freigegeben',
  'heap.state.red': 'Rotzone (8 B hinter jedem Block)',
  'heap.blocks': 'Heap-Blöcke',
  'heap.col.addr': 'Adresse',
  'heap.col.size': 'Größe',
  'heap.col.status': 'Status',
  'heap.col.line': 'Zeile',
  'heap.live': 'lebt',
  'heap.freed': 'freigegeben',
  'heap.empty': 'Noch kein ALLOC. Jeder Block erscheint hier mit der Zeile, die ihn angefordert hat.',

  'result.stamp': 'Bestanden',
  'result.title': 'Gelöst',
  'result.sub': 'Alle {tests} Tests bestanden, auch die verdeckten.',
  'result.tutorialMissing': 'Die Tests laufen, aber die Checkliste ist noch nicht vollständig.',
  'result.metrics': 'Kennzahlen im Vergleich zur Referenzlösung',
  'result.col.metric': 'Kennzahl',
  'result.col.yours': 'Deine',
  'result.col.ref': 'Referenz',
  'result.col.delta': 'Abstand',
  'result.cycles': 'Zyklen (Mittel über alle Tests)',
  'result.size': 'Größe (Befehlswörter)',
  'result.within': 'erreicht',
  'result.over': '+{n} {unit}',
  'result.unit.cycles': 'Zyklen',
  'result.unit.words': 'Wörter',
  'result.badge.compact': 'Kompakt',
  'result.badge.compact.rule': 'höchstens so viele Befehlswörter wie die Referenz',
  'result.badge.fast': 'Flink',
  'result.badge.fast.rule': 'höchstens so viele Zyklen wie die Referenz',
  'result.earned': 'verdient',
  'result.notEarned': 'noch nicht verdient',
  'result.learned': 'Was du gerade gelernt hast',
  'result.slots': 'Lösung aufheben (bis zu drei Plätze pro Level):',
  'result.slotSave': 'In Platz {n}',
  'result.slotReplace': 'Platz {n} ersetzen',
  'result.slotSaved': 'Platz {n} gespeichert',
  'result.next': 'Weiter: {num} {title}',
  'result.toPlan': 'Zum Kapitelplan',

  'sandbox.title': 'Sandkasten',
  'sandbox.intro': 'Freies Programm ohne Aufgabe und ohne Tests. Eingaben tippst du in die Konsole, während das Programm wartet.',
  'sandbox.examples': 'Beispiele',
  'sandbox.ex.count': 'Zählen',
  'sandbox.ex.count.note': 'eine Schleife gibt 1 bis 10 aus',
  'sandbox.ex.echo': 'Verdoppeln',
  'sandbox.ex.echo.note': 'liest deine Zahlen und wartet auf Eingabe',
  'sandbox.ex.hello': 'Text',
  'sandbox.ex.hello.note': 'ein String im data-Segment',
  'sandbox.ex.list': 'Liste',
  'sandbox.ex.list.note': 'drei Knoten auf dem Heap',
  'sandbox.ex.flood': 'Flutfüllung',
  'sandbox.ex.flood.note': 'Rekursion, bis der Stack überläuft',
  'sandbox.share': 'Teilen',
  'sandbox.shareNote': 'Der Code steckt komprimiert im Link. Nichts wird hochgeladen, und geöffnet startet er nie von selbst.',
  'sandbox.shareButton': 'Link erzeugen',
  'sandbox.shareLink': 'Teilen-Link',
  'sandbox.copied': 'Link kopiert.',
  'sandbox.copyManual': 'Kopieren ging nicht automatisch; der Link ist markiert.',
  'sandbox.badLink': 'Der geteilte Link ist beschädigt; der Code konnte nicht gelesen werden.',

  'manual.toc': 'Themen',
  'manual.pick': 'Thema wählen',
  'manual.topic': 'Thema',

  'manual.register.title': 'Register',
  'manual.register.body': `Ein **Register** ist ein Speicherplatz direkt im Prozessor, 32 Bit breit. Stapelwerk hat zehn: \`r0\` bis \`r7\`, \`fp\` und \`sp\`.

- \`r0\`–\`r3\`: Argumente und Rückgabewert. Wer eine Funktion aufruft, muss sie selbst sichern (*caller-saved*).
- \`r4\`–\`r7\`: muss die aufgerufene Funktion unverändert zurückgeben (*callee-saved*).
- \`fp\`: Rahmenzeiger, zeigt auf den aktuellen [Stack-Rahmen](handbuch:aufruf).
- \`sp\`: Stackzeiger, startet bei \`0xF000\`.

Der **PC** (Programmzähler) zeigt auf den nächsten Befehl. Du kannst ihn nicht direkt beschreiben, nur mit Sprüngen ändern. Im Pausenzustand kannst du Register im Debugger ändern: Klick auf den Wert.`,

  'manual.befehle.title': 'Befehle',
  'manual.befehle.body': `Jeder Befehl ist genau 4 Bytes lang. Rechenbefehle haben drei Operanden oder eine Kurzform mit zwei: \`add r1, r2\` heißt \`add r1, r1, r2\`.

- Laden und Setzen: \`mov\`, \`li\` (32-Bit-Konstante, zwei Wörter), \`movhi\`, \`lea\`
- Rechnen: \`add sub mul div divu rem remu neg\`
- Bits: \`and or xor not shl shr sar\`
- Vergleichen: \`cmp\` (wie \`sub\`, nur Flags), \`tst\` (wie \`and\`, nur Flags)
- Speicher: \`ld st\` (Wort), \`ldb ldbs stb\` (Byte)
- Stack: \`push pop call ret\`
- Sprünge: \`jmp jz jnz jlt jge jle jgt jb jae jbe ja js jns jo jno\`
- Sonst: \`sys\`, \`nop\`, \`halt\`

Zyklen: die meisten Befehle 1, Speicher und Stack 2, \`mul\` 3, Division 8, \`sys\` 4.`,

  'manual.flags.title': 'Flags',
  'manual.flags.body': `Rechenbefehle setzen vier **Flags**:

- **Z** (Null): das Ergebnis war 0.
- **N** (Negativ): Bit 31 des Ergebnisses ist gesetzt.
- **C** (Carry): bei \`add\` ein Übertrag aus Bit 31, bei \`sub\` und \`cmp\` ein **Borgen**, also a < b ohne Vorzeichen.
- **V** (Overflow): das Ergebnis passt nicht in eine vorzeichenbehaftete 32-Bit-Zahl.

\`cmp a, b\` rechnet a − b und behält nur die Flags. Danach liest ein bedingter Sprung sie: \`jlt\` für kleiner mit Vorzeichen, \`jb\` für kleiner ohne Vorzeichen. \`mov\`, \`ld\` und Sprünge lassen die Flags unverändert.`,

  'manual.zweierkomplement.title': 'Zweierkomplement',
  'manual.zweierkomplement.body': `Negative Zahlen sind Bits wie alle anderen. Im **Zweierkomplement** ist −1 die Zahl \`0xFFFFFFFF\`: alle 32 Bits gesetzt. Um eine Zahl zu negieren, kippst du alle Bits und zählst 1 dazu.

Dieselben Bits bedeuten ohne Vorzeichen 4 294 967 295 und mit Vorzeichen −1. Der Prozessor rechnet beides gleich; erst die Flags und die Sprungbefehle entscheiden, wie du das Ergebnis liest.

- \`shr\` schiebt Nullen nach (logisch), \`sar\` kopiert das Vorzeichenbit (arithmetisch).
- Überlauf passiert modulo 2³², nie mit Fehler.`,

  'manual.speicher.title': 'Speicher',
  'manual.speicher.body': `Die Maschine hat 64 KiB Speicher in Segmenten mit eigenen Rechten:

- \`0000\`–\`00FF\` null: jeder Zugriff ist ein Fehler.
- \`0100\`–\`3FFF\` text: dein Code, nur lesen und ausführen.
- \`4000\`–\`5FFF\` data: deine Daten.
- \`6000\`–\`7FFF\` input: Eingabedaten der Tests (\`INPUT\`).
- \`8000\`–\`BFFF\` heap: nur über \`sys ALLOC\`.
- \`C000\`–\`CFFF\` guard: Schutzbereich.
- \`D000\`–\`EFFF\` stack.
- \`F000\`–\`FFFF\` screen: der [Bildschirm](handbuch:bildschirm).

Wörter liegen **little-endian**: das niedrigste Byte an der niedrigsten Adresse. Wortzugriffe brauchen eine durch 4 teilbare Adresse.`,

  'manual.stack.title': 'Stack',
  'manual.stack.body': `Der **Stack** (Stapel) liegt am oberen Ende des Speichers und wächst nach unten. \`push r1\` zieht 4 von sp ab und legt r1 dort ab; \`pop r1\` holt den Wert und zählt 4 dazu. Was zuletzt kam, geht zuerst.

\`call ziel\` legt die Rücksprungadresse auf den Stack und springt; \`ret\` holt sie wieder. Jeder Pfad durch eine Funktion muss den Stack genau so verlassen, wie er ihn vorgefunden hat.

In der Stack-Ansicht stehen hohe Adressen oben. Die Maßketten links zeigen, wie groß der aktuelle Rahmen ist und dass die Rücksprungadresse immer 4 Bytes über fp liegt.`,

  'manual.aufruf.title': 'Aufrufkonvention',
  'manual.aufruf.body': `Damit Funktionen zusammenpassen, gilt eine **Aufrufkonvention**:

- Argumente 1–4 in \`r0\`–\`r3\`, weitere auf dem Stack. Rückgabe in \`r0\`.
- \`r0\`–\`r3\` darf die Funktion zerstören; \`r4\`–\`r7\` und \`fp\` muss sie unverändert zurückgeben; sp muss nach \`ret\` wieder stimmen.

Prolog und Epilog bauen einen **Rahmen**:

\`\`\`
        push fp
        mov fp, sp
        sub sp, 8        ; Platz für lokale Variablen
        ...
        mov sp, fp
        pop fp
        ret
\`\`\`

Im Rahmen liegt bei \`[fp+0]\` der alte fp, bei \`[fp+4]\` die Rücksprungadresse, darunter die lokalen Variablen.`,

  'manual.heap.title': 'Heap',
  'manual.heap.body': `Der **Heap** ist Speicher, den du zur Laufzeit anforderst: \`sys ALLOC\` mit der Größe in r0 liefert eine Adresse in r0 (oder 0, wenn kein Platz ist). \`sys FREE\` gibt den Block zurück.

- Neuer Speicher ist mit \`CD\` gefüllt, freigegebener mit \`DD\`; im Hexdump siehst du das sofort.
- Hinter jedem Block liegen 8 Bytes **Rotzone**. Wer darüber hinaus schreibt, löst einen Fehler aus.
- Zugriff nach \`FREE\` und doppeltes Freigeben werden erkannt.

Die Heap-Karte zeigt jeden 4-Byte-Abschnitt mit eigenem Muster: frei, belegt, freigegeben, Rotzone.`,

  'manual.bildschirm.title': 'Bildschirm',
  'manual.bildschirm.body': `Der Bildschirm hat 64 × 64 Pixel und ist ganz normaler Speicher ab \`SCREEN\` (\`0xF000\`): ein Byte pro Pixel, Zeile für Zeile. Pixel (x, y) liegt bei \`SCREEN + y·64 + x\`.

Die unteren 4 Bits wählen eine von 16 Farben. Die Namen sind Konstanten: \`BLACK BLUE GREEN CYAN RED MAGENTA BROWN LIGHTGRAY DARKGRAY LIGHTBLUE LIGHTGREEN LIGHTCYAN LIGHTRED PINK YELLOW WHITE\`.

Bei Bild-Levels zeigt **Soll** das erwartete Bild und **Unterschied** jedes falsche Pixel schraffiert.`,

  'manual.systemaufrufe.title': 'Systemaufrufe',
  'manual.systemaufrufe.body': `\`sys NAME\` ruft das Betriebssystem der Maschine. Systemaufrufe ändern nur r0 und kosten 4 Zyklen.

- \`READ_INT\`: nächste Zahl nach r0
- \`WRITE_INT\`: r0 als Zahl ausgeben
- \`READ_CHAR\`: nächstes Zeichen nach r0, am Ende −1
- \`WRITE_CHAR\`: unterstes Byte von r0 ausgeben
- \`WRITE_STR\`: Zeichenkette ab Adresse r0 bis zum Byte 0
- \`ALLOC\`, \`FREE\`: [Heap](handbuch:heap)
- \`RAND\`: Zufallszahl nach r0

Wichtig: Weil Systemaufrufe nach r0 schreiben, ist ein Wert dort danach verloren.`,

  'manual.debugger.title': 'Debugger',
  'manual.debugger.body': `- **Schritt** (S) führt genau einen Befehl aus.
- **Zurück** (Z) geht einen Schritt zurück, auch nach einem Fehler.
- **Lauf** (L) läuft bis zum nächsten **Haltepunkt**, bis \`halt\` oder bis zu einem Fehler.
- **Haltepunkte** setzt du per Klick in die Zeilenspalte oder mit B.
- Die **Zeitleiste** springt zu jedem Schritt, den die Maschine schon gemacht hat.
- Neben jedem Register führt ein Knopf zurück bis zu seiner **letzten Änderung**.

Was ein Schritt verändert hat, ist schraffiert und rot unterstrichen; der alte Wert steht durchgestrichen daneben. Die Zahlen links neben dem Code zählen, wie oft jede Zeile lief.`,

  'help.title': 'Tastenkürzel',
  'help.key': 'Taste',
  'help.action': 'Wirkung',
  'help.step': 'Einen Schritt ausführen',
  'help.back': 'Einen Schritt zurück',
  'help.runPause': 'Lauf starten oder anhalten',
  'help.restart': 'Neustart des Testfalls',
  'help.breakpoint': 'Haltepunkt an der aktuellen Zeile (im Editor: Strg+B)',
  'help.tests': 'Tests anzeigen',
  'help.help': 'Diese Übersicht',
  'help.assembleTest': 'Assemblieren und testen',
  'help.leaveEditor': 'Editor verlassen',
  'help.tab': 'Im Editor: Tabulator einfügen (vorher Esc, um weiterzuspringen)',
  'help.note': 'Buchstaben-Kürzel ruhen, solange du im Editor tippst. Mit Strg (⌘) gehen dort Schritt, Haltepunkt und Testen.',

  'live.step': 'Zeile {line}: {instr}. {changes}',
  'live.noChange': 'Keine Änderung',
  'live.bytes': '{count} Byte geändert',
  'live.back': 'Zurück zu Schritt {step}, Zeile {line}',
  'live.atStart': 'Schon am Anfang.',
  'live.restart': 'Neustart.',
  'live.paused': 'Pausiert bei Schritt {step}.',
  'live.stopped': '{status}, Zeile {line}.',
  'live.asmErrors': '{count} Assemblerfehler.',
  'live.assembled': 'Assembliert.',
  'live.solved': 'Gelöst: {cycles} Zyklen, {size} Befehlswörter.',
  'live.testsFailed': '{failed} Tests nicht bestanden.',
  'live.noInstr': 'In Zeile {line} steht kein Befehl.',
  'live.bpOn': 'Haltepunkt in Zeile {line}.',
  'live.bpOff': 'Haltepunkt in Zeile {line} entfernt.',
  'live.regSet': '{reg} auf {value} gesetzt.',
  'live.backToChange': 'Zurück vor die letzte Änderung von {reg}: Schritt {step}, Zeile {line}.',

  'about.title': 'Über Stapelwerk',
  'about.lede': 'Ein offenes Lernspiel über Rechnerarchitektur: kein Konto, keine Daten, läuft offline.',
  'about.what.title': 'Was das ist',
  'about.what.body': `Stapelwerk schließt die Lücke zwischen den Modellrechnern der Schule (Johnny, Bonsai, Little Man Computer) und den Uni-Werkzeugen (MARS, RARS, Ripes). Die Maschine ist klein genug für einen Nachmittag, hat aber alles, was im ersten Semester abgeprüft wird: Zweierkomplement, Flags, Stack-Rahmen, Aufrufkonvention, Heap und speicherabgebildete Ein- und Ausgabe.

Die CPU ist erfunden. Sie hat kein Pipeline- und kein Cache-Modell. Was ein Level über echte Prozessoren sagt, gilt aber für x86, ARM und RISC-V.`,
  'about.privacy.title': 'Datenschutz',
  'about.privacy.body': `Stapelwerk setzt keine Cookies, zählt keine Besuche und lädt nichts von fremden Servern; auch die Schriften liegen hier. Dein Fortschritt bleibt im Speicher deines Browsers (localStorage) und verlässt dein Gerät nur, wenn du ihn selbst exportierst.`,
  'about.progress.title': 'Fortschritt',
  'about.progress.body': 'Gelöste Levels, Bestwerte, Abzeichen und dein Code liegen nur in diesem Browser. Als Datei kannst du alles sichern oder auf ein anderes Gerät mitnehmen.',
  'about.export': 'Exportieren',
  'about.import': 'Importieren',
  'about.delete': 'Fortschritt löschen',
  'about.exported': 'Datei gespeichert.',
  'about.imported': 'Fortschritt geladen.',
  'about.importFailed': 'Die Datei ist kein Stapelwerk-Fortschritt ({error}).',
  'about.importTooBig': 'Die Datei ist zu groß.',
  'about.deleteTitle': 'Fortschritt löschen?',
  'about.deleteText': 'Alle gelösten Levels, Abzeichen und dein Code in diesem Browser werden gelöscht. Das lässt sich nicht rückgängig machen; exportier vorher, wenn du sicher sein willst.',
  'about.deleteConfirm': 'Endgültig löschen',
  'about.deleted': 'Fortschritt gelöscht.',
  'about.keys.title': 'Tastenkürzel',
  'about.licenses.title': 'Lizenzen',
  'about.licenses.body': 'Der Code steht unter der MIT-Lizenz. Die Schriften Sofia Sans und IBM Plex Mono stehen unter der SIL Open Font License 1.1:',
  'about.fontSofia': 'Sofia Sans (OFL 1.1)',
  'about.fontPlex': 'IBM Plex Mono (OFL 1.1)',
  'about.ai.title': 'Wie das gebaut wurde',
  'about.ai.body': 'Stapelwerk wurde mit Claude-Code-Agenten gebaut. Idee, Auswahl der Inhalte und Kapitelplan kommen von Stefan Paun; die Agenten haben unter seiner Anleitung Code, Levels und Texte geschrieben. Geprüft wird mit automatischen Tests und mit Spieltests.',
  'about.engine.title': 'Maschine',
  'about.engineMock': 'Gerade läuft der Demo-Nachbau in JavaScript. Er bildet einen Teil der Befehle nach und dient der Entwicklung der Oberfläche.',
  'about.engineReal': 'Die Maschine läuft als WebAssembly-Kern, geschrieben in Rust ohne fremde Bibliotheken.',

  'access.r': 'Lesen',
  'access.w': 'Schreiben',

  'asm.summary': '{count} Fehler beim Assemblieren. Die Liste steht unter dem Code.',
  'asm.suggestion': 'Meintest du „{suggestion}“?',
  'asm.hint_li': 'Für große Zahlen gibt es „li“.',
  'asm.unknown_mnemonic': 'Unbekannter Befehl „{name}“.',
  'asm.unknown_register': 'Unbekanntes Register „{name}“. Es gibt r0–r7, fp und sp.',
  'asm.operand_count': 'Erwartet {expected} Operanden, gefunden {got}.',
  'asm.operand_type': 'Hier gehört ein anderer Operand hin ({expected}).',
  'asm.imm_range': 'Der Wert {value} liegt außerhalb von {min} bis {max}.',
  'asm.undefined_symbol': '„{name}“ ist nirgends definiert.',
  'asm.duplicate_label': 'Die Marke „{name}“ gibt es schon in Zeile {first_line}.',
  'asm.reserved_name': '„{name}“ ist ein reservierter Name.',
  'asm.bad_number': 'Diese Zahl kann ich nicht lesen.',
  'asm.bad_string': 'Diese Zeichenkette ist nicht korrekt abgeschlossen oder hat ein unbekanntes Escape.',
  'asm.unexpected_char': 'Unerwartetes Zeichen „{ch}“.',
  'asm.local_label_without_global': 'Eine lokale Marke (.name) braucht davor eine normale Marke.',
  'asm.needs_constant': 'Hier muss eine Konstante stehen, die vorher definiert ist.',
  'asm.expr_div_zero': 'Division durch 0 im Ausdruck.',
  'asm.expr_overflow': 'Der Ausdruck läuft über.',
  'asm.data_in_text': 'Daten gehören in den Abschnitt .data.',
  'asm.code_in_data': 'Befehle gehören in den Abschnitt .text.',
  'asm.section_overflow': 'Abschnitt {section} ist mit {size} Bytes zu groß (höchstens {max}).',
  'asm.forbidden_instruction': '„{mnemonic}“ ist in diesem Level nicht erlaubt.',
  'asm.too_many_errors': 'Zu viele Fehler; hier höre ich auf.',

  'fault.illegal_instruction': 'Ungültiger Befehl {word}.',
  'fault.zero_instruction': 'Der Prozessor ist auf ein leeres Wort gelaufen. halt vergessen?',
  'fault.exec_outside_code': 'Ausführung außerhalb des Codes.',
  'fault.bad_jump_target': 'Sprungziel {target} ist kein Befehl im Code.',
  'fault.bad_return_address': 'ret springt nach {target}, dort ist kein Code. Steht oben auf dem Stack wirklich die Rücksprungadresse?',
  'fault.null_access': '{access} an Adresse {addr}: der Bereich ab 0 ist gesperrt (Nullzeiger?).',
  'fault.out_of_range': '{access} an Adresse {addr}: außerhalb des Speichers.',
  'fault.guard_page': '{access} an Adresse {addr}: Schutzbereich zwischen Heap und Stack.',
  'fault.write_to_code': 'Schreiben nach {addr}: Code darf nicht verändert werden.',
  'fault.misaligned': 'Adresse {addr} ist für einen Zugriff mit {size} Bytes nicht ausgerichtet (·4?).',
  'fault.stack_overflow': 'Stapelüberlauf: sp wäre {sp}.',
  'fault.stack_underflow': 'Stapelunterlauf: sp wäre {sp}. Mehr pop als push?',
  'fault.stack_misaligned': 'sp {sp} ist nicht durch 4 teilbar.',
  'fault.div_zero': 'Division durch 0.',
  'fault.input_end': 'Die Eingabe ist zu Ende, das Programm will aber weiterlesen.',
  'fault.heap_unallocated': 'Zugriff auf {addr}: dieser Heap-Speicher wurde nie angefordert.',
  'fault.heap_overflow': 'Zugriff auf {addr}: hinter dem Ende des Blocks (Rotzone).',
  'fault.use_after_free': 'Zugriff auf {addr}: dieser Block ist schon freigegeben.',
  'fault.invalid_free': 'FREE mit {addr}: das ist kein Blockanfang.',
  'fault.double_free': 'FREE mit {addr}: dieser Block ist schon freigegeben.',
  'fault.unknown_syscall': 'Unbekannter Systemaufruf {n}.',
  'fault.string_too_long': 'Die Zeichenkette ab {addr} endet nicht innerhalb von 4096 Bytes.',
  'fault.output_limit': 'Mehr als 8192 Ausgaben; das Programm hört nicht auf.',
  'run.budget_exceeded': 'Nach {cycles} Zyklen abgebrochen; am häufigsten lief Zeile {hottest_line}.',

  'check.out_mismatch': 'Ausgabe {index}: erwartet {expected}, war {actual}.',
  'check.out_missing': 'Ausgabe {index} fehlt: erwartet {expected}.',
  'check.out_extra': 'Eine Ausgabe zu viel an Stelle {index}: {actual}.',
  'check.text_mismatch': 'Text an Stelle {offset}: erwartet „{expected}“, war „{actual}“.',
  'check.mem_mismatch': 'Speicher bei {addr}: erwartet {expected}, war {actual}.',
  'check.reg_mismatch': '{reg}: erwartet {expected}, war {actual}.',
  'check.screen_mismatch': '{count} Pixel falsch, das erste bei ({x}|{y}): erwartet Farbe {expected}, war {actual}.',
  'check.heap_not_empty': '{count} Heap-Blöcke wurden nie freigegeben (angefordert in Zeile {alloc_line}).',
  'check.abi_clobbered': '{reg} hat sich verändert ({before} → {after}), muss aber erhalten bleiben.',
  'check.not_halted': 'Das Programm ist nicht mit halt beendet worden.',

  'check.not_halted_budget': 'Zyklenbudget aufgebraucht ({cycles} Zyklen): Zeile {line} lief {count}-mal. Endlosschleife?',

  'lvl.unknown_section': 'Unbekannter Abschnitt [{name}].',
  'lvl.unknown_key': 'Unbekannter Schlüssel „{key}“.',
  'lvl.missing_key': 'Schlüssel „{key}“ fehlt.',
  'lvl.bad_value': 'Ungültiger Wert für „{key}“.',
  'lvl.duplicate_section': 'Abschnitt doppelt.',
  'lvl.no_tests': 'Das Level hat keine Tests.',
  'lvl.allow_and_forbid': 'allow und forbid zugleich sind nicht erlaubt.',
  'lvl.bad_image': 'Bild {name}, Zeile {line} ist ungültig.',

  'abi.bad_handle': 'Interner Fehler: ungültiger Verweis.',
  'abi.bad_arg': 'Interner Fehler: ungültiges Argument.',
  'abi.not_assembled': 'Erst assemblieren.',
};

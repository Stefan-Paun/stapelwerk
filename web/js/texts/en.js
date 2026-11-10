// English interface texts. Same keys and placeholders as de.js.
export default {
  'meta.tagline': 'Assembly puzzles on a CPU of its own',

  'nav.main': 'Main navigation',
  'nav.skip': 'Skip to content',
  'nav.home': 'Chapter plan',
  'nav.plan': 'Chapter plan',
  'nav.sandbox': 'Sandbox',
  'nav.manual': 'Manual',
  'nav.about': 'About',
  'nav.crumbs': 'Path',

  'theme.toDark': 'Dark theme (Blueprint)',
  'theme.toLight': 'Light theme (Drawing board)',

  'boot.failed': 'The machine could not start.',
  'boot.failedText': 'Reload the page. If it happens again, your browser probably does not support WebAssembly.',
  'boot.internal': 'Internal error in the machine. It was restarted; your code is still there.',
  'boot.viewFailed': 'This view crashed.',
  'boot.progressReset': 'The saved progress was damaged and was not loaded.',

  'notfound.title': 'This page does not exist.',
  'notfound.text': 'The link points to no known view. The chapter plan lists every level.',

  'common.changed': 'changed',
  'common.cancel': 'Cancel',
  'common.close': 'Close',

  'chapter.0': 'Power On',
  'chapter.1': 'Registers & Arithmetic',
  'chapter.2': 'Jumps & Loops',
  'chapter.3': 'Bits & Numbers',
  'chapter.4': 'Memory',
  'chapter.5': 'Stack & Subroutines',
  'chapter.6': 'Recursion',
  'chapter.7': 'Heap & Lists',

  'plan.title': 'Chapter plan',
  'plan.lede': 'Thirty puzzles on a small 32-bit machine, from registers to the heap. Every level teaches what a real processor and a C compiler do.',
  'plan.continue': 'Continue with {num} {title}',
  'plan.solved': '{done} of {total} solved',
  'plan.unlockAll': 'Unlock everything',
  'plan.chapter': 'Chapter {n} · {title}',
  'plan.chapterShort': 'Chapter',
  'plan.progress': '{done}/{total}',
  'plan.state.solved': 'solved',
  'plan.state.open': 'open',
  'plan.state.locked': 'locked',

  'kind.tutorial': 'Introduction',
  'kind.debug': 'Debugging',
  'kind.final': 'Final',
  'kind.bonus': 'Bonus',
  'kind.screen': 'Screen',
  'kind.mobile': 'Phone',


  'ws.views': 'Views',
  'ws.tab.task': 'Level',
  'ws.tab.code': 'Code',
  'ws.tab.regs': 'Registers',
  'ws.tab.stack': 'Stack',
  'ws.tab.screen': 'Screen',
  'ws.tab.console': 'Console',
  'ws.tab.mem': 'Memory',

  'dbg.label': 'Debugger',
  'dbg.assembleTest': 'Assemble & test',
  'dbg.assemble': 'Assemble',
  'dbg.assembleShort': 'Build',
  'dbg.testShort': 'Test',
  'dbg.modEnter': 'Ctrl ↵',
  'dbg.restart': 'Restart',
  'dbg.back': 'Back',
  'dbg.step': 'Step',
  'dbg.run': 'Run',
  'dbg.pause': 'Pause',
  'dbg.timeline': 'Timeline',
  'dbg.stepOf': 'Step {step} of {total}',
  'dbg.cycles': '{cycles} cycles',
  'dbg.testcase': 'Test case',

  'status.not_assembled': 'Not assembled',
  'status.paused': 'Paused',
  'status.halted': 'Halted (halt)',
  'status.fault': 'Runtime fault',
  'status.breakpoint': 'At breakpoint',
  'status.waiting': 'Waiting for input',
  'status.budget': 'Cycle budget used up',
  'status.running': 'Running',
  'status.dirty': 'code changed',
  'status.tests': 'Tests {passed}/{total}',

  'task.tabs': 'Task',
  'task.task': 'Task',
  'task.hints': 'Hints',
  'task.tests': 'Tests',
  'task.result': 'Result',

  'tutorial.title': 'Checklist',
  'tutorial.step': 'Took a step',
  'tutorial.breakpoint': 'Set a breakpoint',
  'tutorial.run': 'Ran to the breakpoint',
  'tutorial.back': 'Went one step back',
  'tutorial.done': 'done',
  'tutorial.open': 'open',

  'hints.intro': 'Three hints, from the idea to almost the solution. Take only as much as you need.',
  'hints.n': 'Hint {n} of {total}',
  'hints.first': 'Show the first hint',
  'hints.next': 'Show hint {n}',

  'tests.notRun': 'Not tested yet. Assemble & test runs every test.',
  'tests.allPassed': 'All tests passed.',
  'tests.someFailed': '{passed} of {total} tests passed.',
  'tests.passed': 'passed',
  'tests.failed': 'failed',
  'tests.pending': 'not run yet',
  'tests.cycles': '{n} cycles',
  'tests.screen': 'image',
  'tests.open': 'Debug',
  'tests.openLabel': 'Open test {name} in the debugger',
  'tests.hidden': '{count} hidden tests',
  'tests.hiddenNote': 'Inputs stay hidden until one fails.',
  'tests.hiddenResult': '{passed} of {total} passed',

  'editor.title': 'Program',
  'editor.label': 'Program (assembly)',
  'editor.size': '{n} instruction words',
  'editor.linecol': 'L {line}:{col}',
  'editor.toggleBp': 'Set or remove a breakpoint',
  'editor.palette': 'Insert instructions',

  'notice.fault': 'Line {line}: {text}',
  'notice.budget': 'Stopped after {cycles} cycles. Line {line} ran most often; a loop probably never ends.',
  'notice.resetFailed': 'This test cannot be started. Check that your code contains the label the test calls.',
  'notice.solved': 'Solved. All tests passed.',
  'notice.testsFailed': '{failed} of {total} tests failed.',
  'notice.assembled': 'Assembled. Step or Run starts the program.',

  'regs.title': 'Registers',
  'regs.flags': 'Flags',
  'regs.pcNote': 'pc {pc} · line {line}',
  'regs.callerSaved': 'r0–r3 · caller-saved',
  'regs.calleeSaved': 'r4–r7 · callee-saved',
  'regs.pointers': 'Frame and stack pointer',
  'regs.value': '{reg} = {value} (hex {hex}), edit',
  'regs.valueChanged': '{reg} = {value} (hex {hex}), changed, was {before}, edit',
  'regs.editLabel': 'New value for {reg} (decimal, 0x… or 0b…)',
  'regs.editHint': 'Click to edit (paused only)',
  'regs.pausedOnly': 'Editing only works while the machine is paused',
  'regs.backToChange': 'Back to the last change of {reg}',
  'regs.flagLabel': 'Flag {flag} ({meaning}) = {value}',
  'regs.flag.Z': 'Zero: the result was 0',
  'regs.flag.N': 'Negative: bit 31 of the result',
  'regs.flag.C': 'Carry or borrow (unsigned)',
  'regs.flag.V': 'Overflow (signed)',

  'stack.title': 'Stack',
  'stack.caption': 'Stack from the highest address down to sp, with frames',
  'stack.pointer': 'Pointer',
  'stack.addr': 'Address',
  'stack.value': 'Value',
  'stack.meaning': 'Meaning',
  'stack.note': 'sp {sp} · {frames} frames',
  'stack.oldFp': 'old fp',
  'stack.chainEnd': 'end of the chain',
  'stack.ret': 'return',
  'stack.line': 'l. {line}',
  'stack.checker': 'checker',
  'stack.local': 'local',
  'stack.arg': 'argument',
  'stack.pushed': 'pushed',
  'stack.trap': 'return to the checker',
  'stack.frameCurrent': 'current frame',
  'stack.frameN': 'frame {n}',
  'stack.empty': 'The stack is empty: sp sits at the very top, F000.',
  'stack.notAssembled': 'Assemble first; then the stack shows frames and values.',
  'stack.truncated': 'More entries further up are hidden.',

  'screen.title': 'Screen',
  'screen.modeLabel': 'Display',
  'screen.mode.ist': 'Actual',
  'screen.mode.soll': 'Expected',
  'screen.mode.diff': 'Difference',
  'screen.aria': 'Screen 64 × 64, view {mode}, {count} pixels not black',
  'screen.ariaDiff': 'Difference to expected: {count} pixels wrong',
  'screen.hoverHint': 'Point at a pixel: coordinate, address, value and colour',

  'console.title': 'Console',
  'console.input': 'Input',
  'console.output': 'Output',
  'console.expected': 'Expected',
  'console.empty': 'no output yet',
  'console.noInput': 'no input',
  'console.inputInteractive': 'Type inputs below.',
  'console.note': '{read}/{total} read · {out} written',
  'console.waiting': 'The program is waiting for input.',
  'console.inputField': 'Input for the program',
  'console.inputKind': 'Kind of input',
  'console.kind.int': 'Numbers',
  'console.kind.text': 'Text',
  'console.send': 'Send',
  'console.close': 'Close input',
  'console.cut': '{count} older entries hidden',

  'mem.tabs': 'Memory and reference',
  'mem.hex': 'Hex dump',
  'mem.heap': 'Heap',
  'mem.manual': 'Manual',

  'hex.title': 'Hex dump',
  'hex.segments': 'Segment',
  'hex.unit': 'Unit',
  'hex.byte': 'Byte',
  'hex.word': 'Word',
  'hex.scrollLabel': 'Hex dump, scroll with the arrow keys',

  'seg.null': 'null',
  'seg.text': 'text',
  'seg.data': 'data',
  'seg.input': 'input',
  'seg.heap': 'heap',
  'seg.guard': 'guard',
  'seg.stack': 'stack',
  'seg.screen': 'screen',
  'seg.rights.text': 'read, execute',
  'seg.rights.data': 'read, write',
  'seg.rights.input': 'read, write · test input',
  'seg.rights.heap': 'only through ALLOC',
  'seg.rights.stack': 'read, write · grows downwards',
  'seg.rights.screen': 'read, write · 64 × 64 pixels',

  'heap.title': 'Heap map',
  'heap.aria': 'Heap map: {live} live of {total} blocks',
  'heap.state.free': 'free',
  'heap.state.live': 'allocated',
  'heap.state.freed': 'freed',
  'heap.state.red': 'red zone (8 B after every block)',
  'heap.blocks': 'Heap blocks',
  'heap.col.addr': 'Address',
  'heap.col.size': 'Size',
  'heap.col.status': 'Status',
  'heap.col.line': 'Line',
  'heap.live': 'live',
  'heap.freed': 'freed',
  'heap.empty': 'No ALLOC yet. Every block shows up here with the line that requested it.',

  'result.stamp': 'Passed',
  'result.title': 'Solved',
  'result.sub': 'All {tests} tests passed, the hidden ones too.',
  'result.tutorialMissing': 'The tests pass, but the checklist is not complete yet.',
  'result.metrics': 'Metrics compared with the reference solution',
  'result.col.metric': 'Metric',
  'result.col.yours': 'Yours',
  'result.col.ref': 'Reference',
  'result.col.delta': 'Difference',
  'result.cycles': 'Cycles (mean over all tests)',
  'result.size': 'Size (instruction words)',
  'result.within': 'reached',
  'result.over': '+{n} {unit}',
  'result.unit.cycles': 'cycles',
  'result.unit.words': 'words',
  'result.badge.compact': 'Compact',
  'result.badge.compact.rule': 'no more instruction words than the reference',
  'result.badge.fast': 'Swift',
  'result.badge.fast.rule': 'no more cycles than the reference',
  'result.earned': 'earned',
  'result.notEarned': 'not earned yet',
  'result.learned': 'What you just learned',
  'result.slots': 'Keep this solution (up to three slots per level):',
  'result.slotSave': 'Save to slot {n}',
  'result.slotReplace': 'Replace slot {n}',
  'result.slotSaved': 'Slot {n} saved',
  'result.next': 'Next: {num} {title}',
  'result.toPlan': 'To the chapter plan',

  'sandbox.title': 'Sandbox',
  'sandbox.intro': 'A free program without a task and without tests. Type inputs into the console while the program waits.',
  'sandbox.examples': 'Examples',
  'sandbox.ex.count': 'Counting',
  'sandbox.ex.count.note': 'a loop prints 1 to 10',
  'sandbox.ex.echo': 'Doubling',
  'sandbox.ex.echo.note': 'reads your numbers and waits for input',
  'sandbox.ex.hello': 'Text',
  'sandbox.ex.hello.note': 'a string in the data segment',
  'sandbox.ex.list': 'List',
  'sandbox.ex.list.note': 'three nodes on the heap',
  'sandbox.ex.flood': 'Flood fill',
  'sandbox.ex.flood.note': 'recursion until the stack overflows',
  'sandbox.share': 'Share',
  'sandbox.shareNote': 'The code travels compressed inside the link. Nothing is uploaded, and an opened link never starts by itself.',
  'sandbox.shareButton': 'Create link',
  'sandbox.shareLink': 'Share link',
  'sandbox.copied': 'Link copied.',
  'sandbox.copyManual': 'Copying did not work automatically; the link is selected.',
  'sandbox.badLink': 'The shared link is damaged; the code could not be read.',

  'manual.toc': 'Topics',
  'manual.pick': 'Choose a topic',
  'manual.topic': 'Topic',

  'manual.register.title': 'Registers',
  'manual.register.body': `A **register** is a storage cell right inside the processor, 32 bits wide. Stapelwerk has ten: \`r0\` to \`r7\`, \`fp\` and \`sp\`.

- \`r0\`–\`r3\`: arguments and return value. Whoever calls a function has to save them (*caller-saved*).
- \`r4\`–\`r7\`: the called function must hand them back unchanged (*callee-saved*).
- \`fp\`: frame pointer, points at the current [stack frame](manual:aufruf).
- \`sp\`: stack pointer, starts at \`0xF000\`.

The **PC** (program counter) points at the next instruction. You cannot write it directly, only change it with jumps. While paused you can change registers in the debugger: click the value.`,

  'manual.befehle.title': 'Instructions',
  'manual.befehle.body': `Every instruction is exactly 4 bytes long. Arithmetic takes three operands or a short form with two: \`add r1, r2\` means \`add r1, r1, r2\`.

- Load and set: \`mov\`, \`li\` (32-bit constant, two words), \`movhi\`, \`lea\`
- Arithmetic: \`add sub mul div divu rem remu neg\`
- Bits: \`and or xor not shl shr sar\`
- Compare: \`cmp\` (like \`sub\`, flags only), \`tst\` (like \`and\`, flags only)
- Memory: \`ld st\` (word), \`ldb ldbs stb\` (byte)
- Stack: \`push pop call ret\`
- Jumps: \`jmp jz jnz jlt jge jle jgt jb jae jbe ja js jns jo jno\`
- Other: \`sys\`, \`nop\`, \`halt\`

Cycles: most instructions 1, memory and stack 2, \`mul\` 3, division 8, \`sys\` 4.`,

  'manual.flags.title': 'Flags',
  'manual.flags.body': `Arithmetic sets four **flags**:

- **Z** (zero): the result was 0.
- **N** (negative): bit 31 of the result is set.
- **C** (carry): for \`add\` a carry out of bit 31, for \`sub\` and \`cmp\` a **borrow**, i.e. a < b without sign.
- **V** (overflow): the result does not fit into a signed 32-bit number.

\`cmp a, b\` computes a − b and keeps only the flags. A conditional jump reads them afterwards: \`jlt\` for signed less-than, \`jb\` for unsigned. \`mov\`, \`ld\` and jumps leave the flags alone.`,

  'manual.zweierkomplement.title': "Two's complement",
  'manual.zweierkomplement.body': `Negative numbers are bits like any others. In **two's complement**, −1 is \`0xFFFFFFFF\`: all 32 bits set. To negate a number, flip every bit and add 1.

The same bits mean 4 294 967 295 unsigned and −1 signed. The processor computes both the same way; only the flags and the jump instructions decide how you read the result.

- \`shr\` shifts in zeros (logical), \`sar\` copies the sign bit (arithmetic).
- Overflow wraps modulo 2³², it is never an error.`,

  'manual.speicher.title': 'Memory',
  'manual.speicher.body': `The machine has 64 KiB of memory in segments with their own rights:

- \`0000\`–\`00FF\` null: every access is a fault.
- \`0100\`–\`3FFF\` text: your code, read and execute only.
- \`4000\`–\`5FFF\` data: your data.
- \`6000\`–\`7FFF\` input: test input data (\`INPUT\`).
- \`8000\`–\`BFFF\` heap: only through \`sys ALLOC\`.
- \`C000\`–\`CFFF\` guard: protection zone.
- \`D000\`–\`EFFF\` stack.
- \`F000\`–\`FFFF\` screen: the [screen](manual:bildschirm).

Words are **little-endian**: the lowest byte at the lowest address. Word accesses need an address divisible by 4.`,

  'manual.stack.title': 'Stack',
  'manual.stack.body': `The **stack** sits at the top end of memory and grows downwards. \`push r1\` subtracts 4 from sp and stores r1 there; \`pop r1\` fetches the value and adds 4. Last in, first out.

\`call target\` puts the return address on the stack and jumps; \`ret\` takes it back. Every path through a function has to leave the stack exactly as it found it.

In the stack view high addresses are on top. The dimension chains on the left show how large the current frame is, and that the return address always sits 4 bytes above fp.`,

  'manual.aufruf.title': 'Calling convention',
  'manual.aufruf.body': `So that functions fit together, a **calling convention** applies:

- Arguments 1–4 in \`r0\`–\`r3\`, more on the stack. Return value in \`r0\`.
- A function may destroy \`r0\`–\`r3\`; it must return \`r4\`–\`r7\` and \`fp\` unchanged; sp must be right again after \`ret\`.

Prologue and epilogue build a **frame**:

\`\`\`
        push fp
        mov fp, sp
        sub sp, 8        ; room for local variables
        ...
        mov sp, fp
        pop fp
        ret
\`\`\`

Inside the frame, \`[fp+0]\` holds the old fp, \`[fp+4]\` the return address, and the local variables live below.`,

  'manual.heap.title': 'Heap',
  'manual.heap.body': `The **heap** is memory you request at run time: \`sys ALLOC\` with the size in r0 returns an address in r0 (or 0 if there is no room). \`sys FREE\` gives the block back.

- New memory is filled with \`CD\`, freed memory with \`DD\`; the hex dump shows it at once.
- Behind every block lie 8 bytes of **red zone**. Writing past the end triggers a fault.
- Use after \`FREE\` and freeing twice are detected.

The heap map draws every 4-byte granule with its own pattern: free, allocated, freed, red zone.`,

  'manual.bildschirm.title': 'Screen',
  'manual.bildschirm.body': `The screen has 64 × 64 pixels and is plain memory from \`SCREEN\` (\`0xF000\`) on: one byte per pixel, row after row. Pixel (x, y) lives at \`SCREEN + y·64 + x\`.

The lower 4 bits choose one of 16 colours. Their names are constants: \`BLACK BLUE GREEN CYAN RED MAGENTA BROWN LIGHTGRAY DARKGRAY LIGHTBLUE LIGHTGREEN LIGHTCYAN LIGHTRED PINK YELLOW WHITE\`.

In screen levels **Expected** shows the target image and **Difference** hatches every wrong pixel.`,

  'manual.systemaufrufe.title': 'System calls',
  'manual.systemaufrufe.body': `\`sys NAME\` calls the machine's operating system. System calls change only r0 and cost 4 cycles.

- \`READ_INT\`: next number into r0
- \`WRITE_INT\`: write r0 as a number
- \`READ_CHAR\`: next character into r0, −1 at the end
- \`WRITE_CHAR\`: write the lowest byte of r0
- \`WRITE_STR\`: string from address r0 up to the byte 0
- \`ALLOC\`, \`FREE\`: [heap](manual:heap)
- \`RAND\`: random number into r0

Important: since system calls write to r0, a value kept there is lost afterwards.`,

  'manual.debugger.title': 'Debugger',
  'manual.debugger.body': `- **Step** (S) executes exactly one instruction.
- **Back** (Z) goes one step back, even after a fault.
- **Run** (L) runs to the next **breakpoint**, to \`halt\` or to a fault.
- Set **breakpoints** by clicking the line gutter or with B.
- The **timeline** jumps to any step the machine has already taken.
- Next to every register a button goes back to its **last change**.

What a step changed is hatched and underlined in red; the old value stands struck through beside it. The numbers left of the code count how often each line ran.`,

  'help.title': 'Keyboard shortcuts',
  'help.key': 'Key',
  'help.action': 'Action',
  'help.step': 'Execute one step',
  'help.back': 'One step back',
  'help.runPause': 'Start or pause a run',
  'help.restart': 'Restart the test case',
  'help.breakpoint': 'Breakpoint at the current line (in the editor: Ctrl+B)',
  'help.tests': 'Show the tests',
  'help.help': 'This overview',
  'help.assembleTest': 'Assemble and test',
  'help.leaveEditor': 'Leave the editor',
  'help.tab': 'In the editor: insert a tab (press Esc first to move on)',
  'help.note': 'Letter shortcuts rest while you type in the editor. With Ctrl (⌘), Step, Breakpoint and Test work there.',

  'live.step': 'Line {line}: {instr}. {changes}',
  'live.noChange': 'No change',
  'live.bytes': '{count} bytes changed',
  'live.back': 'Back to step {step}, line {line}',
  'live.atStart': 'Already at the start.',
  'live.restart': 'Restarted.',
  'live.paused': 'Paused at step {step}.',
  'live.stopped': '{status}, line {line}.',
  'live.asmErrors': '{count} assembler errors.',
  'live.assembled': 'Assembled.',
  'live.solved': 'Solved: {cycles} cycles, {size} instruction words.',
  'live.testsFailed': '{failed} tests failed.',
  'live.noInstr': 'Line {line} holds no instruction.',
  'live.bpOn': 'Breakpoint at line {line}.',
  'live.bpOff': 'Breakpoint at line {line} removed.',
  'live.regSet': '{reg} set to {value}.',
  'live.backToChange': 'Back before the last change of {reg}: step {step}, line {line}.',

  'about.title': 'About Stapelwerk',
  'about.lede': 'An open learning game about computer architecture: no account, no data, works offline.',
  'about.what.title': 'What this is',
  'about.what.body': `Stapelwerk closes the gap between the model computers used at school (Johnny, Bonsai, Little Man Computer) and university tools (MARS, RARS, Ripes). The machine is small enough to understand in an afternoon, yet it has everything a first-semester exam asks for: two's complement, flags, stack frames, calling conventions, the heap and memory-mapped I/O.

The CPU is invented. It has no pipeline and no cache model. What a level says about real processors does hold for x86, ARM and RISC-V.`,
  'about.privacy.title': 'Privacy',
  'about.privacy.body': `Stapelwerk sets no cookies, counts no visits and loads nothing from other servers; even the fonts are served from here. Your progress stays in your browser's storage (localStorage) and only leaves your device if you export it yourself.`,
  'about.progress.title': 'Progress',
  'about.progress.body': 'Solved levels, best values, badges and your code live only in this browser. Save everything as a file, or take it to another device.',
  'about.export': 'Export',
  'about.import': 'Import',
  'about.delete': 'Delete progress',
  'about.exported': 'File saved.',
  'about.imported': 'Progress loaded.',
  'about.importFailed': 'This file is not Stapelwerk progress ({error}).',
  'about.importTooBig': 'The file is too large.',
  'about.deleteTitle': 'Delete progress?',
  'about.deleteText': 'All solved levels, badges and your code in this browser will be deleted. This cannot be undone; export first if you want to be safe.',
  'about.deleteConfirm': 'Delete for good',
  'about.deleted': 'Progress deleted.',
  'about.keys.title': 'Keyboard shortcuts',
  'about.licenses.title': 'Licences',
  'about.licenses.body': 'The code is MIT licensed. The fonts Sofia Sans and IBM Plex Mono are licensed under the SIL Open Font License 1.1:',
  'about.fontSofia': 'Sofia Sans (OFL 1.1)',
  'about.fontPlex': 'IBM Plex Mono (OFL 1.1)',
  'about.ai.title': 'How this was built',
  'about.ai.body': 'Code, levels and texts were written by Claude Code agents; the idea, the choice of content and the chapter plan come from Stefan Paun. Every level is checked with a reference solution, a mutation test and an independent second solution. Mistakes are still possible, issues are welcome.',
  'about.engine.title': 'Machine',
  'about.engineMock': 'Right now the JavaScript demo stand-in is running. It reproduces part of the instruction set and exists to develop the interface.',
  'about.engineReal': 'The machine runs as a WebAssembly core, written in Rust without third-party libraries.',

  'access.r': 'Read',
  'access.w': 'Write',

  'asm.summary': '{count} errors while assembling. The list is below the code.',
  'asm.suggestion': 'Did you mean “{suggestion}”?',
  'asm.hint_li': 'For large numbers there is “li”.',
  'asm.unknown_mnemonic': 'Unknown instruction “{name}”.',
  'asm.unknown_register': 'Unknown register “{name}”. There are r0–r7, fp and sp.',
  'asm.operand_count': 'Expected {expected} operands, found {got}.',
  'asm.operand_type': 'A different operand belongs here ({expected}).',
  'asm.imm_range': 'The value {value} is outside {min} to {max}.',
  'asm.undefined_symbol': '“{name}” is not defined anywhere.',
  'asm.duplicate_label': 'The label “{name}” already exists in line {first_line}.',
  'asm.reserved_name': '“{name}” is a reserved name.',
  'asm.bad_number': 'I cannot read this number.',
  'asm.bad_string': 'This string is not closed properly or has an unknown escape.',
  'asm.unexpected_char': 'Unexpected character “{ch}”.',
  'asm.local_label_without_global': 'A local label (.name) needs a normal label before it.',
  'asm.needs_constant': 'A constant defined earlier is required here.',
  'asm.expr_div_zero': 'Division by 0 in the expression.',
  'asm.expr_overflow': 'The expression overflows.',
  'asm.data_in_text': 'Data belongs in the .data section.',
  'asm.code_in_data': 'Instructions belong in the .text section.',
  'asm.section_overflow': 'Section {section} is too large at {size} bytes (at most {max}).',
  'asm.forbidden_instruction': '“{mnemonic}” is not allowed in this level.',
  'asm.too_many_errors': 'Too many errors; stopping here.',

  'fault.illegal_instruction': 'Invalid instruction {word}.',
  'fault.zero_instruction': 'The processor ran into an empty word. Forgot halt?',
  'fault.exec_outside_code': 'Execution outside the code.',
  'fault.bad_jump_target': 'Jump target {target} is not an instruction in the code.',
  'fault.bad_return_address': 'ret jumps to {target}, there is no code there. Is the return address really on top of the stack?',
  'fault.null_access': '{access} at address {addr}: the area from 0 is locked (null pointer?).',
  'fault.out_of_range': '{access} at address {addr}: outside memory.',
  'fault.guard_page': '{access} at address {addr}: guard zone between heap and stack.',
  'fault.write_to_code': 'Write to {addr}: code must not be changed.',
  'fault.misaligned': 'Address {addr} is not aligned for a {size}-byte access (·4?).',
  'fault.stack_overflow': 'Stack overflow: sp would be {sp}.',
  'fault.stack_underflow': 'Stack underflow: sp would be {sp}. More pop than push?',
  'fault.stack_misaligned': 'sp {sp} is not divisible by 4.',
  'fault.div_zero': 'Division by 0.',
  'fault.input_end': 'The input has ended, but the program wants to read more.',
  'fault.heap_unallocated': 'Access to {addr}: this heap memory was never requested.',
  'fault.heap_overflow': 'Access to {addr}: past the end of the block (red zone).',
  'fault.use_after_free': 'Access to {addr}: this block has already been freed.',
  'fault.invalid_free': 'FREE with {addr}: that is not the start of a block.',
  'fault.double_free': 'FREE with {addr}: this block has already been freed.',
  'fault.unknown_syscall': 'Unknown system call {n}.',
  'fault.string_too_long': 'The string at {addr} does not end within 4096 bytes.',
  'fault.output_limit': 'More than 8192 outputs; the program does not stop.',
  'run.budget_exceeded': 'Stopped after {cycles} cycles; line {hottest_line} ran most often.',

  'check.out_mismatch': 'Output {index}: expected {expected}, got {actual}.',
  'check.out_missing': 'Output {index} is missing: expected {expected}.',
  'check.out_extra': 'One output too many at position {index}: {actual}.',
  'check.text_mismatch': 'Text at position {offset}: expected “{expected}”, got “{actual}”.',
  'check.mem_mismatch': 'Memory at {addr}: expected {expected}, got {actual}.',
  'check.reg_mismatch': '{reg}: expected {expected}, got {actual}.',
  'check.screen_mismatch': '{count} pixels wrong, the first at ({x}|{y}): expected colour {expected}, got {actual}.',
  'check.heap_not_empty': '{count} heap blocks were never freed (requested in line {alloc_line}).',
  'check.abi_clobbered': '{reg} changed ({before} → {after}) but has to be preserved.',
  'check.not_halted': 'The program did not end with halt.',

  'check.not_halted_budget': 'Cycle budget used up ({cycles} cycles): line {line} ran {count} times. An endless loop?',

  'lvl.unknown_section': 'Unknown section [{name}].',
  'lvl.unknown_key': 'Unknown key “{key}”.',
  'lvl.missing_key': 'Key “{key}” is missing.',
  'lvl.bad_value': 'Invalid value for “{key}”.',
  'lvl.duplicate_section': 'Section appears twice.',
  'lvl.no_tests': 'The level has no tests.',
  'lvl.allow_and_forbid': 'allow and forbid together are not allowed.',
  'lvl.bad_image': 'Image {name}, line {line} is invalid.',

  'abi.bad_handle': 'Internal error: invalid handle.',
  'abi.bad_arg': 'Internal error: invalid argument.',
  'abi.not_assembled': 'Assemble first.',
};

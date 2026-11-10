// The level workspace (and, with sandbox: true, the sandbox): debugger bar,
// task, editor, registers, stack, screen, console and memory tabs. Owns the
// Session and turns user actions into VM calls; the panes only render views.
import { el, mount, setText } from '../dom.js';
import { icon } from '../icons.js';
import { STATUS, STATUS_NAME, REG_NAMES, hex, signed } from '../machine.js';
import { diffBytes } from '../hexdump.js';
import { renderMarkup } from '../markup.js';
import { createEditor } from './editor.js';
import { createRegisters } from './registers.js';
import { createStack } from './stack.js';
import { createScreen } from './screen.js';
import { createConsole } from './console.js';
import { createTests } from './tests.js';
import { createHexdump } from './hexdump.js';
import { createHeapmap } from './heapmap.js';
import { createManualTab } from './manual.js';
import { renderResult } from './result.js';
import { createTabs } from './tabs.js';
import { createSandboxPanel, SANDBOX_EXAMPLES } from './sandbox.js';

const RUN_SLICE = 50000;
const MOBILE_TABS = ['task', 'code', 'regs', 'stack', 'screen', 'console', 'mem'];
const TUTORIAL_ITEMS = ['step', 'breakpoint', 'run', 'back'];

export function faultText(t, fault) {
  const args = { ...fault.args };
  for (const k of ['addr', 'target', 'sp']) if (k in args) args[k] = `0x${hex(args[k], 4)}`;
  if ('word' in args) args.word = `0x${hex(args.word)}`;
  if ('access' in args) args.access = t(`access.${args.access}`);
  return t(fault.key, args);
}

function diagText(t, d) {
  const args = { ...d.args };
  let text = t(d.key, args);
  if (d.args?.suggestion) text += ` ${t('asm.suggestion', { suggestion: d.args.suggestion })}`;
  if (d.args?.hint === 'li') text += ` ${t('asm.hint_li')}`;
  return text;
}

export async function renderWorkspace(app, { levelId = null, sandbox = false, initialCode = null } = {}) {
  const { t, lang, progress } = app;
  const level = sandbox ? null : await app.levels.load(levelId);
  if (!sandbox && !level) return null;
  const { levels } = await app.levels.index();
  const storeId = sandbox ? 'sandbox' : level.id;
  const session = app.vm.newSession(level ? level.handle : null);
  const record = () => progress.level(storeId);
  const visibleTests = level ? level.tests.map((x, i) => ({ ...x, index: i })).filter((x) => !x.hidden) : [];

  const ui = {
    assembledSource: null,
    diagnostics: [],
    result: null,
    testIndex: sandbox ? -1 : (visibleTests[0]?.index ?? -1),
    prev: null,
    changedRegs: new Set(),
    changedBytes: new Set(),
    running: false,
    bpLines: new Set(),
    lineToAddr: new Map(),
    addrToLine: new Map(),
    labels: new Map(),
    trapAddr: null,
    hints: 0,
    notice: null,
    raf: 0,
    sliderRaf: 0,
    typedInts: [],
    typedText: '',
    expected: null,
    expectedFor: null,
    hotLine: null,
  };

  // ------------------------------------------------------------ layout
  const statusChip = el('p', { class: 'status-chip', role: 'status' });
  const titleText = sandbox ? t('sandbox.title') : level.title[lang] ?? level.title.de;
  const head = el('div', { class: 'ws-head' },
    el('nav', { class: 'crumbs', 'aria-label': t('nav.crumbs') },
      el('a', { href: '#/' }, t('nav.plan')),
      el('span', { 'aria-hidden': 'true' }, '/'),
      el('span', {}, sandbox ? t('nav.sandbox') : t('plan.chapter', { n: level.chapter, title: t(`chapter.${level.chapter}`) }))),
    el('h1', { class: 'ws-title', id: 'ws-title', tabindex: '-1' },
      sandbox ? null : el('span', { class: 'ws-num mono' }, `${level.chapter}.${level.order}`),
      titleText),
    statusChip);

  const btn = (action, iconName, label, key, cls = '') => el('button', {
    type: 'button', class: ['btn', 'dbg-btn', cls], dataset: { action },
    'aria-keyshortcuts': key ?? null,
    title: key ? `${label} (${key})` : null,
    onclick: () => act(action),
  }, icon(iconName), el('span', { class: 'btn-label' }, label), key ? el('kbd', { 'aria-hidden': 'true' }, key) : null);
  const runBtn = btn('runPause', 'run', t('dbg.run'), 'L');
  const stepBtn = btn('step', 'step', t('dbg.step'), 'S');
  const backBtn = btn('back', 'back', t('dbg.back'), 'Z');
  const restartBtn = btn('restart', 'restart', t('dbg.restart'), 'R');
  const primary = el('button', {
    type: 'button', class: 'btn btn-primary dbg-primary', dataset: { action: 'assembleTest' },
    'aria-keyshortcuts': 'Control+Enter Meta+Enter',
    onclick: () => act('assembleTest'),
  }, icon(sandbox ? 'assemble' : 'test'), el('span', { class: 'btn-label' }, el('span', { class: 'lbl-long' }, sandbox ? t('dbg.assemble') : t('dbg.assembleTest')), el('span', { class: 'lbl-short', 'aria-hidden': 'true' }, sandbox ? t('dbg.assembleShort') : t('dbg.testShort'))), el('kbd', { 'aria-hidden': 'true' }, t('dbg.modEnter')));
  const slider = el('input', { type: 'range', min: 0, max: 0, value: 0, step: 1, class: 'timeline-range', id: 'timeline', 'aria-describedby': 'timeline-out' });
  const sliderOut = el('output', { class: 'timeline-out mono', id: 'timeline-out', for: 'timeline' });
  const timeline = el('div', { class: 'timeline' },
    el('label', { for: 'timeline', class: 'timeline-label' }, t('dbg.timeline')), slider, sliderOut);
  const testSelect = el('select', { class: 'test-select', 'aria-label': t('dbg.testcase'), onchange: () => { ui.testIndex = Number(testSelect.value); restart(); } },
    visibleTests.map((x) => el('option', { value: x.index }, x.name)));
  const toolbar = el('div', { class: 'dbg', role: 'toolbar', 'aria-label': t('dbg.label') },
    primary,
    el('div', { class: 'dbg-group' }, restartBtn, backBtn, stepBtn, runBtn),
    timeline,
    sandbox ? null : el('label', { class: 'test-pick' }, el('span', {}, t('dbg.testcase')), testSelect));

  // Task pane
  const taskBody = el('div', { class: 'task prose' });
  const hintsBody = el('div', { class: 'hints prose' });
  const tests = createTests({ t, onOpen: (i) => openTest(i), faultText: (f) => faultText(t, f) });
  const resultBody = el('div', { class: 'result-wrap' });
  let taskTabs = null;
  let taskPane;
  if (sandbox) {
    const panel = createSandboxPanel({ t, getCode: () => editor.value, onLoad: (code) => { editor.value = code; onCode(code); invalidate(); } });
    taskPane = el('section', { class: 'pane pane-task', 'aria-label': t('sandbox.title') }, panel.node);
  } else {
    taskTabs = createTabs({
      label: t('task.tabs'),
      items: [
        { id: 'task', label: t('task.task'), panel: taskBody },
        { id: 'hints', label: t('task.hints'), panel: hintsBody },
        { id: 'tests', label: t('task.tests'), panel: tests.node },
        { id: 'result', label: t('task.result'), panel: resultBody, hidden: true },
      ],
    });
    taskPane = el('section', { class: 'pane pane-task', 'aria-label': t('task.tabs') }, taskTabs.node);
  }

  // Code pane
  const sizeNote = el('span', { class: 'pane-note mono' });
  const notice = el('div', { class: 'notice', role: 'status', hidden: true });
  const saved = record().code;
  const code = initialCode ?? saved ?? (sandbox ? SANDBOX_EXAMPLES[0].code : level.start);
  let saveTimer = 0;
  const onCode = (value) => {
    clearTimeout(saveTimer);
    saveTimer = setTimeout(() => progress.saveCode(storeId, value), 400);
    renderStatus();
  };
  const editor = createEditor({
    t,
    value: code,
    label: t('editor.label'),
    onInput: onCode,
    onToggleBreakpoint: (line) => toggleBreakpoint(line),
    onEscape: () => stepBtn.focus(),
  });
  const codePane = el('section', { class: 'pane pane-code', 'aria-labelledby': 'h-code' },
    el('header', { class: 'pane-head' }, el('h2', { id: 'h-code' }, t('editor.title')), sizeNote),
    notice, editor.node);

  // State panes
  const registers = createRegisters({ t, onSetReg: (r, v) => setReg(r, v), onBackToChange: (k, target) => backToChange(k, target) });
  const stack = createStack({ t });
  const screen = createScreen({ t });
  const consoleView = createConsole({
    t,
    interactive: sandbox,
    onInt: (v) => { session.pushInt(v); ui.typedInts.push(v); afterInput(); },
    onText: (s) => { session.pushText(s); ui.typedText += s; afterInput(); },
    onClose: () => { session.closeInput(); afterInput(); },
  });
  const hexdump = createHexdump({ t });
  const heapmap = createHeapmap({ t });
  const manual = createManualTab({ t });
  const memTabs = createTabs({
    label: t('mem.tabs'),
    items: [
      { id: 'hex', label: t('mem.hex'), panel: hexdump.node },
      { id: 'heap', label: t('mem.heap'), panel: heapmap.node },
      { id: 'manual', label: t('mem.manual'), panel: manual.node },
    ],
    onChange: () => refresh(),
  });
  const memPane = el('section', { class: 'pane pane-mem', 'aria-label': t('mem.tabs') }, memTabs.node);

  const mobileTabs = el('div', { class: 'ws-mobile-tabs', role: 'tablist', 'aria-label': t('ws.views') },
    MOBILE_TABS.map((id) => el('button', {
      type: 'button', role: 'tab', class: 'mtab', dataset: { tab: id },
      'aria-selected': id === 'code' ? 'true' : 'false',
      onclick: () => setMobileTab(id),
    }, id === 'task' && sandbox ? t('sandbox.examples') : t(`ws.tab.${id}`))));

  const grid = el('div', { class: 'ws-grid' },
    el('div', { class: 'ws-col ws-col-a' }, taskPane, codePane),
    el('div', { class: 'ws-col ws-col-b' }, registers.node, stack.node),
    el('div', { class: 'ws-col ws-col-c' }, screen.node, consoleView.node, memPane));
  const root = el('div', { class: 'ws', dataset: { mode: sandbox ? 'sandbox' : 'level', mtab: 'code' } },
    head, toolbar, mobileTabs, grid);

  function setMobileTab(id) {
    root.dataset.mtab = id;
    for (const b of mobileTabs.children) b.setAttribute('aria-selected', b.dataset.tab === id ? 'true' : 'false');
    refresh();
  }

  // ------------------------------------------------------------ task content
  function renderTask() {
    if (sandbox) return;
    const items = [renderMarkup(level.text(lang), { headingLevel: 3 })];
    if (level.kind === 'tutorial') {
      const rec = record();
      items.push(el('div', { class: 'checklist' },
        el('h3', {}, t('tutorial.title')),
        el('ul', {}, TUTORIAL_ITEMS.map((item) => el('li', { class: rec.tutorial[item] ? 'is-done' : '' },
          el('span', { class: 'check-box', 'aria-hidden': 'true' }, rec.tutorial[item] ? icon('check') : null),
          t(`tutorial.${item}`),
          el('span', { class: 'sr-only' }, rec.tutorial[item] ? ` (${t('tutorial.done')})` : ` (${t('tutorial.open')})`))))));
    }
    mount(taskBody, items);
  }

  function renderHints() {
    if (sandbox) return;
    const hints = level.hints(lang);
    const shown = hints.slice(0, ui.hints).map((h, i) => el('section', { class: 'hint' },
      el('h3', {}, t('hints.n', { n: i + 1, total: hints.length })), renderMarkup(h, { headingLevel: 4 })));
    const more = ui.hints < hints.length
      ? el('button', { type: 'button', class: 'btn', onclick: () => { ui.hints += 1; renderHints(); hintsBody.querySelector('.hint:last-of-type h3')?.focus(); } },
        t(ui.hints === 0 ? 'hints.first' : 'hints.next', { n: ui.hints + 1 }))
      : null;
    mount(hintsBody, el('p', { class: 'muted' }, t('hints.intro')), shown, more);
    for (const h of hintsBody.querySelectorAll('.hint h3')) h.tabIndex = -1;
  }

  // ------------------------------------------------------------ machine glue
  function rebuildMaps() {
    ui.lineToAddr.clear();
    ui.addrToLine.clear();
    for (const e of session.lineMap()) {
      if (e.f !== 0) continue;
      if (!ui.lineToAddr.has(e.l)) ui.lineToAddr.set(e.l, e.a);
      ui.addrToLine.set(e.a, e.l);
    }
    ui.labels.clear();
    ui.trapAddr = null;
    for (const s of session.symbols()) {
      if (s.name === '__trap') ui.trapAddr = s.value;
      else if (!ui.labels.has(s.value)) ui.labels.set(s.value, s.name);
    }
  }

  function applyBreakpoints() {
    session.clearBreakpoints();
    for (const line of ui.bpLines) {
      const a = ui.lineToAddr.get(line);
      if (a !== undefined) session.setBreakpoint(a, true);
    }
  }

  function assemble() {
    const res = session.assemble(editor.value);
    ui.assembledSource = editor.value;
    ui.diagnostics = res.diagnostics ?? [];
    ui.hotLine = null;
    ui.prev = null;
    ui.changedRegs = new Set();
    ui.changedBytes = new Set();
    if (res.ok) {
      rebuildMaps();
      applyBreakpoints();
      setNotice(null);
      resetMachine();
    } else {
      ui.result = null;
      setNotice('error', t('asm.summary', { count: ui.diagnostics.length }));
    }
    editor.setErrors(ui.diagnostics.filter((d) => d.file === 0).map((d) => ({ line: d.line, col: d.col, text: diagText(t, d) })));
    setText(sizeNote, res.ok ? t('editor.size', { n: res.size }) : '');
    return res.ok;
  }

  function ensureAssembled() {
    if (ui.assembledSource === editor.value && session.state().status !== STATUS.NOT_ASSEMBLED) return true;
    const ok = assemble();
    refresh();
    if (!ok) app.announce(t('live.asmErrors', { count: ui.diagnostics.length }));
    return ok;
  }

  function capture() {
    const st = session.state();
    ui.prev = { regs: [...st.regs], flags: st.flags, mem: session.memory().slice(), pc: st.pc, steps: st.steps };
  }

  function diff() {
    if (!ui.prev) { ui.changedRegs = new Set(); ui.changedBytes = new Set(); return; }
    const st = session.state();
    ui.changedRegs = new Set(st.regs.map((v, i) => (v !== ui.prev.regs[i] ? i : -1)).filter((i) => i >= 0));
    ui.changedBytes = diffBytes(ui.prev.mem, session.memory());
    ui.quietBytes = Boolean(ui.runDiff);
    ui.runDiff = false;
  }

  function invalidate() {
    stop();
    ui.prev = null;
    ui.changedRegs = new Set();
    ui.changedBytes = new Set();
    refresh();
  }

  function setNotice(kind, text) {
    ui.notice = kind ? { kind, text } : null;
    notice.hidden = !kind;
    notice.className = kind ? `notice notice-${kind}` : 'notice';
    if (kind) mount(notice, icon(kind === 'ok' ? 'check' : 'cross'), el('span', {}, text));
  }

  function noticeFromState() {
    const st = session.state();
    if (st.status === STATUS.FAULT) {
      const f = session.fault();
      if (f) setNotice('error', t('notice.fault', { line: ui.addrToLine.get(f.pc) ?? '?', text: faultText(t, f) }));
    } else if (st.status === STATUS.BUDGET_EXCEEDED) {
      const counts = session.lineCounts();
      const hottest = counts.reduce((best, c) => (c[1] > (best?.[1] ?? 0) ? c : best), null);
      ui.hotLine = hottest?.[0] ?? null;
      setNotice('warn', t('notice.budget', { cycles: app.i18n.num(st.cycles), line: hottest?.[0] ?? '?' }));
    } else if (st.status === STATUS.HALTED && ui.notice?.kind !== 'ok' && ui.notice?.kind !== 'error') setNotice(null);
    else if (ui.notice?.kind === 'warn' || (ui.notice?.kind === 'error' && ui.diagnostics.length === 0)) setNotice(null);
  }

  function describeStep(prevPc) {
    const st = session.state();
    const instr = session.disasm(prevPc, 1)[0]?.text ?? '';
    const changes = [];
    for (const i of ui.changedRegs) changes.push(`${REG_NAMES[i]}: ${signed(ui.prev.regs[i])} → ${signed(st.regs[i])}`);
    if (ui.changedBytes.size) changes.push(t('live.bytes', { count: ui.changedBytes.size }));
    return t('live.step', { line: ui.addrToLine.get(prevPc) ?? '?', instr, changes: changes.join(', ') || t('live.noChange') });
  }

  // ------------------------------------------------------------ actions
  // Puts the machine at the start of the selected test. A test whose start already fails
  // (e.g. a function test whose `call` label is missing) comes back as FAULT and is shown
  // like any other fault; an outright error status gets its own message instead of
  // silently leaving the machine in some other state.
  function resetMachine({ quiet = false } = {}) {
    const status = session.reset(ui.testIndex) >>> 0;
    if (status === STATUS.ERROR) {
      if (!quiet) setNotice('error', t('notice.resetFailed'));
    } else if (!quiet) noticeFromState();
    return status;
  }

  function step() {
    if (!ensureAssembled()) return;
    stop();
    const st = session.state();
    if (st.status !== STATUS.PAUSED && st.status !== STATUS.BREAKPOINT) { app.announce(statusText()); return; }
    capture();
    session.step(1);
    diff();
    tutorial('step');
    noticeFromState();
    refresh();
    app.announce(describeStep(ui.prev.pc));
  }

  function back() {
    if (!ensureAssembled()) return;
    stop();
    if (session.state().steps === 0) { app.announce(t('live.atStart')); return; }
    capture();
    session.back(1);
    diff();
    tutorial('back');
    noticeFromState();
    refresh();
    app.announce(t('live.back', { step: session.state().steps, line: ui.addrToLine.get(session.state().pc) ?? '?' }));
  }

  function restart() {
    stop();
    if (!ensureAssembled()) return;
    setNotice(null);
    resetMachine();
    ui.prev = null;
    diff();
    refresh();
    app.announce(t('live.restart'));
  }

  function gotoStep(n) {
    if (!ensureAssembled()) return;
    stop();
    capture();
    session.goto(n);
    diff();
    noticeFromState();
    refresh();
  }

  function stop() {
    if (ui.raf) cancelAnimationFrame(ui.raf);
    ui.raf = 0;
    if (ui.running) { ui.running = false; renderToolbar(); }
  }

  function runPause() {
    if (ui.running) {
      stop();
      app.announce(t('live.paused', { step: session.state().steps }));
      refresh();
      return;
    }
    if (!ensureAssembled()) return;
    const st = session.state();
    if (st.status !== STATUS.PAUSED && st.status !== STATUS.BREAKPOINT) { app.announce(statusText()); return; }
    capture();
    ui.running = true;
    renderToolbar();
    const frame = () => {
      const status = session.run(RUN_SLICE);
      if (status === STATUS.PAUSED && ui.running) {
        ui.raf = requestAnimationFrame(frame);
        refresh(true);
        return;
      }
      ui.running = false;
      ui.raf = 0;
      ui.runDiff = true;
      diff();
      if (status === STATUS.BREAKPOINT) tutorial('run');
      noticeFromState();
      refresh();
      app.announce(t('live.stopped', { status: statusText(), line: ui.addrToLine.get(session.state().pc) ?? '?' }));
    };
    ui.raf = requestAnimationFrame(frame);
  }

  function afterInput() {
    refresh();
    if (session.state().status === STATUS.PAUSED && !ui.running && ui.prev) runPause();
  }

  function toggleBreakpoint(line) {
    if (!ensureAssembled()) return;
    if (!ui.lineToAddr.has(line)) { app.announce(t('live.noInstr', { line })); return; }
    if (ui.bpLines.has(line)) ui.bpLines.delete(line); else { ui.bpLines.add(line); tutorial('breakpoint'); }
    applyBreakpoints();
    refresh();
    app.announce(t(ui.bpLines.has(line) ? 'live.bpOn' : 'live.bpOff', { line }));
  }

  function setReg(reg, value) {
    if (!ensureAssembled()) return;
    capture();
    session.setReg(reg, value);
    diff();
    refresh();
    app.announce(t('live.regSet', { reg: reg === 10 ? t('regs.flags') : REG_NAMES[reg], value: reg === 10 ? value : signed(value) }));
  }

  function backToChange(kind, target) {
    if (!ensureAssembled()) return;
    stop();
    capture();
    session.backToChange(kind, target);
    diff();
    noticeFromState();
    refresh();
    app.announce(t('live.backToChange', { reg: REG_NAMES[target] ?? '', step: session.state().steps, line: ui.addrToLine.get(session.state().pc) ?? '?' }));
  }

  function openTest(i) {
    ui.testIndex = i;
    if (!level.tests[i].hidden) testSelect.value = String(i);
    restart();
    setMobileTab('code');
    stepBtn.focus();
  }

  function tutorial(item) {
    if (sandbox || level.kind !== 'tutorial') return;
    if (record().tutorial[item]) return;
    progress.markTutorial(level.id, item);
    renderTask();
  }

  function assembleAndTest() {
    stop();
    const ok = assemble();
    if (!ok) {
      refresh();
      app.announce(t('live.asmErrors', { count: ui.diagnostics.length }));
      editor.goToLine(ui.diagnostics.find((d) => d.file === 0)?.line ?? 1, ui.diagnostics[0]?.col ?? 1);
      return;
    }
    if (sandbox) {
      setNotice('ok', t('notice.assembled'));
      refresh();
      app.announce(t('live.assembled'));
      return;
    }
    const result = session.runTests();
    ui.result = result;
    // An endless loop shows up as an exhausted budget: mark the hottest line.
    const spin = result.tests.find((x) => x.mismatch?.key === 'check.not_halted' && x.mismatch.args?.hottest_line);
    ui.hotLine = spin ? spin.mismatch.args.hottest_line : null;
    const resetStatus = resetMachine({ quiet: true });
    const tutorialMissing = level.kind === 'tutorial' && !progress.tutorialDone(level.id);
    if (result.passed && !tutorialMissing) {
      progress.markSolved(level.id, result.metrics, result.reference);
      setNotice('ok', t('notice.solved'));
      showResult(result, false);
      taskTabs.select('result');
      resultBody.querySelector('h2')?.focus();
      app.announce(t('live.solved', { cycles: result.metrics.cycles_avg, size: result.metrics.size }));
    } else {
      if (result.passed) setNotice('warn', t('result.tutorialMissing'));
      else setNotice('error', t('notice.testsFailed', { failed: result.tests.filter((x) => !x.passed).length, total: result.tests.length }));
      taskTabs.select('tests');
      app.announce(result.passed ? t('result.tutorialMissing') : t('live.testsFailed', { failed: result.tests.filter((x) => !x.passed).length }));
    }
    if (resetStatus === STATUS.ERROR) setNotice('error', t('notice.resetFailed'));
    if (root.dataset.mtab === 'code' || root.dataset.mtab === 'task') setMobileTab('task');
    refresh();
  }

  function showResult(result, tutorialMissing) {
    const idx = levels.findIndex((l) => l.id === level.id);
    const next = levels.slice(idx + 1).find((l) => progress.isUnlocked(levels, l.id)) ?? null;
    taskTabs.setHidden('result', false);
    mount(resultBody, renderResult({
      t, lang, level, result, record: record(), next, tutorialMissing,
      onSaveSlot: (i) => progress.saveSlot(level.id, i, editor.value),
    }));
  }

  function act(action) {
    try {
      return dispatch(action);
    } catch (err) {
      // vm.js throws after a trap (and has already reloaded the core) or on misuse.
      console.error(err);
      stop();
      setNotice('error', t('boot.internal'));
      refresh();
      app.announce(t('boot.internal'));
      return true;
    }
  }

  function dispatch(action) {
    switch (action) {
      case 'assembleTest': assembleAndTest(); break;
      case 'step': step(); break;
      case 'back': back(); break;
      case 'runPause': runPause(); break;
      case 'restart': restart(); break;
      case 'breakpoint': {
        const inEditor = document.activeElement === editor.textarea;
        const pcLine = ui.addrToLine.get(session.state().pc);
        toggleBreakpoint(inEditor ? editor.caretLine() : pcLine ?? editor.caretLine());
        break;
      }
      case 'tests': if (taskTabs) { taskTabs.select('tests'); setMobileTab('task'); } break;
      case 'leaveEditor': stepBtn.focus(); break;
      default: return false;
    }
    return true;
  }

  // ------------------------------------------------------------ rendering
  function statusText() {
    const st = session.state();
    if (st.status === STATUS.NOT_ASSEMBLED || ui.assembledSource === null) return t('status.not_assembled');
    if (ui.running) return t('status.running');
    return t(`status.${STATUS_NAME[st.status] ?? 'paused'}`);
  }

  function renderToolbar() {
    const st = session.state();
    const assembled = st.status !== STATUS.NOT_ASSEMBLED;
    const canGo = assembled && (st.status === STATUS.PAUSED || st.status === STATUS.BREAKPOINT);
    mount(runBtn, icon(ui.running ? 'pause' : 'run'), el('span', { class: 'btn-label' }, ui.running ? t('dbg.pause') : t('dbg.run')), el('kbd', { 'aria-hidden': 'true' }, 'L'));
    runBtn.setAttribute('aria-pressed', ui.running ? 'true' : 'false');
    stepBtn.setAttribute('aria-disabled', canGo || !assembled ? 'false' : 'true');
    backBtn.setAttribute('aria-disabled', assembled && st.steps > 0 ? 'false' : 'true');
    const horizon = Math.max(session.horizon?.() ?? st.steps, st.steps);
    slider.max = String(horizon);
    slider.value = String(st.steps);
    slider.disabled = !assembled || horizon === 0;
    slider.setAttribute('aria-valuetext', t('dbg.stepOf', { step: st.steps, total: horizon }));
    mount(sliderOut, el('span', { class: 'tl-step' }, t('dbg.stepOf', { step: app.i18n.num(st.steps), total: app.i18n.num(horizon) })), el('span', { class: 'tl-cyc' }, ` · ${t('dbg.cycles', { cycles: app.i18n.num(st.cycles) })}`));
  }

  function renderStatus() {
    const st = session.state();
    const dirty = ui.assembledSource !== null && ui.assembledSource !== editor.value;
    const kind = ui.running ? 'running' : ui.assembledSource === null || st.status === STATUS.NOT_ASSEMBLED ? 'idle' : STATUS_NAME[st.status];
    statusChip.dataset.kind = kind;
    const parts = [el('span', { class: 'status-dot', 'aria-hidden': 'true' }), el('span', {}, statusText())];
    if (dirty) parts.push(el('span', { class: 'status-dirty' }, ` · ${t('status.dirty')}`));
    if (!sandbox && ui.result) parts.push(el('span', { class: ui.result.passed ? 'status-ok' : 'status-bad' }, ` · ${t('status.tests', { passed: ui.result.tests.filter((x) => x.passed).length, total: ui.result.tests.length })}`));
    mount(statusChip, parts);
  }

  let pending = false;
  function refresh(light = false) {
    if (light) {
      if (pending) return;
      pending = true;
      requestAnimationFrame(() => { pending = false; doRefresh(true); });
      return;
    }
    doRefresh(false);
  }

  function doRefresh(light) {
    const st = session.state();
    const assembled = st.status !== STATUS.NOT_ASSEMBLED && ui.assembledSource !== null;
    const mem = session.memory();
    const line = assembled ? ui.addrToLine.get(st.pc) ?? null : null;
    renderToolbar();
    renderStatus();
    const counts = new Map(assembled ? session.lineCounts() : []);
    editor.update({ current: assembled && st.status !== STATUS.HALTED ? line : null, hot: ui.hotLine, errors: ui.diagnostics, counts, breakpoints: ui.bpLines, codeLines: new Set(ui.lineToAddr.keys()) });
    const editable = assembled && !ui.running && (st.status === STATUS.PAUSED || st.status === STATUS.BREAKPOINT);
    registers.update({ regs: st.regs, flags: st.flags, pc: st.pc, line, prevRegs: ui.prev?.regs, prevFlags: ui.prev?.flags, changedRegs: ui.changedRegs, editable, canTravel: assembled && st.steps > 0 && !ui.running });
    const frames = assembled ? session.frames() : [];
    const mtab = root.dataset.mtab;
    const narrow = matchMedia('(max-width: 759px)').matches;
    const visible = (id) => !narrow || mtab === id;
    if (visible('stack')) stack.update({ mem, regs: st.regs, frames, changedBytes: ui.changedBytes, quiet: ui.quietBytes, assembled, lineOf: (a) => ui.addrToLine.get(a) ?? null, trapAddr: ui.trapAddr });
    if (ui.expectedFor !== ui.testIndex) {
      ui.expectedFor = ui.testIndex;
      ui.expected = level && ui.testIndex >= 0 && level.tests[ui.testIndex]?.screen ? session.referenceScreen?.(ui.testIndex) ?? null : null;
    }
    const expected = ui.expected;
    if (visible('screen')) screen.update({ mem, expected });
    if (visible('console')) {
      const test = level && ui.testIndex >= 0 ? level.tests[ui.testIndex] : null;
      const output = session.output();
      consoleView.update({
        state: st,
        output,
        outInts: output.filter((o) => o.kind === 0).map((o) => o.value),
        input: test ? { ints: test.in ?? [], text: test.in_text ?? '' } : { ints: ui.typedInts, text: ui.typedText },
        expectedOut: test?.out ?? null,
        focusInput: sandbox,
      });
    }
    if (!light && visible('mem')) {
      if (memTabs.current === 'hex') hexdump.update({ mem, changedBytes: ui.changedBytes, quiet: ui.quietBytes, labels: ui.labels, pc: st.pc, sp: st.regs[9] });
      if (memTabs.current === 'heap') heapmap.update({ shadow: session.shadow(), heap: assembled ? session.heap() : [] });
    }
    if (!sandbox && !light) tests.update({ level, result: ui.result, current: ui.testIndex });
  }

  // ------------------------------------------------------------ start
  renderTask();
  renderHints();
  slider.addEventListener('input', () => {
    const v = Number(slider.value);
    cancelAnimationFrame(ui.sliderRaf);
    ui.sliderRaf = requestAnimationFrame(() => gotoStep(v));
  });
  refresh();

  return {
    node: root,
    focusTarget: root.querySelector('#ws-title'),
    onAction: act,
    destroy() {
      stop();
      clearTimeout(saveTimer);
      progress.saveCode(storeId, editor.value);
      stack.destroy?.();
      session.free();
    },
  };
}

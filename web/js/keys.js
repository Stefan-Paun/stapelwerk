// Keyboard shortcuts → actions. Pure: takes a plain event-like object.
// Plain letters act when focus is not in a text field. While typing in the
// editor they rest; only Ctrl/⌘ combos that do not collide with editing or
// browser shortcuts act there (Ctrl/⌘+Enter, Ctrl/⌘+S step, Ctrl/⌘+B breakpoint).

export const SHORTCUTS = [
  { key: 's', action: 'step' },
  { key: 'z', action: 'back' },
  { key: 'l', action: 'runPause' },
  { key: 'r', action: 'restart' },
  { key: 'b', action: 'breakpoint' },
  { key: 't', action: 'tests' },
  { key: '?', action: 'help' },
];

const BY_KEY = new Map(SHORTCUTS.map((s) => [s.key, s.action]));
const IN_EDITOR_WITH_MOD = new Map([['s', 'step'], ['b', 'breakpoint']]);

/**
 * @param {{key: string, ctrlKey?: boolean, metaKey?: boolean, altKey?: boolean, shiftKey?: boolean}} ev
 * @param {{typing?: boolean}} ctx  typing = focus is in the editor or another text field
 * @returns {string|null} action name
 */
export function actionForKey(ev, ctx = {}) {
  if (!ev || typeof ev.key !== 'string' || ev.key === '') return null;
  const mod = Boolean(ev.ctrlKey || ev.metaKey);
  const typing = Boolean(ctx.typing);
  if (ev.altKey) return null;
  if (ev.key === 'Enter') return mod ? 'assembleTest' : null;
  if (ev.key === 'Escape') return typing ? 'leaveEditor' : null;
  const key = ev.key.length === 1 ? ev.key.toLowerCase() : ev.key;
  if (mod) {
    if (!typing || ev.shiftKey) return null;
    return IN_EDITOR_WITH_MOD.get(key) ?? null;
  }
  if (typing) return null;
  return BY_KEY.get(key) ?? null;
}

// Accessible tabs (WAI-ARIA tabs pattern, manual activation by click/Enter,
// arrow keys move between tabs).
import { el } from '../dom.js';

let seq = 0;

/**
 * items: [{id, label, panel: Node, hidden?: boolean}]
 * Returns {node, list, select(id), setHidden(id, bool), current}
 */
export function createTabs({ label, items, initial, onChange, className = '' }) {
  const base = `tabs${++seq}`;
  let current = initial ?? items[0].id;
  const buttons = new Map();
  const panels = new Map();
  const list = el('div', { class: 'tablist', role: 'tablist', 'aria-label': label });
  const panelWrap = el('div', { class: 'tabpanels' });
  for (const item of items) {
    const tabId = `${base}-${item.id}-tab`;
    const panelId = `${base}-${item.id}-panel`;
    const btn = el('button', {
      type: 'button', role: 'tab', id: tabId, class: 'tab', 'aria-controls': panelId,
      onclick: () => select(item.id, true),
    }, item.label);
    const panel = el('div', { role: 'tabpanel', id: panelId, class: 'tabpanel', 'aria-labelledby': tabId, tabindex: '0' }, item.panel);
    if (item.hidden) btn.hidden = true;
    buttons.set(item.id, btn);
    panels.set(item.id, panel);
    list.append(btn);
    panelWrap.append(panel);
  }
  list.addEventListener('keydown', (ev) => {
    if (!['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(ev.key)) return;
    const visible = [...buttons.entries()].filter(([, b]) => !b.hidden);
    const idx = visible.findIndex(([id]) => id === current);
    let next = idx;
    if (ev.key === 'ArrowRight') next = (idx + 1) % visible.length;
    if (ev.key === 'ArrowLeft') next = (idx - 1 + visible.length) % visible.length;
    if (ev.key === 'Home') next = 0;
    if (ev.key === 'End') next = visible.length - 1;
    ev.preventDefault();
    select(visible[next][0], true);
    visible[next][1].focus();
  });

  function select(id, user = false, notify = true) {
    if (!buttons.has(id)) return;
    current = id;
    for (const [k, b] of buttons) {
      const on = k === id;
      b.setAttribute('aria-selected', on ? 'true' : 'false');
      b.tabIndex = on ? 0 : -1;
      panels.get(k).hidden = !on;
    }
    if (notify) onChange?.(id, user);
  }
  select(current, false, false);

  return {
    node: el('div', { class: ['tabs', className] }, list, panelWrap),
    list,
    select,
    setHidden(id, hidden) { const b = buttons.get(id); if (b) b.hidden = hidden; },
    get current() { return current; },
  };
}

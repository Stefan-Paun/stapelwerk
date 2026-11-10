// Small store with subscribers. The VM state itself is never copied in here;
// views read it from the Session. The store only carries a tick counter that
// bumps whenever the machine changed, plus UI state.

export function createStore(initial) {
  let state = { tick: 0, ...initial };
  const subscribers = new Set();

  function get() {
    return state;
  }

  function set(patch) {
    const next = typeof patch === 'function' ? patch(state) : patch;
    let changed = false;
    for (const key of Object.keys(next)) {
      if (state[key] !== next[key]) { changed = true; break; }
    }
    if (!changed) return;
    const prev = state;
    state = { ...state, ...next };
    for (const fn of subscribers) fn(state, prev);
  }

  /** Signal that the machine changed (step, run slice, reset, …). */
  function bump() {
    set({ tick: state.tick + 1 });
  }

  function subscribe(fn) {
    subscribers.add(fn);
    return () => subscribers.delete(fn);
  }

  return { get, set, bump, subscribe };
}

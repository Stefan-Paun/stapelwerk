// Tiny DOM helpers. Text only ever goes through textContent; attributes through
// setAttribute. No HTML strings anywhere in this code base (a test enforces it).

const SVG_NS = 'http://www.w3.org/2000/svg';

function applyAttrs(node, attrs) {
  if (!attrs) return;
  for (const [name, value] of Object.entries(attrs)) {
    if (value === undefined || value === null || value === false) continue;
    if (name === 'class') node.setAttribute('class', Array.isArray(value) ? value.filter(Boolean).join(' ') : value);
    else if (name === 'text') node.textContent = String(value);
    else if (name === 'dataset') for (const [k, v] of Object.entries(value)) node.dataset[k] = String(v);
    else if (name.startsWith('on') && typeof value === 'function') node.addEventListener(name.slice(2), value);
    else if (name === 'value' && 'value' in node) node.value = value;
    else if (name === 'checked' && 'checked' in node) node.checked = Boolean(value);
    else node.setAttribute(name, value === true ? '' : String(value));
  }
}

function append(node, children) {
  for (const child of children.flat(Infinity)) {
    if (child === null || child === undefined || child === false) continue;
    node.append(child instanceof Node ? child : document.createTextNode(String(child)));
  }
}

/** Create an HTML element: el('button', {class: 'x', onclick}, 'Label', childNode). */
export function el(tag, attrs, ...children) {
  const node = document.createElement(tag);
  applyAttrs(node, attrs);
  append(node, children);
  return node;
}

/** Create an SVG element in the SVG namespace. */
export function svg(tag, attrs, ...children) {
  const node = document.createElementNS(SVG_NS, tag);
  applyAttrs(node, attrs);
  append(node, children);
  return node;
}

/** Replace all children of a node. */
export function mount(node, ...children) {
  node.replaceChildren();
  append(node, children);
  return node;
}

/** Set text only when it changed (keeps screen readers and layout calm). */
export function setText(node, text) {
  const value = String(text);
  if (node.textContent !== value) node.textContent = value;
}

/** Toggle a boolean attribute or class quickly. */
export function toggleClass(node, name, on) {
  node.classList.toggle(name, Boolean(on));
}

export function frag(...children) {
  const f = document.createDocumentFragment();
  append(f, children);
  return f;
}

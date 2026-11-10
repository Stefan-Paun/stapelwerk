// Safe Markdown subset for level and manual texts:
//   # / ## headings, paragraphs, **bold**, *italic*, `code`, ``` code blocks,
//   "- " lists, links only to the manual: [Text](handbuch:topic) or (manual:topic).
// parseMarkup() is pure and returns a small tree; toDom() turns it into nodes
// with createElement/textContent only. HTML in the source stays text.

const LINK_TARGET = /^(?:handbuch|manual):([a-z0-9][a-z0-9-]{0,63})$/;

/** Parse inline text into a list of inline nodes. */
export function parseInline(text) {
  const out = [];
  let buf = '';
  const flush = () => { if (buf) { out.push({ type: 'text', text: buf }); buf = ''; } };
  let i = 0;
  while (i < text.length) {
    const ch = text[i];
    if (ch === '`') {
      const end = text.indexOf('`', i + 1);
      if (end > i) { flush(); out.push({ type: 'code', text: text.slice(i + 1, end) }); i = end + 1; continue; }
    }
    if (ch === '*' && text[i + 1] === '*') {
      const end = text.indexOf('**', i + 2);
      if (end > i + 2) { flush(); out.push({ type: 'strong', children: parseInline(text.slice(i + 2, end)) }); i = end + 2; continue; }
    }
    if (ch === '*' && text[i + 1] !== '*' && text[i + 1] !== ' ' && text[i + 1] !== undefined) {
      const end = text.indexOf('*', i + 1);
      if (end > i + 1 && text[end - 1] !== ' ') { flush(); out.push({ type: 'em', children: parseInline(text.slice(i + 1, end)) }); i = end + 1; continue; }
    }
    if (ch === '[') {
      const close = text.indexOf('](', i + 1);
      const end = close > 0 ? text.indexOf(')', close + 2) : -1;
      if (close > 0 && end > 0) {
        const label = text.slice(i + 1, close);
        const target = text.slice(close + 2, end).trim();
        const m = LINK_TARGET.exec(target);
        flush();
        if (m) out.push({ type: 'link', topic: m[1], children: parseInline(label) });
        else out.push(...parseInline(label)); // foreign links become plain text
        i = end + 1;
        continue;
      }
    }
    buf += ch;
    i += 1;
  }
  flush();
  return out;
}

/** Parse a whole text into block nodes. */
export function parseMarkup(source) {
  const lines = String(source ?? '').replace(/\r\n?/g, '\n').split('\n');
  const blocks = [];
  let para = [];
  let list = null;
  const endPara = () => {
    if (para.length) { blocks.push({ type: 'p', children: parseInline(para.join(' ')) }); para = []; }
  };
  const endList = () => { if (list) { blocks.push(list); list = null; } };
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    if (line.startsWith('```')) {
      endPara(); endList();
      const lang = line.slice(3).trim();
      const body = [];
      i += 1;
      while (i < lines.length && !lines[i].startsWith('```')) { body.push(lines[i]); i += 1; }
      blocks.push({ type: 'pre', lang, text: body.join('\n') });
      continue;
    }
    const heading = /^(#{1,2}) +(.*)$/.exec(line);
    if (heading) {
      endPara(); endList();
      blocks.push({ type: heading[1].length === 1 ? 'h1' : 'h2', children: parseInline(heading[2].trim()) });
      continue;
    }
    const item = /^- +(.*)$/.exec(line);
    if (item) {
      endPara();
      if (!list) list = { type: 'ul', items: [] };
      list.items.push(parseInline(item[1]));
      continue;
    }
    if (line.trim() === '') { endPara(); endList(); continue; }
    if (list && /^ {2,}\S/.test(line)) {
      const last = list.items[list.items.length - 1];
      last.push({ type: 'text', text: ' ' }, ...parseInline(line.trim()));
      continue;
    }
    endList();
    para.push(line.trim());
  }
  endPara(); endList();
  return blocks;
}

function inlineToDom(doc, nodes, opts) {
  const frag = doc.createDocumentFragment();
  for (const n of nodes) {
    if (n.type === 'text') frag.append(doc.createTextNode(n.text));
    else if (n.type === 'code') { const c = doc.createElement('code'); c.textContent = n.text; frag.append(c); }
    else if (n.type === 'strong' || n.type === 'em') {
      const e = doc.createElement(n.type);
      e.append(inlineToDom(doc, n.children, opts));
      frag.append(e);
    } else if (n.type === 'link') {
      const a = doc.createElement('a');
      a.setAttribute('href', opts.linkHref ? opts.linkHref(n.topic) : `#/manual/${n.topic}`);
      a.setAttribute('class', 'term');
      a.append(inlineToDom(doc, n.children, opts));
      frag.append(a);
    }
  }
  return frag;
}

/**
 * Turn parsed blocks into a DocumentFragment.
 * opts.headingLevel shifts headings (h1 → h{headingLevel}), default 3.
 */
export function toDom(blocks, opts = {}, doc = globalThis.document) {
  const base = opts.headingLevel ?? 3;
  const frag = doc.createDocumentFragment();
  for (const b of blocks) {
    if (b.type === 'p') { const p = doc.createElement('p'); p.append(inlineToDom(doc, b.children, opts)); frag.append(p); }
    else if (b.type === 'h1' || b.type === 'h2') {
      const level = Math.min(6, base + (b.type === 'h2' ? 1 : 0));
      const h = doc.createElement(`h${level}`);
      h.append(inlineToDom(doc, b.children, opts));
      frag.append(h);
    } else if (b.type === 'ul') {
      const ul = doc.createElement('ul');
      for (const item of b.items) { const li = doc.createElement('li'); li.append(inlineToDom(doc, item, opts)); ul.append(li); }
      frag.append(ul);
    } else if (b.type === 'pre') {
      const pre = doc.createElement('pre');
      const code = doc.createElement('code');
      code.textContent = b.text;
      if (b.lang) pre.setAttribute('data-lang', b.lang);
      pre.append(code);
      frag.append(pre);
    }
  }
  return frag;
}

/** Convenience: source text → fragment. */
export function renderMarkup(source, opts = {}, doc = globalThis.document) {
  return toDom(parseMarkup(source), opts, doc);
}

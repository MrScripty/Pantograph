import * as commonmark from 'commonmark';

export const escapeHtml = text => text.replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;').replaceAll('"', '&quot;');

// Read textual heading content from the parsed document, never by stripping tags
// from HTML. The normal safe renderer still handles inline formatting and URLs.
export function renderResearchMarkdown(tree) {
  const headings = [];
  const used = new Map();
  const ids = new WeakMap();
  const walker = tree.walker();
  let event;
  while ((event = walker.next())) {
    const node = event.node;
    if (!event.entering || node.type !== 'heading') continue;
    let plain = '';
    const children = node.walker();
    let child;
    while ((child = children.next())) {
      if (!child.entering) continue;
      if (child.node.type === 'text' || child.node.type === 'code') plain += child.node.literal;
      else if (child.node.type === 'softbreak' || child.node.type === 'linebreak') plain += ' ';
    }
    const base = plain.toLowerCase().replace(/[^\p{L}\p{N}\s_-]/gu, '').replace(/\s/g, '-');
    const count = used.get(base) ?? 0;
    used.set(base, count + 1);
    const id = base + (count ? `-${count}` : '');
    ids.set(node, id);
    if (node.level <= 2) headings.push(`<a href="#${escapeHtml(id)}">${escapeHtml(plain)}</a>`);
  }
  const renderer = new commonmark.HtmlRenderer({ safe: true });
  renderer.heading = function (node, entering) {
    const tag = `h${node.level}`;
    if (entering) {
      this.cr();
      this.tag(tag, [...this.attrs(node), ['id', escapeHtml(ids.get(node))]]);
    } else {
      this.tag(`/${tag}`);
      this.cr();
    }
  };
  return { html: renderer.render(tree), headings };
}

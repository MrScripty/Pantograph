// Build the static GitHub Pages presentation without deploying it.
import fs from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { createHash } from 'node:crypto';
import * as commonmark from 'commonmark';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const output = path.join(root, 'dist', 'presentation');
const researchRoot = path.join(root, 'docs', 'research');
const manifest = JSON.parse(await fs.readFile(path.join(researchRoot, 'source-manifest.json'), 'utf8'));
for (const file of manifest.files) {
  const content = await fs.readFile(path.join(researchRoot, file.path));
  if (createHash('sha256').update(content).digest('hex') !== file.sha256) {
    throw new Error(`Research edition hash mismatch: ${file.path}`);
  }
}
await fs.mkdir(output, { recursive: true });
await fs.cp(path.join(root, 'docs', 'presentation'), output, { recursive: true });
await fs.cp(researchRoot, path.join(output, 'research'), { recursive: true });
await fs.cp(path.join(root, 'docs', 'images'), path.join(output, 'images'), { recursive: true });
await fs.writeFile(path.join(output, '.nojekyll'), '');
const escape = text => text.replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;').replaceAll('"', '&quot;');
const manuscripts = ['README.md', ...manifest.files.filter(file => file.path.endsWith('.md')).map(file => file.path)];
const parser = new commonmark.Parser();
for (const file of manuscripts) {
  const target = path.join(output, 'research', file.replace(/\.md$/, '.html'));
  const prefix = path.relative(path.dirname(target), output).split(path.sep).join('/');
  const tree = parser.parse(await fs.readFile(path.join(researchRoot, file), 'utf8'));
  const walker = tree.walker();
  let event;
  while ((event = walker.next())) {
    const node = event.node;
    if (!event.entering || node.type !== 'link' || !node.destination || /^(?:[a-z]+:|#|\/)/i.test(node.destination)) continue;
    const [destination, anchor] = node.destination.split('#');
    const resolved = path.resolve(researchRoot, path.dirname(file), destination);
    const researchRelative = path.relative(researchRoot, resolved).split(path.sep).join('/');
    if (manuscripts.includes(researchRelative)) {
      node.destination = destination.replace(/\.md$/, '.html') + (anchor ? `#${anchor}` : '');
    } else if (researchRelative.startsWith('..')) {
      node.destination = `https://github.com/MrScripty/Pantograph/blob/main/${path.relative(root, resolved).split(path.sep).join('/')}${anchor ? `#${anchor}` : ''}`;
    }
  }
  const headings = [];
  const used = new Map();
  const html = new commonmark.HtmlRenderer({ safe: true }).render(tree).replace(/<h([1-6])>([\s\S]*?)<\/h\1>/g, (_, level, text) => {
    const plain = text.replace(/<[^>]*>/g, '').replaceAll('&amp;', '&');
    const base = plain.toLowerCase().replace(/[^\p{L}\p{N}\s_-]/gu, '').replace(/\s/g, '-');
    const count = used.get(base) ?? 0; used.set(base, count + 1);
    const id = base + (count ? `-${count}` : '');
    if (Number(level) <= 2) headings.push(`<a href="#${escape(id)}">${text}</a>`);
    return `<h${level} id="${escape(id)}">${text}</h${level}>`;
  });
  const title = file === 'README.md' ? 'Research sources' : file.split('/').at(-1).replace('.md', '').replaceAll('-', ' ');
  await fs.writeFile(target, `<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>${escape(title)} · Pantograph</title><link rel="stylesheet" href="${prefix}/style.css"><body><main class="reader"><a href="${prefix}/index.html#research">← Pantograph research</a><p class="reader-notice">Research edition: 2 October 2026. Source inspection is pinned to 4938e405. Proposals and standalone simulations are not production qualification. <a href="${escape(file.split('/').at(-1))}">Original Markdown</a></p><details><summary>Contents</summary><nav class="reader-toc">${headings.join('')}</nav></details>${html}</main></body></html>`);
}
console.log(`Built ${output}; rendered ${manuscripts.length} research documents; verified original manuscript hashes. No deployment performed.`);

/* Local stand-in for the Design canvas runtime. Renders <x-dc> templates:
   {{path}} interpolation, <sc-for list as>, <sc-if value>, onClick/onChange handlers,
   data-props defaults with a small tweak panel. Enough to view the mockups offline. */
(function () {
  class DCLogic {
    constructor(props) { this.props = props; this.state = {}; }
    setState(patch) { Object.assign(this.state, patch); scheduleRender(); }
  }
  window.DCLogic = DCLogic;

  let component, template, mount, propsSpec, renderQueued = false;

  function lit(s) {
    s = s.trim();
    if (s === 'true') return true;
    if (s === 'false') return false;
    if (s === 'null') return null;
    if (/^-?\d+(\.\d+)?$/.test(s)) return Number(s);
    if (/^'.*'$/.test(s) || /^".*"$/.test(s)) return s.slice(1, -1);
    return undefined;
  }
  function lookup(path, scope, vals) {
    const l = lit(path);
    if (l !== undefined) return l;
    const parts = path.split('.');
    let cur = Object.prototype.hasOwnProperty.call(scope, parts[0]) ? scope[parts[0]] : vals[parts[0]];
    for (let i = 1; i < parts.length && cur != null; i++) cur = cur[parts[i]];
    return cur;
  }
  const RE_ONE = /^\s*\{\{\s*([^}]*?)\s*\}\}\s*$/;
  const RE_ALL = /\{\{\s*([^}]*?)\s*\}\}/g;
  function evalExpr(str, scope, vals) {
    const m = str.match(RE_ONE);
    if (m) return lookup(m[1], scope, vals);
    return interp(str, scope, vals);
  }
  function interp(str, scope, vals) {
    if (!str.includes('{{')) return str;
    return str.replace(RE_ALL, (_, p) => { const v = lookup(p, scope, vals); return v == null ? '' : String(v); });
  }
  function parseAttrs(s) {
    const out = {}; const re = /([\w-]+)="([^"]*)"/g; let m;
    while ((m = re.exec(s))) out[m[1]] = m[2];
    return out;
  }

  function renderChildren(src, scope, vals) { return renderList(Array.from(src.childNodes), scope, vals); }
  function renderList(kids, scope, vals) {
    const out = []; let i = 0;
    while (i < kids.length) {
      const k = kids[i];
      if (k.nodeType === 8) {
        const m = k.data.match(/^\s*sc-(for|if)\s+([\s\S]*)$/);
        if (m) {
          let depth = 1, j = i + 1;
          for (; j < kids.length; j++) {
            if (kids[j].nodeType !== 8) continue;
            if (/^\s*sc-(for|if)\b/.test(kids[j].data)) depth++;
            else if (/^\s*\/sc-(for|if)/.test(kids[j].data)) { depth--; if (depth === 0) break; }
          }
          const body = kids.slice(i + 1, j); const attrs = parseAttrs(m[2]);
          if (m[1] === 'for') {
            const list = evalExpr(attrs.list || '', scope, vals) || [];
            list.forEach((item) => { const sc = Object.assign({}, scope); sc[attrs.as] = item; out.push(...renderList(body, sc, vals)); });
          } else if (evalExpr(attrs.value || '', scope, vals)) {
            out.push(...renderList(body, scope, vals));
          }
          i = j + 1; continue;
        }
        i++; continue;
      }
      out.push(...renderNode(k, scope, vals)); i++;
    }
    return out;
  }
  function renderNode(n, scope, vals) {
    if (n.nodeType === 3) { const t = document.createTextNode(interp(n.data, scope, vals)); return [t]; }
    if (n.nodeType !== 1) return [];
    const el = n.cloneNode(false);
    for (const a of Array.from(el.attributes)) {
      if (a.name.startsWith('hint-')) { el.removeAttribute(a.name); continue; }
      if (!a.value.includes('{{')) continue;
      if (a.name.startsWith('on')) {
        const fn = evalExpr(a.value, scope, vals);
        el.removeAttribute(a.name);
        if (typeof fn === 'function') el.addEventListener(a.name.slice(2), fn);
        continue;
      }
      const v = evalExpr(a.value, scope, vals);
      el.setAttribute(a.name, v == null ? '' : String(v));
    }
    if (el.hasAttribute('aria-disabled') && el.getAttribute('aria-disabled') === 'true') el.style.opacity = '.5';
    renderChildren(n, scope, vals).forEach((c) => el.appendChild(c));
    if ((el.tagName === 'INPUT' || el.tagName === 'SELECT' || el.tagName === 'TEXTAREA') && n.hasAttribute('value')) {
      const v = el.getAttribute('value'); if (v != null) requestAnimationFrame(() => { el.value = v; });
    }
    return [el];
  }

  function scheduleRender() { if (renderQueued) return; renderQueued = true; requestAnimationFrame(render); }
  function render() {
    renderQueued = false;
    const vals = component.renderVals();
    const nodes = renderChildren(template, {}, vals);
    const ae = document.activeElement; const key = ae && ae.dataset ? ae.dataset.key : null;
    const pos = key && 'selectionStart' in ae ? ae.selectionStart : null;
    mount.replaceChildren(...nodes);
    if (key) { const again = mount.querySelector('[data-key="' + key + '"]'); if (again) { again.focus(); if (pos != null) try { again.setSelectionRange(pos, pos); } catch (_) {} } }
    else { const af = mount.querySelector('[data-autofocus]'); if (af && !component._afDone) { component._afDone = true; af.focus(); } if (!af) component._afDone = false; }
    relink(mount);
  }
  const PAGES = { sessions: 'Sessions.html', scheduler: 'Revised.html', diagnostics: 'Diagnostics.html', io: 'IoInspector.html', network: 'Network.html', graph: 'Graph.html', nodes: 'NodeEditor.html', library: 'Library.html' };
  function relink(root) {
    if (!/^https?:$/.test(location.protocol)) return;
    root.querySelectorAll('a[href^="#"]').forEach((a) => { const page = PAGES[a.getAttribute('href').slice(1)]; if (page) a.setAttribute('href', page); });
  }

  function readProps(spec) {
    const props = {}; const q = new URLSearchParams(location.search);
    for (const [k, d] of Object.entries(spec)) {
      if (k.startsWith('$')) continue;
      let v = d.default;
      if (q.has(k)) { const raw = q.get(k); v = d.editor === 'boolean' ? raw === 'true' || raw === '1' : raw; }
      props[k] = v;
    }
    return props;
  }
  function tweakPanel(spec, props) {
    const keys = Object.keys(spec).filter((k) => !k.startsWith('$'));
    if (!keys.length) return;
    const box = document.createElement('div');
    box.setAttribute('aria-label', 'Tweaks');
    box.style.cssText = 'position:fixed;right:10px;bottom:10px;z-index:9999;font:12px system-ui,sans-serif;color:#E4E6EA;background:rgba(35,39,45,.95);border:1px solid #4A5059;border-radius:4px;padding:6px 8px;display:flex;gap:10px;align-items:center;opacity:.35;transition:opacity .15s';
    box.onmouseenter = () => (box.style.opacity = '1'); box.onmouseleave = () => (box.style.opacity = '.35');
    const home = document.createElement('a'); home.href = './index.html'; home.textContent = 'All boards'; home.style.cssText = 'color:#D9B06A;text-decoration:none;margin-right:4px'; box.appendChild(home);
    keys.forEach((k) => {
      const d = spec[k]; const lab = document.createElement('label'); lab.style.cssText = 'display:inline-flex;gap:4px;align-items:center';
      lab.appendChild(document.createTextNode(k));
      let input;
      if (d.editor === 'boolean') { input = document.createElement('input'); input.type = 'checkbox'; input.checked = !!props[k]; input.onchange = () => { props[k] = input.checked; scheduleRender(); }; }
      else if (d.options || d.editor === 'enum') { input = document.createElement('select'); (d.options || []).forEach((o) => { const op = document.createElement('option'); op.value = o; op.textContent = o; op.selected = o === props[k]; input.appendChild(op); }); input.onchange = () => { props[k] = input.value; scheduleRender(); }; }
      else { input = document.createElement('input'); input.type = d.editor === 'color' ? 'color' : 'text'; input.value = props[k]; input.onchange = () => { props[k] = input.value; scheduleRender(); }; }
      input.style.cssText = 'font:inherit;background:#23272D;color:inherit;border:1px solid #4A5059;border-radius:3px;height:22px';
      lab.appendChild(input); box.appendChild(lab);
    });
    document.body.appendChild(box);
  }

  async function boot() {
    const src = await (await fetch(location.href, { cache: 'no-store' })).text();
    const xm = src.match(/<x-dc>([\s\S]*?)<\/x-dc>/);
    if (!xm) return;
    let body = xm[1];
    const hm = body.match(/<helmet>([\s\S]*?)<\/helmet>/);
    if (hm) { body = body.replace(hm[0], ''); const h = document.createElement('div'); h.innerHTML = hm[1]; Array.from(h.childNodes).forEach((n) => document.head.appendChild(n)); }
    body = body.replace(/<sc-(for|if)\s+([^>]*)>/g, '<!--sc-$1 $2-->').replace(/<\/sc-(for|if)>/g, '<!--/sc-$1-->');
    template = document.createElement('div'); template.innerHTML = body;
    const scriptEl = document.querySelector('script[data-dc-script]');
    propsSpec = JSON.parse(scriptEl.getAttribute('data-props') || '{}');
    const props = readProps(propsSpec);
    const Component = new Function('DCLogic', scriptEl.textContent + '\nreturn Component;')(DCLogic);
    component = new Component(props);
    const old = document.querySelector('x-dc'); mount = document.createElement('div'); mount.id = 'app';
    if (old) old.replaceWith(mount); else document.body.prepend(mount);
    render();
    tweakPanel(propsSpec, props);
  }
  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', boot); else boot();
})();

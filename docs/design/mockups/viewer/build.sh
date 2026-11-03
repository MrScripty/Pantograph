#!/usr/bin/env bash
# Builds the local mockup site from ../boards into ./site and serves it on http://127.0.0.1:8765/
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
src="$here/../boards"
out="$here/site"
mkdir -p "$out"
cp "$here/support.js" "$out/support.js"
cp "$src/shell.css" "$out/shell.css"
cp "$here/current-scheduler.png" "$out/current-scheduler.png"
for f in "$src"/*.dc.html; do
  b="$(basename "$f" .dc.html)"
  sed -e 's#src="/_blob/[0-9a-f]*"#src="./current-scheduler.png"#' "$f" > "$out/$b.html"
done
python3 - "$out/index.html" <<'PY'
import sys, pathlib
rows = [("Round 1: current build and three directions", ["Current", "Main", "Studio", "Instrument"]),
        ("Round 2: D and E (Scheduler)", ["Dark", "Revised"]),
        ("Round 3: every tab in E's language", ["Sessions", "Diagnostics", "IoInspector", "Network", "Graph", "NodeEditor", "Library"])]
cards = []
for title, names in rows:
    cards.append(f'<h2>{title}</h2><div class="row">')
    for n in names:
        cards.append(f'<a class="card" href="{n}.html"><iframe src="{n}.html" tabindex="-1" loading="lazy"></iframe><span>{n}</span></a>')
    cards.append('</div>')
html = '''<!doctype html><html lang="en"><head><meta charset="utf-8"><title>Pantograph GUI mockups</title>
<style>body{margin:0;padding:24px 28px;background:#2A2E35;color:#E4E6EA;font:14px/1.45 system-ui,sans-serif}h1{font:500 22px Georgia,serif;margin:0 0 4px}p{color:#A3A9B4;margin:0 0 18px}h2{font-size:13px;color:#A3A9B4;font-weight:700;margin:22px 0 10px}.row{display:grid;grid-template-columns:repeat(auto-fill,minmax(360px,1fr));gap:16px}.card{display:block;color:inherit;text-decoration:none;border:1px solid #4A5059;border-radius:4px;background:#33383F;overflow:hidden;position:relative}.card:hover{border-color:#D9B06A}.card iframe{position:absolute;top:0;left:0;width:1440px;height:900px;border:0;transform:scale(.25);transform-origin:0 0;pointer-events:none}.card::before{content:"";display:block;padding-top:62.5%}.card span{display:block;padding:8px 10px;border-top:1px solid #4A5059;background:#2E333A;position:relative}</style></head><body>
<h1>Pantograph GUI mockups</h1><p>Click a board to open it full size. Boards share one run context; Ctrl+C over a run copies its id, Ctrl+V over the chip views it.</p>''' + ''.join(cards) + '</body></html>'
pathlib.Path(sys.argv[1]).write_text(html)
PY
echo "built in $out"
if [ "${1:-}" = "--serve" ]; then exec python3 -m http.server 8765 --directory "$out"; fi

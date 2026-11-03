#!/usr/bin/env python3
"""Expands the shared run-context picker into every board.
Markup goes where a board has <!--ctx--> (or between <!--ctx-start--> ... <!--ctx-end-->);
the mixin goes where the class has /*ctx*/ (or between /*ctx-start*/ ... /*ctx-end*/). Idempotent."""
import pathlib, re, sys
here = pathlib.Path(__file__).parent
markup = (here / "ctx-markup.html").read_text().rstrip("\n")
mixin = (here / "ctx-mixin.js").read_text().rstrip("\n")
M0, M1 = "<!--ctx-start-->", "<!--ctx-end-->"
J0, J1 = "/*ctx-start*/", "/*ctx-end*/"
for f in sorted((here / "project").glob("*.dc.html")):
    s = f.read_text()
    if "<!--ctx-->" not in s and M0 not in s:
        continue
    s = s.replace("<!--ctx-->", f"{M0}\n{markup}\n    {M1}")
    s = re.sub(re.escape(M0) + r"[\s\S]*?" + re.escape(M1), lambda _: f"{M0}\n{markup}\n    {M1}", s)
    s = s.replace("/*ctx*/", f"{J0}\n{mixin}\n  {J1}")
    s = re.sub(re.escape(J0) + r"[\s\S]*?" + re.escape(J1), lambda _: f"{J0}\n{mixin}\n  {J1}", s)
    f.write_text(s)
    print("injected", f.name)

#!/usr/bin/env python3
import json
import re
import sys
from pathlib import Path

if len(sys.argv) != 2:
    print("usage: parse_cutechess.py <logfile>", file=sys.stderr)
    sys.exit(2)

text = Path(sys.argv[1]).read_text(errors="replace")

out = {
    "elo_difference": None,
    "elo_error": None,
    "draw_ratio_pct": None,
    "los_pct": None,
    "wins": None,
    "losses": None,
    "draws": None,
    "games": None,
}

m = re.search(r"Elo difference:\s*([-\w.]+)\s*\+/-\s*([-\w.]+),\s*LOS:\s*([-\w.]+)\s*%,\s*DrawRatio:\s*([-\w.]+)\s*%", text)
if m:
    out["elo_difference"] = m.group(1)
    out["elo_error"] = m.group(2)
    out["los_pct"] = m.group(3)
    out["draw_ratio_pct"] = m.group(4)

scores = re.findall(r"Score of .*?:\s*(\d+)\s*-\s*(\d+)\s*-\s*(\d+)\s*\[.*?\]\s*(\d+)", text)
if scores:
    w, l, d, g = scores[-1]
    out["wins"] = int(w)
    out["losses"] = int(l)
    out["draws"] = int(d)
    out["games"] = int(g)

print(json.dumps(out, indent=2))

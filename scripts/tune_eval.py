#!/usr/bin/env python3
import json
import os
import random
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ENGINE = ROOT / "target/release/incremental-chess-engine"
GAUNTLET = ROOT / "scripts/gauntlet.sh"
OUT = ROOT / "benchmarks/results"
WEIGHTS = ROOT / "benchmarks/weights"
WEIGHTS.mkdir(parents=True, exist_ok=True)


def run(cmd):
    p = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    if p.returncode != 0:
        print(p.stdout)
        raise SystemExit(p.returncode)
    return p.stdout


def dump_default(path: Path):
    run(["bash", "-lc", f"printf 'dumpweights {path}\\nquit\\n' | \"{ENGINE}\""])


def load_vec(path: Path):
    txt = path.read_text().strip()
    return [int(x) for x in txt.split()] if txt else []


def save_vec(path: Path, vec):
    path.write_text(" ".join(str(x) for x in vec))


def eval_vs_base(candidate: Path, base: Path, games=60, tc="10+0.1"):
    name = f"tune-{candidate.stem}"
    run(
        [
            str(GAUNTLET),
            "--engine-a",
            str(ENGINE),
            "--engine-a-dir",
            str(ROOT),
            "--engine-a-name",
            "Cand",
            "--engine-b",
            str(ENGINE),
            "--engine-b-dir",
            str(ROOT),
            "--engine-b-name",
            "Base",
            "--weights-a",
            str(candidate),
            "--weights-b",
            str(base),
            "--games",
            str(games),
            "--tc",
            tc,
            "--name",
            name,
        ]
    )
    js = sorted(OUT.glob(f"{name}-*.json"))[-1]
    d = json.loads(js.read_text())
    elo = float(d["elo_difference"]) if d["elo_difference"] not in (None, "nan", "-inf", "inf") else -9999.0
    return elo


def main():
    random.seed(7)
    base = WEIGHTS / "base.txt"
    if not base.exists():
        dump_default(base)
    vec = load_vec(base)
    if not vec:
        raise SystemExit("could not load base weights")

    # Tune only the last eval extras (mobility/king safety/passed scaling).
    tune_idx = list(range(len(vec) - 13, len(vec)))
    cur = vec[:]
    cur_path = WEIGHTS / "cur.txt"
    save_vec(cur_path, cur)
    cur_elo = eval_vs_base(cur_path, base, games=40, tc="5+0.1")
    print("initial elo vs base:", cur_elo)

    for it in range(12):
        cand = cur[:]
        for i in tune_idx:
            cand[i] += random.randint(-4, 4)
        cand_path = WEIGHTS / f"cand-{it}.txt"
        save_vec(cand_path, cand)
        elo = eval_vs_base(cand_path, base, games=40, tc="5+0.1")
        print(f"iter {it}: elo={elo}")
        if elo > cur_elo:
            cur_elo = elo
            cur = cand
            save_vec(WEIGHTS / "best.txt", cur)
            print("accepted")

    print("best elo:", cur_elo)
    print("best weights:", WEIGHTS / "best.txt")


if __name__ == "__main__":
    main()

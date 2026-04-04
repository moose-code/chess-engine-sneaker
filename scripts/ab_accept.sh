#!/usr/bin/env bash
set -euo pipefail

# A/B acceptance harness:
# - runs gauntlet
# - parses summary JSON
# - exits 0 only if elo > 0 and LOS >= threshold
#
# Example:
# scripts/ab_accept.sh \
#   --new "./benchmarks/bin/new-engine" \
#   --base "./benchmarks/bin/base-engine" \
#   --games 600 --tc "10+0.1" --los 95

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
GAUNTLET="$ROOT/scripts/gauntlet.sh"

NEW_ENGINE=""
BASE_ENGINE=""
GAMES=400
TC="10+0.1"
LOS_MIN=95
NAME="ab"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --new) NEW_ENGINE="$2"; shift 2 ;;
    --base) BASE_ENGINE="$2"; shift 2 ;;
    --games) GAMES="$2"; shift 2 ;;
    --tc) TC="$2"; shift 2 ;;
    --los) LOS_MIN="$2"; shift 2 ;;
    --name) NAME="$2"; shift 2 ;;
    *)
      echo "Unknown arg: $1" >&2
      exit 1
      ;;
  esac
done

if [[ -z "$NEW_ENGINE" || -z "$BASE_ENGINE" ]]; then
  echo "Need --new and --base engine binaries." >&2
  exit 1
fi

"$GAUNTLET" \
  --engine-a "$NEW_ENGINE" --engine-a-dir "." --engine-a-name "New" \
  --engine-b "$BASE_ENGINE" --engine-b-dir "." --engine-b-name "Base" \
  --games "$GAMES" --tc "$TC" --name "$NAME"

LATEST_JSON="$(ls -1t "$ROOT"/benchmarks/results/"$NAME"-*.json | head -1)"
if [[ -z "$LATEST_JSON" ]]; then
  echo "No summary json found." >&2
  exit 2
fi

python3 - "$LATEST_JSON" "$LOS_MIN" <<'PY'
import json, sys
p = sys.argv[1]
los_min = float(sys.argv[2])
d = json.load(open(p))
elo = float(d["elo_difference"]) if d["elo_difference"] not in (None, "nan", "-inf", "inf") else -9999
los = float(d["los_pct"]) if d["los_pct"] not in (None, "nan") else 0.0
print(f"A/B summary: elo={elo}, los={los}%")
if elo > 0 and los >= los_min:
    print("ACCEPT")
    sys.exit(0)
print("REJECT")
sys.exit(1)
PY

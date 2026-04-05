#!/usr/bin/env bash
set -euo pipefail

# Reproducible cutechess gauntlet runner.
# Example:
# scripts/gauntlet.sh \
#   --engine-a "./target/release/incremental-chess-engine" \
#   --engine-a-dir "." \
#   --engine-b "stockfish" \
#   --engine-b-dir "." \
#   --games 400 --tc "10+0.1" --name "baseline"

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CLI="$ROOT/cutechess/build/cutechess-cli"
OUT_DIR="$ROOT/benchmarks/results"
OPENINGS_FILE="$ROOT/benchmarks/openings/startpos.pgn"

ENGINE_A=""
ENGINE_A_DIR="$ROOT"
ENGINE_A_NAME="EngineA"
ENGINE_B=""
ENGINE_B_DIR="$ROOT"
ENGINE_B_NAME="EngineB"
GAMES=200
TC="10+0.1"
NAME="run"
THREADS_A=1
THREADS_B=1
HASH_A=64
HASH_B=64
WEIGHTS_A=""
WEIGHTS_B=""
TB_PATH=""
TB_PIECES=6
ENGINE_A_OPTS=""
ENGINE_B_OPTS=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --engine-a) ENGINE_A="$2"; shift 2 ;;
    --engine-a-dir) ENGINE_A_DIR="$2"; shift 2 ;;
    --engine-a-name) ENGINE_A_NAME="$2"; shift 2 ;;
    --engine-b) ENGINE_B="$2"; shift 2 ;;
    --engine-b-dir) ENGINE_B_DIR="$2"; shift 2 ;;
    --engine-b-name) ENGINE_B_NAME="$2"; shift 2 ;;
    --games) GAMES="$2"; shift 2 ;;
    --tc) TC="$2"; shift 2 ;;
    --name) NAME="$2"; shift 2 ;;
    --openings-file) OPENINGS_FILE="$2"; shift 2 ;;
    --threads-a) THREADS_A="$2"; shift 2 ;;
    --threads-b) THREADS_B="$2"; shift 2 ;;
    --hash-a) HASH_A="$2"; shift 2 ;;
    --hash-b) HASH_B="$2"; shift 2 ;;
    --weights-a) WEIGHTS_A="$2"; shift 2 ;;
    --weights-b) WEIGHTS_B="$2"; shift 2 ;;
    --tb-path) TB_PATH="$2"; shift 2 ;;
    --tb-pieces) TB_PIECES="$2"; shift 2 ;;
    --engine-a-options) ENGINE_A_OPTS="$2"; shift 2 ;;
    --engine-b-options) ENGINE_B_OPTS="$2"; shift 2 ;;
    *)
      echo "Unknown argument: $1" >&2
      exit 1
      ;;
  esac
done

if [[ -z "$ENGINE_A" || -z "$ENGINE_B" ]]; then
  echo "Both --engine-a and --engine-b are required." >&2
  exit 1
fi
if [[ ! -x "$CLI" ]]; then
  echo "Missing cutechess-cli at $CLI" >&2
  exit 1
fi

mkdir -p "$OUT_DIR" "$(dirname "$OPENINGS_FILE")"

if [[ ! -f "$OPENINGS_FILE" ]]; then
  cat > "$OPENINGS_FILE" <<'PGN'
[Event "StartPos"]
[Site "?"]
[Date "2026.01.01"]
[Round "1"]
[White "W"]
[Black "B"]
[Result "*"]

*
PGN
fi

STAMP="$(date +%Y%m%d-%H%M%S)"
LOG="$OUT_DIR/${NAME}-${STAMP}.log"
PGN="$OUT_DIR/${NAME}-${STAMP}.pgn"

echo "Running gauntlet..."
echo "  A: $ENGINE_A_NAME ($ENGINE_A)"
echo "  B: $ENGINE_B_NAME ($ENGINE_B)"
echo "  games=$GAMES tc=$TC"
echo "  log=$LOG"

TB_ARGS=()
if [[ -n "$TB_PATH" ]]; then
  TB_ARGS=( -tb "$TB_PATH" -tbpieces "$TB_PIECES" )
fi
WEIGHT_OPT_A=()
WEIGHT_OPT_B=()
if [[ -n "$WEIGHTS_A" ]]; then
  WEIGHT_OPT_A=( option.EvalWeightsFile="$WEIGHTS_A" )
fi
if [[ -n "$WEIGHTS_B" ]]; then
  WEIGHT_OPT_B=( option.EvalWeightsFile="$WEIGHTS_B" )
fi
EXTRA_A=()
EXTRA_B=()
if [[ -n "$ENGINE_A_OPTS" ]]; then
  IFS=',' read -r -a arr <<< "$ENGINE_A_OPTS"
  for kv in "${arr[@]}"; do
    kv="$(echo "$kv" | xargs)"
    [[ -z "$kv" ]] && continue
    EXTRA_A+=( "option.$kv" )
  done
fi
if [[ -n "$ENGINE_B_OPTS" ]]; then
  IFS=',' read -r -a arr <<< "$ENGINE_B_OPTS"
  for kv in "${arr[@]}"; do
    kv="$(echo "$kv" | xargs)"
    [[ -z "$kv" ]] && continue
    EXTRA_B+=( "option.$kv" )
  done
fi

"$CLI" \
  -engine cmd="$ENGINE_A" dir="$ENGINE_A_DIR" proto=uci name="$ENGINE_A_NAME" \
    option.Threads="$THREADS_A" option.Hash="$HASH_A" ${WEIGHT_OPT_A[@]+"${WEIGHT_OPT_A[@]}"} ${EXTRA_A[@]+"${EXTRA_A[@]}"} \
  -engine cmd="$ENGINE_B" dir="$ENGINE_B_DIR" proto=uci name="$ENGINE_B_NAME" \
    option.Threads="$THREADS_B" option.Hash="$HASH_B" ${WEIGHT_OPT_B[@]+"${WEIGHT_OPT_B[@]}"} ${EXTRA_B[@]+"${EXTRA_B[@]}"} \
  -each tc="$TC" \
  -games "$GAMES" -repeat \
  -openings file="$OPENINGS_FILE" format=pgn order=random plies=16 \
  -pgnout "$PGN" \
  ${TB_ARGS[@]+"${TB_ARGS[@]}"} \
  -recover \
  2>&1 | tee "$LOG"

python3 "$ROOT/scripts/parse_cutechess.py" "$LOG" > "${LOG%.log}.json"
echo "Wrote summary: ${LOG%.log}.json"

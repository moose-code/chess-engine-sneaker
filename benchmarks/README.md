# Baseline and A/B Gauntlets

Use this folder to run reproducible Elo tests and compare patch sets.

## 1) Build

```bash
cargo build --release --target-dir target
```

## 2) Run baseline (recommended 400-1000 games)

```bash
scripts/gauntlet.sh \
  --engine-a "./target/release/incremental-chess-engine" \
  --engine-a-dir "." \
  --engine-a-name "Incremental" \
  --engine-b "stockfish" \
  --engine-b-dir "." \
  --engine-b-name "Stockfish" \
  --games 400 \
  --tc "10+0.1" \
  --threads-a 4 --threads-b 4 \
  --hash-a 256 --hash-b 256 \
  --openings-file "benchmarks/openings/blitz-suite.pgn" \
  --name "baseline-vs-stockfish"
```

If `stockfish` is not on PATH, pass an absolute path in `--engine-b`.

To benchmark against a real Stockfish 2500 setting:

```bash
scripts/gauntlet.sh \
  --engine-a "./target/release/incremental-chess-engine" \
  --engine-a-name "Sneaker" \
  --engine-b "stockfish" \
  --engine-b-name "SF2500" \
  --engine-b-options "UCI_LimitStrength=true,UCI_Elo=2500" \
  --games 200 --tc "3+0.1" --threads-a 4 --threads-b 4
```

## 3) Compare two local revisions (A/B)

- Check out revision A, build, copy binary to `benchmarks/bin/engine-a`.
- Check out revision B, build, copy binary to `benchmarks/bin/engine-b`.
- Run gauntlet with `--engine-a` and `--engine-b` pointing to those files.

## 4) Outputs

- `.log`: raw cutechess output
- `.pgn`: games
- `.json`: parsed summary with Elo line, LOS, draw ratio, score

Store one baseline per major patch set and reject noisy/non-significant gains.

### Optional Syzygy adjudication in matches

If you have Syzygy WDL files locally:

```bash
scripts/gauntlet.sh \
  --engine-a "./target/release/incremental-chess-engine" \
  --engine-b "stockfish" \
  --games 200 --tc "10+0.1" \
  --tb-path "/path/to/syzygy" \
  --tb-pieces 6 \
  --name "tb-check"
```

## 5) Weight tuning loop (hybrid path)

The engine supports:

- `setoption name EvalWeightsFile value /path/to/weights.txt`
- custom command `dumpweights /path/to/weights.txt`

Use:

```bash
python3 scripts/tune_eval.py
```

This runs a lightweight hill-climbing loop over the advanced eval terms
(mobility/king-safety/passed scaling) using short A/B gauntlets.

## 6) A/B acceptance gate

```bash
scripts/ab_accept.sh \
  --new ./benchmarks/bin/new-engine \
  --base ./benchmarks/bin/base-engine \
  --games 600 --tc "10+0.1" --los 95
```

Accept only if Elo is positive and LOS clears the threshold.

## 7) Per-engine UCI option overrides

`gauntlet.sh` supports comma-separated per-engine option overrides:

- `--engine-a-options "Key1=Val1,Key2=Val2"`
- `--engine-b-options "Key1=Val1,Key2=Val2"`

Examples:

- Disable singular extension for Sneaker:
  `--engine-a-options "UseSingular=false"`
- Match Stockfish at a target Elo:
  `--engine-b-options "UCI_LimitStrength=true,UCI_Elo=2500"`

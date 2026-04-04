# incremental-chess-engine

A small **UCI** chess engine in Rust: bitboard board representation, legal move generation (perft-tested), and a search stack aimed at steady strength gains (negamax, alpha–beta, iterative deepening, transposition table, quiescence, killers, history, null-move pruning, late-move reductions, aspiration windows, tapered evaluation).

## Requirements

- [Rust](https://www.rust-lang.org/) toolchain (2021 edition; stable is fine)

## Build and test

```bash
cargo build --release
cargo test
```

Release binary: `target/release/incremental-chess-engine`  
Tests use `opt-level = 3` so deep perft stays quick.

## Run (UCI)

The engine speaks UCI on stdin/stdout:

```bash
./target/release/incremental-chess-engine
```

Typical GUI commands: `uci`, `isready`, `position startpos`, `position fen … moves …`, `go depth 12`, `go movetime 500`, `go wtime … btime … winc … binc …`, `setoption name Hash value 16`, `ucinewgame`, `quit`.

Non-standard helpers for debugging:

- `perft 6` — node count at depth 6 from the current position  
- `d` / `display` — side to move, hash, clocks  

Point any **UCI-capable GUI** (or [Cute Chess](https://github.com/cutechess/cutechess), etc.) at the binary; set the working directory to this repo if the GUI supports it.

### cutechess-cli example

After installing Cute Chess’s CLI:

```bash
cutechess-cli \
  -engine cmd=./target/release/incremental-chess-engine dir=. proto=uci \
  -engine cmd=stockfish proto=uci \
  -each tc=60+0.6 -games 100 -repeat -openings file=book.pgn format=pgn order=random plies=16
```

Adjust paths, opening book, and opponent as needed.

## Crate layout

| Path | Role |
|------|------|
| `src/board.rs` | Bitboards, FEN, Zobrist, make/unmake |
| `src/movegen.rs` | Move generation, `perft` |
| `src/search.rs` | Negamax, TT, null move, LMR, aspirations |
| `src/eval.rs` | Tapered eval, tunable `EvalWeights` |
| `src/uci.rs` | UCI loop |
| `src/types.rs` | Pieces, squares, packed moves |
| `tests/perft.rs` | Start position perft regression |
| `tests/epd_smoke.rs` | Shallow tactical sanity checks |

## Roadmap ideas

Faster movegen (no board clones), SEE, opening book, Syzygy, multi-threaded search, Texel-style tuning using `EvalWeights::apply_from_i32_slice`.

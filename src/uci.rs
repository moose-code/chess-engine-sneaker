//! UCI protocol loop, custom `perft`, debug helpers.

use crate::board::Board;
use crate::eval::load_weights_from_file;
use crate::movegen::MoveGen;
use crate::nnue::NnueModel;
use crate::search::Search;
use crate::types::Color;
use std::fs;
use std::io::{self, BufRead, Write};
use std::time::Duration;

pub fn run_uci_loop() {
    let mut out = io::stdout();
    let stdin = io::stdin();
    let mut board = Board::new();
    let mut search = Search::new();

    for line in stdin.lock().lines().map_while(Result::ok) {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut parts = line.split_whitespace();
        let cmd = parts.next().unwrap_or("");

        match cmd {
            "uci" => {
                writeln!(out, "id name Incremental").ok();
                writeln!(out, "id author incremental-chess-engine").ok();
                writeln!(
                    out,
                    "option name Hash type spin default 16 min 1 max 512"
                )
                .ok();
                writeln!(
                    out,
                    "option name Threads type spin default 1 min 1 max 8"
                )
                .ok();
                writeln!(
                    out,
                    "option name EvalWeightsFile type string default <empty>"
                )
                .ok();
                writeln!(
                    out,
                    "option name NnueFile type string default <empty>"
                )
                .ok();
                writeln!(
                    out,
                    "option name UseSeePrune type check default true"
                )
                .ok();
                writeln!(
                    out,
                    "option name UseSingular type check default false"
                )
                .ok();
                writeln!(
                    out,
                    "option name UseSmpRoot type check default true"
                )
                .ok();
                writeln!(out, "uciok").ok();
            }
            "isready" => {
                writeln!(out, "readyok").ok();
            }
            "ucinewgame" => {
                board = Board::new();
                search = Search::new();
            }
            "debug" => {}
            "position" => {
                let tok: Vec<&str> = line.split_whitespace().collect();
                let mut idx = 1usize;
                let mut b = if idx < tok.len() && tok[idx] == "startpos" {
                    idx += 1;
                    Board::new()
                } else if idx < tok.len() && tok[idx] == "fen" {
                    idx += 1;
                    let start = idx;
                    while idx < tok.len() && tok[idx] != "moves" {
                        idx += 1;
                    }
                    let fen = tok[start..idx].join(" ");
                    Board::from_fen(&fen).unwrap_or_else(|_| Board::new())
                } else {
                    Board::new()
                };
                if idx < tok.len() && tok[idx] == "moves" {
                    idx += 1;
                }
                while idx < tok.len() {
                    let mut legal = Vec::with_capacity(256);
                    MoveGen::gen_legal(&mut legal, &mut b);
                    if let Some(m) = legal.into_iter().find(|m| m.to_uci() == tok[idx]) {
                        let _undo = b.make_move(m);
                    } else {
                        // Invalid move list from GUI/CLI: stop replay to avoid board desync.
                        break;
                    }
                    idx += 1;
                }
                board = b;
            }
            "setoption" => {
                let tok: Vec<&str> = line.split_whitespace().collect();
                if tok.len() >= 5
                    && tok[1] == "name"
                    && tok[2] == "Hash"
                    && tok[3] == "value"
                {
                    if let Ok(mb) = tok[4].parse::<usize>() {
                        search.set_tt_size(mb);
                    }
                } else if tok.len() >= 5
                    && tok[1] == "name"
                    && tok[2] == "Threads"
                    && tok[3] == "value"
                {
                    if let Ok(t) = tok[4].parse::<usize>() {
                        search.set_threads(t);
                    }
                } else if tok.len() >= 5
                    && tok[1] == "name"
                    && tok[2] == "EvalWeightsFile"
                    && tok[3] == "value"
                {
                    let path = tok[4..].join(" ");
                    if let Some(w) = load_weights_from_file(path.trim()) {
                        search.weights = w;
                    }
                } else if tok.len() >= 5
                    && tok[1] == "name"
                    && tok[2] == "NnueFile"
                    && tok[3] == "value"
                {
                    let path = tok[4..].join(" ");
                    search.set_nnue(NnueModel::load(path.trim()));
                } else if tok.len() >= 5
                    && tok[1] == "name"
                    && tok[2] == "UseSeePrune"
                    && tok[3] == "value"
                {
                    let on = matches!(
                        tok[4].to_ascii_lowercase().as_str(),
                        "true" | "1" | "yes" | "on"
                    );
                    search.set_use_see_prune(on);
                } else if tok.len() >= 5
                    && tok[1] == "name"
                    && tok[2] == "UseSingular"
                    && tok[3] == "value"
                {
                    let on = matches!(
                        tok[4].to_ascii_lowercase().as_str(),
                        "true" | "1" | "yes" | "on"
                    );
                    search.set_use_singular(on);
                } else if tok.len() >= 5
                    && tok[1] == "name"
                    && tok[2] == "UseSmpRoot"
                    && tok[3] == "value"
                {
                    let on = matches!(
                        tok[4].to_ascii_lowercase().as_str(),
                        "true" | "1" | "yes" | "on"
                    );
                    search.set_use_smp_root(on);
                }
            }
            "go" => {
                let mut depth = 6i32;
                let mut movetime_ms: Option<u64> = None;
                let mut wtime: Option<u64> = None;
                let mut btime: Option<u64> = None;
                let mut winc: Option<u64> = None;
                let mut binc: Option<u64> = None;
                let mut pit = parts.peekable();
                while let Some(p) = pit.next() {
                    match p {
                        "depth" => {
                            if let Some(d) = pit.next() {
                                depth = d.parse().unwrap_or(depth);
                            }
                        }
                        "movetime" => {
                            if let Some(ms) = pit.next() {
                                movetime_ms = ms.parse().ok();
                            }
                        }
                        "wtime" => {
                            if let Some(ms) = pit.next() {
                                wtime = ms.parse().ok();
                            }
                        }
                        "btime" => {
                            if let Some(ms) = pit.next() {
                                btime = ms.parse().ok();
                            }
                        }
                        "winc" => {
                            if let Some(ms) = pit.next() {
                                winc = ms.parse().ok();
                            }
                        }
                        "binc" => {
                            if let Some(ms) = pit.next() {
                                binc = ms.parse().ok();
                            }
                        }
                        "infinite" => {
                            depth = 64;
                        }
                        _ => {}
                    }
                }
                // movetime 0 makes the deadline immediate → no search → bestmove 0000 → GUI forfeit.
                if let Some(ms) = movetime_ms {
                    if ms > 0 {
                        search.set_deadline_after(Duration::from_millis(ms));
                    } else {
                        search.set_deadline(None);
                    }
                } else if let (Some(wt), Some(bt)) = (wtime, btime) {
                    let wi = winc.unwrap_or(0);
                    let bi = binc.unwrap_or(0);
                    let my_time = match board.side_to_move() {
                        Color::White => wt.saturating_add(wi),
                        Color::Black => bt.saturating_add(bi),
                    };
                    // Old formula (my_time/40).max(50).min(my_time.saturating_sub(50)) hits 0 when
                    // my_time < 50 → instant timeout → bestmove 0000 / "disconnects".
                    if my_time == 0 {
                        search.set_deadline(None);
                    } else {
                        let slice = (my_time / 20).max(1).min(my_time);
                        search.set_deadline_after(Duration::from_millis(slice));
                    }
                } else {
                    search.set_deadline(None);
                }
                if let Some((bm, sc)) = search.best_move(&mut board, depth) {
                    writeln!(out, "info score cp {}", sc).ok();
                    writeln!(out, "bestmove {}", bm.to_uci()).ok();
                } else {
                    let mut buf = Vec::with_capacity(256);
                    MoveGen::gen_legal(&mut buf, &mut board);
                    if let Some(m) = buf.first() {
                        writeln!(out, "bestmove {}", m.to_uci()).ok();
                    } else {
                        writeln!(out, "bestmove 0000").ok();
                    }
                }
            }
            "perft" => {
                if let Some(d) = parts.next().and_then(|s| s.parse::<u32>().ok()) {
                    let n = MoveGen::perft(&mut board, d);
                    writeln!(out, "info string perft {} {}", d, n).ok();
                }
            }
            "dumpweights" => {
                if let Some(path) = parts.next() {
                    let flat = search.weights.to_flat_i32_vec();
                    let txt = flat
                        .iter()
                        .map(|v| v.to_string())
                        .collect::<Vec<_>>()
                        .join(" ");
                    if fs::write(path, txt).is_ok() {
                        writeln!(out, "info string dumped weights {}", path).ok();
                    }
                }
            }
            "d" | "display" => {
                let stm = match board.side_to_move() {
                    Color::White => 'w',
                    Color::Black => 'b',
                };
                writeln!(
                    out,
                    "info string stm {} hash {:x} half {} full {}",
                    stm,
                    board.hash(),
                    board.halfmove_clock(),
                    board.fullmove_number()
                )
                .ok();
            }
            "quit" => break,
            _ => {}
        }
        out.flush().ok();
    }
}

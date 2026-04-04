//! UCI protocol loop, custom `perft`, debug helpers.

use crate::board::Board;
use crate::movegen::MoveGen;
use crate::search::Search;
use crate::types::{Color, Move};
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
                    if let Ok(m) = Move::from_uci(tok[idx]) {
                        let _undo = b.make_move(m);
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
                if let Some(ms) = movetime_ms {
                    search.set_deadline_after(Duration::from_millis(ms));
                } else if let (Some(wt), Some(bt)) = (wtime, btime) {
                    let wi = winc.unwrap_or(0);
                    let bi = binc.unwrap_or(0);
                    let my_time = match board.side_to_move() {
                        Color::White => wt.saturating_add(wi),
                        Color::Black => bt.saturating_add(bi),
                    };
                    let time_for_move =
                        (my_time / 40).max(50).min(my_time.saturating_sub(50));
                    search.set_deadline_after(Duration::from_millis(time_for_move));
                } else {
                    search.set_deadline(None);
                }
                if let Some((bm, sc)) = search.best_move(&mut board, depth) {
                    writeln!(out, "info score cp {}", sc).ok();
                    writeln!(out, "bestmove {}", bm.to_uci()).ok();
                } else {
                    writeln!(out, "bestmove 0000").ok();
                }
            }
            "perft" => {
                if let Some(d) = parts.next().and_then(|s| s.parse::<u32>().ok()) {
                    let n = MoveGen::perft(&mut board, d);
                    writeln!(out, "info string perft {} {}", d, n).ok();
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

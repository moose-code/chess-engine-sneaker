//! Tactical smoke tests at low depth.

use incremental_chess_engine::board::Board;
use incremental_chess_engine::eval::EvalWeights;
use incremental_chess_engine::search::Search;
use incremental_chess_engine::types::Move;

fn best_at_depth(fen: &str, depth: i32) -> Option<Move> {
    let mut b = Board::from_fen(fen).ok()?;
    let mut s = Search::new();
    s.weights = EvalWeights::default();
    s.best_move(&mut b, depth).map(|(m, _)| m)
}

#[test]
fn epd_back_rank_mate() {
    let fen = "6k1/5ppp/8/8/8/8/5PPP/4R1K1 w - - 0 1";
    let bm = best_at_depth(fen, 5).expect("move");
    assert_eq!(bm.to_uci(), "e1e8");
}

#[test]
fn epd_hanging_queen_capture() {
    let fen = "Q3k3/8/8/8/8/8/8/q3K3 w - - 0 1";
    let bm = best_at_depth(fen, 6).expect("move");
    assert_eq!(bm.to_uci(), "a8a1");
}

#[test]
fn epd_only_winning_queen_move() {
    // Black d5 is weak; Bxd5 and exd5 are both crushing at moderate depth.
    let fen = "r1bqkb1r/pppp1ppp/2n2n2/3pp3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 0 1";
    let bm = best_at_depth(fen, 8).expect("move");
    let uci = bm.to_uci();
    assert!(
        uci == "c4d5" || uci == "e4d5",
        "expected Bxd5 or exd5, got {}",
        uci
    );
}

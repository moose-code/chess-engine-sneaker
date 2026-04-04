//! Start position perft regression.

use incremental_chess_engine::board::Board;
use incremental_chess_engine::movegen::MoveGen;

#[test]
fn startpos_perft_6() {
    let mut b = Board::new();
    assert_eq!(MoveGen::perft(&mut b, 0), 1);
    assert_eq!(MoveGen::perft(&mut b, 1), 20);
    assert_eq!(MoveGen::perft(&mut b, 2), 400);
    assert_eq!(MoveGen::perft(&mut b, 3), 8902);
    assert_eq!(MoveGen::perft(&mut b, 4), 197281);
    assert_eq!(MoveGen::perft(&mut b, 5), 4865609);
    assert_eq!(MoveGen::perft(&mut b, 6), 119_060_324);
}

use incremental_chess_engine::board::Board;
use incremental_chess_engine::search::Search;

fn parse_line(line: &str) -> Option<(String, Vec<String>)> {
    let mut parts = line.split(';');
    let fen = parts.next()?.trim().to_string();
    let mut best = Vec::new();
    for p in parts {
        let t = p.trim();
        if let Some(rest) = t.strip_prefix("bm ") {
            best = rest
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }
    }
    if fen.is_empty() || best.is_empty() {
        return None;
    }
    Some((fen, best))
}

#[test]
fn blunder_bank_positions_hold() {
    let txt = std::fs::read_to_string("tests/blunder_bank.epd").expect("epd");
    for line in txt.lines().filter(|l| !l.trim().is_empty()) {
        let (fen, bests) = parse_line(line).expect("parse");
        let mut b = Board::from_fen(&fen).expect("fen");
        let mut s = Search::new();
        let (m, _) = s.best_move(&mut b, 8).expect("best move");
        let u = m.to_uci();
        assert!(
            bests.iter().any(|x| x == &u),
            "expected one of {:?}, got {} in {}",
            bests,
            u,
            fen
        );
    }
}

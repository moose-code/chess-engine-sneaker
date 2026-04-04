//! Tapered evaluation (MG/EG), tunable weights.

use crate::board::Board;
use crate::types::{Color, PieceType};

#[derive(Clone, Copy, Debug, Default)]
pub struct TaperedScore {
    pub mg: i32,
    pub eg: i32,
}

impl TaperedScore {
    #[inline]
    pub const fn new(mg: i32, eg: i32) -> Self {
        Self { mg, eg }
    }

    #[inline]
    pub fn add_assign(&mut self, o: Self) {
        self.mg += o.mg;
        self.eg += o.eg;
    }
}

#[derive(Clone, Debug)]
pub struct EvalWeights {
    pub piece_mg: [i32; 6],
    pub piece_eg: [i32; 6],
    pub pst_mg: [[i32; 64]; 6],
    pub pst_eg: [[i32; 64]; 6],
    pub passed_mg: i32,
    pub passed_eg: i32,
    pub doubled_mg: i32,
    pub doubled_eg: i32,
    pub isolated_mg: i32,
    pub isolated_eg: i32,
}

impl Default for EvalWeights {
    fn default() -> Self {
        Self::standard()
    }
}

impl EvalWeights {
    pub fn standard() -> Self {
        let mut w = Self {
            piece_mg: [100, 320, 330, 500, 900, 20000],
            piece_eg: [120, 300, 320, 550, 950, 20000],
            pst_mg: [[0; 64]; 6],
            pst_eg: [[0; 64]; 6],
            passed_mg: 20,
            passed_eg: 60,
            doubled_mg: -12,
            doubled_eg: -18,
            isolated_mg: -8,
            isolated_eg: -15,
        };
        init_default_pst(&mut w);
        w
    }

    /// Flat order: piece_mg×6, piece_eg×6, pst_mg 6×64, pst_eg 6×64, passed_mg,eg, doubled_mg,eg, isolated_mg,eg
    pub fn apply_from_i32_slice(&mut self, data: &[i32]) {
        const N: usize = 6 + 6 + 6 * 64 + 6 * 64 + 6;
        if data.len() < N {
            return;
        }
        let mut i = 0usize;
        for j in 0..6 {
            self.piece_mg[j] = data[i];
            i += 1;
        }
        for j in 0..6 {
            self.piece_eg[j] = data[i];
            i += 1;
        }
        for p in 0..6 {
            for sq in 0..64 {
                self.pst_mg[p][sq] = data[i];
                i += 1;
            }
        }
        for p in 0..6 {
            for sq in 0..64 {
                self.pst_eg[p][sq] = data[i];
                i += 1;
            }
        }
        self.passed_mg = data[i];
        self.passed_eg = data[i + 1];
        self.doubled_mg = data[i + 2];
        self.doubled_eg = data[i + 3];
        self.isolated_mg = data[i + 4];
        self.isolated_eg = data[i + 5];
    }
}

fn set_pst_row(tab_mg: &mut [i32; 64], tab_eg: &mut [i32; 64], rank: u8, mg: &[i32; 8], eg: &[i32; 8]) {
    for f in 0u8..8 {
        let sq = (rank as usize) * 8 + f as usize;
        tab_mg[sq] = mg[f as usize];
        tab_eg[sq] = eg[f as usize];
    }
}

fn mirror_white_pst(out_mg: &mut [[i32; 64]; 6], out_eg: &mut [[i32; 64]; 6], pt: usize, w_mg: &[[i32; 8]; 8], w_eg: &[[i32; 8]; 8]) {
    for r in 0u8..8 {
        set_pst_row(&mut out_mg[pt], &mut out_eg[pt], r, &w_mg[r as usize], &w_eg[r as usize]);
    }
}

fn init_default_pst(w: &mut EvalWeights) {
    let pawn_mg: [[i32; 8]; 8] = [
        [0; 8],
        [0; 8],
        [0; 8],
        [0; 8],
        [-2; 8],
        [-4; 8],
        [6; 8],
        [0; 8],
    ];
    let pawn_eg: [[i32; 8]; 8] = [
        [0; 8],
        [0; 8],
        [0; 8],
        [10; 8],
        [20; 8],
        [40; 8],
        [0; 8],
        [0; 8],
    ];
    let knight_mg: [[i32; 8]; 8] = [
        [-50, -40, -30, -30, -30, -30, -40, -50],
        [-40, -20, 0, 0, 0, 0, -20, -40],
        [-30, 0, 10, 15, 15, 10, 0, -30],
        [-30, 5, 15, 20, 20, 15, 5, -30],
        [-30, 0, 15, 20, 20, 15, 0, -30],
        [-30, 5, 10, 15, 15, 10, 5, -30],
        [-40, -20, 0, 0, 0, 0, -20, -40],
        [-50, -40, -30, -30, -30, -30, -40, -50],
    ];
    let knight_eg = knight_mg;
    let bishop_mg: [[i32; 8]; 8] = [
        [-20, -10, -10, -10, -10, -10, -10, -20],
        [-10, 0, 0, 0, 0, 0, 0, -10],
        [-10, 0, 5, 10, 10, 5, 0, -10],
        [-10, 5, 5, 10, 10, 5, 5, -10],
        [-10, 0, 10, 10, 10, 10, 0, -10],
        [-10, 10, 10, 10, 10, 10, 10, -10],
        [-10, 5, 0, 0, 0, 0, 5, -10],
        [-20, -10, -10, -10, -10, -10, -10, -20],
    ];
    let bishop_eg = bishop_mg;
    let rook_mg: [[i32; 8]; 8] = [
        [0, 0, 5, 10, 10, 5, 0, 0],
        [0, 0, 5, 10, 10, 5, 0, 0],
        [0, 0, 5, 10, 10, 5, 0, 0],
        [0, 0, 5, 10, 10, 5, 0, 0],
        [0, 0, 5, 10, 10, 5, 0, 0],
        [0, 0, 5, 10, 10, 5, 0, 0],
        [0, 0, 5, 10, 10, 5, 0, 0],
        [0, 0, 5, 10, 10, 5, 0, 0],
    ];
    let rook_eg: [[i32; 8]; 8] = [
        [0, 0, 5, 10, 10, 5, 0, 0],
        [0, 0, 5, 10, 10, 5, 0, 0],
        [0, 0, 5, 10, 10, 5, 0, 0],
        [0, 0, 5, 10, 10, 5, 0, 0],
        [10, 10, 15, 20, 20, 15, 10, 10],
        [10, 10, 15, 20, 20, 15, 10, 10],
        [10, 10, 15, 20, 20, 15, 10, 10],
        [10, 10, 15, 20, 20, 15, 10, 10],
    ];
    let queen_mg: [[i32; 8]; 8] = [
        [-20, -10, -10, -5, -5, -10, -10, -20],
        [-10, 0, 0, 0, 0, 0, 0, -10],
        [-10, 0, 5, 5, 5, 5, 0, -10],
        [-5, 0, 5, 5, 5, 5, 0, -5],
        [0, 0, 5, 5, 5, 5, 0, -5],
        [-10, 0, 5, 5, 5, 5, 0, -10],
        [-10, 0, 0, 0, 0, 0, 0, -10],
        [-20, -10, -10, -5, -5, -10, -10, -20],
    ];
    let queen_eg = queen_mg;
    let king_mg: [[i32; 8]; 8] = [
        [-30, -40, -40, -50, -50, -40, -40, -30],
        [-30, -40, -40, -50, -50, -40, -40, -30],
        [-30, -40, -40, -50, -50, -40, -40, -30],
        [-30, -40, -40, -50, -50, -40, -40, -30],
        [-20, -30, -30, -40, -40, -30, -30, -20],
        [-10, -20, -20, -20, -20, -20, -20, -10],
        [20, 20, 0, 0, 0, 0, 20, 20],
        [20, 30, 10, 0, 0, 10, 30, 20],
    ];
    let king_eg: [[i32; 8]; 8] = [
        [-50, -10, -10, -10, -10, -10, -10, -50],
        [-10, 20, 30, 30, 30, 30, 20, -10],
        [-10, 30, 40, 50, 50, 40, 30, -10],
        [-10, 30, 40, 50, 50, 40, 30, -10],
        [-10, 30, 40, 50, 50, 40, 30, -10],
        [-10, 30, 40, 50, 50, 40, 30, -10],
        [-10, 20, 30, 30, 30, 30, 20, -10],
        [-50, -10, -10, -10, -10, -10, -10, -50],
    ];

    mirror_white_pst(&mut w.pst_mg, &mut w.pst_eg, PieceType::Pawn.idx(), &pawn_mg, &pawn_eg);
    mirror_white_pst(
        &mut w.pst_mg,
        &mut w.pst_eg,
        PieceType::Knight.idx(),
        &knight_mg,
        &knight_eg,
    );
    mirror_white_pst(
        &mut w.pst_mg,
        &mut w.pst_eg,
        PieceType::Bishop.idx(),
        &bishop_mg,
        &bishop_eg,
    );
    mirror_white_pst(&mut w.pst_mg, &mut w.pst_eg, PieceType::Rook.idx(), &rook_mg, &rook_eg);
    mirror_white_pst(
        &mut w.pst_mg,
        &mut w.pst_eg,
        PieceType::Queen.idx(),
        &queen_mg,
        &queen_eg,
    );
    mirror_white_pst(&mut w.pst_mg, &mut w.pst_eg, PieceType::King.idx(), &king_mg, &king_eg);
}

fn phase_of(b: &Board) -> i32 {
    let mut ph = 0i32;
    for c in [Color::White, Color::Black] {
        for pt in [
            PieceType::Knight,
            PieceType::Bishop,
            PieceType::Rook,
            PieceType::Queen,
        ] {
            let cnt = b.piece_bb[c.idx()][pt.idx()].count_ones() as i32;
            let w = match pt {
                PieceType::Knight => 1,
                PieceType::Bishop => 1,
                PieceType::Rook => 2,
                PieceType::Queen => 4,
                _ => 0,
            };
            ph += cnt * w;
        }
    }
    ph.clamp(0, 24)
}

#[inline]
fn sq_flip(sq: usize, c: Color) -> usize {
    if c == Color::White {
        sq
    } else {
        let r = sq / 8;
        let f = sq % 8;
        (7 - r) * 8 + f
    }
}

fn pawn_structure(b: &Board, w: &EvalWeights, us: Color) -> TaperedScore {
    let mut s = TaperedScore::default();
    let our = b.piece_bb[us.idx()][PieceType::Pawn.idx()];
    let their = b.piece_bb[us.flip().idx()][PieceType::Pawn.idx()];
    let all_pawns = our | their;
    let mut bb = our;
    while bb != 0 {
        let sq = bb.trailing_zeros() as usize;
        bb &= bb - 1;
        let f = sq % 8;
        let r = sq / 8;
        let file_mask = 0x0101010101010101u64 << f;
        if (our & file_mask).count_ones() > 1 {
            s.mg += w.doubled_mg;
            s.eg += w.doubled_eg;
        }
        let mut isolated = true;
        if f > 0 && (all_pawns & (0x0101010101010101u64 << (f - 1))) != 0 {
            isolated = false;
        }
        if f < 7 && (all_pawns & (0x0101010101010101u64 << (f + 1))) != 0 {
            isolated = false;
        }
        if isolated {
            s.mg += w.isolated_mg;
            s.eg += w.isolated_eg;
        }
        let mut passed = true;
        let dir: i32 = if us == Color::White { 1 } else { -1 };
        let mut rr = r as i32 + dir;
        while rr >= 0 && rr <= 7 {
            let mut df: i32 = -1;
            while df <= 1 {
                let nf = f as i32 + df;
                if nf >= 0 && nf <= 7 {
                    let idx = rr as usize * 8 + nf as usize;
                    if their & (1u64 << idx) != 0 {
                        passed = false;
                        break;
                    }
                }
                df += 1;
            }
            if !passed {
                break;
            }
            rr += dir;
        }
        if passed {
            s.mg += w.passed_mg;
            s.eg += w.passed_eg;
        }
    }
    s
}

/// Static evaluation from White's perspective before tempo; negamax applies side-to-move.
pub fn evaluate_white_pov(b: &Board, w: &EvalWeights) -> i32 {
    let mut acc = TaperedScore::default();
    for c in [Color::White, Color::Black] {
        let sign = if c == Color::White { 1 } else { -1 };
        for pt in [
            PieceType::Pawn,
            PieceType::Knight,
            PieceType::Bishop,
            PieceType::Rook,
            PieceType::Queen,
            PieceType::King,
        ] {
            let mut bb = b.piece_bb[c.idx()][pt.idx()];
            while bb != 0 {
                let sq = bb.trailing_zeros() as usize;
                bb &= bb - 1;
                let idx = sq_flip(sq, c);
                let pti = pt.idx();
                acc.mg += sign * (w.piece_mg[pti] + w.pst_mg[pti][idx]);
                acc.eg += sign * (w.piece_eg[pti] + w.pst_eg[pti][idx]);
            }
        }
        let ps = pawn_structure(b, w, c);
        if c == Color::White {
            acc.add_assign(ps);
        } else {
            acc.mg -= ps.mg;
            acc.eg -= ps.eg;
        }
    }
    let ph = phase_of(b);
    let egw = 24 - ph;

    let wb = b.piece_bb[Color::White.idx()][PieceType::Bishop.idx()].count_ones();
    if wb >= 2 {
        acc.mg += 22;
        acc.eg += 52;
    }
    let bbsh = b.piece_bb[Color::Black.idx()][PieceType::Bishop.idx()].count_ones();
    if bbsh >= 2 {
        acc.mg -= 22;
        acc.eg -= 52;
    }

    if ph + egw == 0 {
        acc.eg
    } else {
        (acc.mg * ph + acc.eg * egw) / 24
    }
}

/// Evaluation for side to move (negamax): positive is good for current side.
pub fn evaluate(b: &Board, w: &EvalWeights) -> i32 {
    const TEMPO: i32 = 14;
    let t = evaluate_white_pov(b, w);
    let s = if b.side_to_move() == Color::Black {
        -t
    } else {
        t
    };
    s + TEMPO
}

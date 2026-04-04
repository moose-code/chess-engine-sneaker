//! Bitboard board: FEN, Zobrist, make/unmake.

use crate::types::{
    Color, F_DOUBLE, F_EP, F_OO, F_OOO, Move, Piece, PieceType, Square,
};
use std::sync::OnceLock;

const NO_EP: u8 = 64;

#[inline]
fn bit(sq: Square) -> u64 {
    1u64 << sq.0
}

#[inline]
fn lsb(b: u64) -> u8 {
    b.trailing_zeros() as u8
}

#[inline]
fn ep_zob_index(ep: u8) -> usize {
    if ep == NO_EP {
        8
    } else {
        (ep & 7) as usize
    }
}

fn zobrist_init() -> ([[u64; 12]; 64], u64, [u64; 4], [u64; 9]) {
    let mut s: u64 = 0x243f_6a88_85a3_08d3;
    let mut next = || -> u64 {
        s ^= s >> 12;
        s ^= s << 25;
        s ^= s >> 27;
        s = s.wrapping_mul(0x2545_F491_4F6C_DD1D);
        s
    };
    let mut piece: [[u64; 12]; 64] = [[0; 12]; 64];
    for sq in 0..64 {
        for pt in 0..12 {
            piece[sq as usize][pt] = next();
        }
    }
    let side = next();
    let mut castle = [0u64; 4];
    for c in &mut castle {
        *c = next();
    }
    let mut epfile = [0u64; 9];
    for e in &mut epfile {
        *e = next();
    }
    (piece, side, castle, epfile)
}

fn zobrist_tables() -> &'static ([[u64; 12]; 64], u64, [u64; 4], [u64; 9]) {
    static Z: OnceLock<([[u64; 12]; 64], u64, [u64; 4], [u64; 9])> = OnceLock::new();
    Z.get_or_init(zobrist_init)
}

#[derive(Clone, Debug)]
pub struct Undo {
    pub m: Move,
    pub captured: Option<Piece>,
    pub ep: u8,
    pub castle: u8,
    pub halfmove: u8,
    pub fullmove: u16,
    pub hash: u64,
}

#[derive(Clone)]
pub struct Board {
    pub piece_bb: [[u64; 6]; 2],
    pub occupied: u64,
    pub white: u64,
    pub black: u64,
    pub stm: Color,
    pub castle: u8,
    pub ep: u8,
    pub halfmove: u8,
    pub fullmove: u16,
    hash: u64,
}

impl Default for Board {
    fn default() -> Self {
        Self::from_fen(START_FEN).expect("start fen")
    }
}

pub const START_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

impl Board {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_fen(fen: &str) -> Result<Self, ()> {
        let parts: Vec<&str> = fen.split_whitespace().collect();
        if parts.len() < 4 {
            return Err(());
        }
        let mut b = Board {
            piece_bb: [[0u64; 6]; 2],
            occupied: 0,
            white: 0,
            black: 0,
            stm: if parts[1] == "w" {
                Color::White
            } else if parts[1] == "b" {
                Color::Black
            } else {
                return Err(());
            },
            castle: 0,
            ep: NO_EP,
            halfmove: parts.get(4).and_then(|s| s.parse().ok()).unwrap_or(0),
            fullmove: parts.get(5).and_then(|s| s.parse().ok()).unwrap_or(1),
            hash: 0,
        };

        let mut rank = 7i32;
        let mut file = 0i32;
        for ch in parts[0].chars() {
            if ch == '/' {
                rank -= 1;
                file = 0;
                continue;
            }
            if let Some(d) = ch.to_digit(10) {
                file += d as i32;
                continue;
            }
            let sq = Square((rank * 8 + file) as u8);
            let (c, pt) = match ch {
                'P' => (Color::White, PieceType::Pawn),
                'N' => (Color::White, PieceType::Knight),
                'B' => (Color::White, PieceType::Bishop),
                'R' => (Color::White, PieceType::Rook),
                'Q' => (Color::White, PieceType::Queen),
                'K' => (Color::White, PieceType::King),
                'p' => (Color::Black, PieceType::Pawn),
                'n' => (Color::Black, PieceType::Knight),
                'b' => (Color::Black, PieceType::Bishop),
                'r' => (Color::Black, PieceType::Rook),
                'q' => (Color::Black, PieceType::Queen),
                'k' => (Color::Black, PieceType::King),
                _ => return Err(()),
            };
            b.add_piece(Piece { color: c, pt }, sq);
            file += 1;
        }

        for ch in parts[2].chars() {
            match ch {
                'K' => b.castle |= 1,
                'Q' => b.castle |= 2,
                'k' => b.castle |= 4,
                'q' => b.castle |= 8,
                '-' => {}
                _ => return Err(()),
            }
        }

        if parts[3] != "-" {
            b.ep = parts[3].parse::<Square>().map(|s| s.0).map_err(|_| ())?;
        }

        b.recompute_hash();
        Ok(b)
    }

    fn add_piece(&mut self, p: Piece, sq: Square) {
        let bb = bit(sq);
        self.piece_bb[p.color.idx()][p.pt.idx()] |= bb;
        self.occupied |= bb;
        match p.color {
            Color::White => self.white |= bb,
            Color::Black => self.black |= bb,
        }
    }

    fn remove_piece(&mut self, p: Piece, sq: Square) {
        let bb = !bit(sq);
        self.piece_bb[p.color.idx()][p.pt.idx()] &= bb;
        self.occupied &= bb;
        match p.color {
            Color::White => self.white &= bb,
            Color::Black => self.black &= bb,
        }
    }

    fn recompute_hash(&mut self) {
        let (pk, sidek, ck, ek) = zobrist_tables();
        let mut h = 0u64;
        for sq in 0u8..64 {
            if let Some(p) = self.piece_at(Square(sq)) {
                h ^= pk[sq as usize][p.idx()];
            }
        }
        if self.stm == Color::Black {
            h ^= *sidek;
        }
        if self.castle & 1 != 0 {
            h ^= ck[0];
        }
        if self.castle & 2 != 0 {
            h ^= ck[1];
        }
        if self.castle & 4 != 0 {
            h ^= ck[2];
        }
        if self.castle & 8 != 0 {
            h ^= ck[3];
        }
        h ^= ek[ep_zob_index(self.ep)];
        self.hash = h;
    }

    #[inline]
    pub fn hash(&self) -> u64 {
        self.hash
    }

    #[inline]
    pub fn side_to_move(&self) -> Color {
        self.stm
    }

    #[inline]
    pub fn ep_square_raw(&self) -> u8 {
        self.ep
    }

    #[inline]
    pub fn halfmove_clock(&self) -> u8 {
        self.halfmove
    }

    #[inline]
    pub fn fullmove_number(&self) -> u16 {
        self.fullmove
    }

    #[inline]
    pub fn castling_rights(&self) -> u8 {
        self.castle
    }

    pub fn piece_at(&self, sq: Square) -> Option<Piece> {
        let m = bit(sq);
        if self.occupied & m == 0 {
            return None;
        }
        for c in [Color::White, Color::Black] {
            for pt in [
                PieceType::Pawn,
                PieceType::Knight,
                PieceType::Bishop,
                PieceType::Rook,
                PieceType::Queen,
                PieceType::King,
            ] {
                if self.piece_bb[c.idx()][pt.idx()] & m != 0 {
                    return Some(Piece { color: c, pt });
                }
            }
        }
        None
    }

    pub fn king_sq(&self, c: Color) -> Square {
        let bb = self.piece_bb[c.idx()][PieceType::King.idx()];
        Square(lsb(bb))
    }

    pub fn in_check(&self) -> bool {
        let ksq = self.king_sq(self.stm);
        self.sq_attacked(ksq, self.stm.flip())
    }

    pub fn sq_attacked(&self, sq: Square, by: Color) -> bool {
        let occ = self.occupied;
        let them = by;
        let pr = sq.rank() as i8;
        let pf = sq.file() as i8;
        let pawn_dir: i8 = if them == Color::White { 1 } else { -1 };
        let pawn_from_r = pr - pawn_dir;
        if pawn_from_r >= 0 && pawn_from_r <= 7 {
            for df in [-1i8, 1] {
                let nf = pf + df;
                if nf < 0 || nf > 7 {
                    continue;
                }
                let from = Square::new(nf as u8, pawn_from_r as u8);
                if let Some(p) = self.piece_at(from) {
                    if p.color == them && p.pt == PieceType::Pawn {
                        return true;
                    }
                }
            }
        }

        let kn = crate::movegen::KNIGHT_ATTACKS[sq.0 as usize];
        if kn & self.piece_bb[them.idx()][PieceType::Knight.idx()] != 0 {
            return true;
        }

        let ki = crate::movegen::KING_ATTACKS[sq.0 as usize];
        if ki & self.piece_bb[them.idx()][PieceType::King.idx()] != 0 {
            return true;
        }

        let rook_like = self.piece_bb[them.idx()][PieceType::Rook.idx()]
            | self.piece_bb[them.idx()][PieceType::Queen.idx()];
        let bishop_like = self.piece_bb[them.idx()][PieceType::Bishop.idx()]
            | self.piece_bb[them.idx()][PieceType::Queen.idx()];

        for &(dr, df) in &[(1i8, 0i8), (-1, 0), (0, 1), (0, -1)] {
            let mut r = pr + dr;
            let mut f = pf + df;
            while r >= 0 && r <= 7 && f >= 0 && f <= 7 {
                let s = Square::new(f as u8, r as u8);
                let b = bit(s);
                if occ & b != 0 {
                    if rook_like & b != 0 {
                        return true;
                    }
                    break;
                }
                r += dr;
                f += df;
            }
        }

        for &(dr, df) in &[(1i8, 1i8), (1, -1), (-1, 1), (-1, -1)] {
            let mut r = pr + dr;
            let mut f = pf + df;
            while r >= 0 && r <= 7 && f >= 0 && f <= 7 {
                let s = Square::new(f as u8, r as u8);
                let b = bit(s);
                if occ & b != 0 {
                    if bishop_like & b != 0 {
                        return true;
                    }
                    break;
                }
                r += dr;
                f += df;
            }
        }

        false
    }

    fn update_castle_hash(&mut self, old: u8, new: u8) {
        let (_, _, ck, _) = zobrist_tables();
        let x = old ^ new;
        if x & 1 != 0 {
            self.hash ^= ck[0];
        }
        if x & 2 != 0 {
            self.hash ^= ck[1];
        }
        if x & 4 != 0 {
            self.hash ^= ck[2];
        }
        if x & 8 != 0 {
            self.hash ^= ck[3];
        }
    }

    pub fn make_move(&mut self, m: Move) -> Undo {
        let (pk, sidek, _, ek) = zobrist_tables();
        let from = m.from_sq();
        let to = m.to_sq();
        let moving = self.piece_at(from).expect("illegal make");
        debug_assert_eq!(moving.color, self.stm);

        let u = Undo {
            m,
            captured: None,
            ep: self.ep,
            castle: self.castle,
            halfmove: self.halfmove,
            fullmove: self.fullmove,
            hash: self.hash,
        };

        self.hash ^= ek[ep_zob_index(self.ep)];
        self.ep = NO_EP;

        let mut cap_sq = to;
        let cap_piece = self.piece_at(to);

        let captured_for_undo = match m.kind() {
            F_EP => {
                cap_sq = Square::new(to.file(), from.rank());
                self.piece_at(cap_sq)
            }
            F_OO | F_OOO => None,
            _ => cap_piece,
        };

        let mut u = u;
        u.captured = captured_for_undo;

        let reset_half = moving.pt == PieceType::Pawn || captured_for_undo.is_some();
        if reset_half {
            self.halfmove = 0;
        } else {
            self.halfmove = self.halfmove.saturating_add(1);
        }

        if self.stm == Color::Black {
            self.fullmove = self.fullmove.saturating_add(1);
        }

        let old_c = self.castle;

        match m.kind() {
            F_OO => {
                let (kf, kt, rf, rt) = if self.stm == Color::White {
                    (Square::new(4, 0), Square::new(6, 0), Square::new(7, 0), Square::new(5, 0))
                } else {
                    (Square::new(4, 7), Square::new(6, 7), Square::new(7, 7), Square::new(5, 7))
                };
                let rk = Piece {
                    color: self.stm,
                    pt: PieceType::Rook,
                };
                self.hash ^= pk[kf.0 as usize][moving.idx()];
                self.hash ^= pk[rf.0 as usize][rk.idx()];
                self.remove_piece(moving, kf);
                self.remove_piece(rk, rf);
                self.add_piece(moving, kt);
                self.add_piece(rk, rt);
                self.hash ^= pk[kt.0 as usize][moving.idx()];
                self.hash ^= pk[rt.0 as usize][rk.idx()];
            }
            F_OOO => {
                let (kf, kt, rf, rt) = if self.stm == Color::White {
                    (Square::new(4, 0), Square::new(2, 0), Square::new(0, 0), Square::new(3, 0))
                } else {
                    (Square::new(4, 7), Square::new(2, 7), Square::new(0, 7), Square::new(3, 7))
                };
                let rk = Piece {
                    color: self.stm,
                    pt: PieceType::Rook,
                };
                self.hash ^= pk[kf.0 as usize][moving.idx()];
                self.hash ^= pk[rf.0 as usize][rk.idx()];
                self.remove_piece(moving, kf);
                self.remove_piece(rk, rf);
                self.add_piece(moving, kt);
                self.add_piece(rk, rt);
                self.hash ^= pk[kt.0 as usize][moving.idx()];
                self.hash ^= pk[rt.0 as usize][rk.idx()];
            }
            _ => {
                self.hash ^= pk[from.0 as usize][moving.idx()];
                if let Some(cp) = captured_for_undo {
                    self.hash ^= pk[cap_sq.0 as usize][cp.idx()];
                    self.remove_piece(cp, cap_sq);
                }
                self.remove_piece(moving, from);
                let mut placed = moving;
                if m.is_promotion() {
                    placed.pt = PieceType::from_promo_code(m.promo_code()).unwrap_or(PieceType::Queen);
                }
                self.add_piece(placed, to);
                self.hash ^= pk[to.0 as usize][placed.idx()];
            }
        }

        match m.kind() {
            F_OO | F_OOO => {
                match self.stm {
                    Color::White => self.castle &= !3,
                    Color::Black => self.castle &= !12,
                }
            }
            _ => {
                if moving.pt == PieceType::King {
                    match self.stm {
                        Color::White => self.castle &= !3,
                        Color::Black => self.castle &= !12,
                    }
                }
                if moving.pt == PieceType::Rook {
                    match from.0 {
                        0 => self.castle &= !2,
                        7 => self.castle &= !1,
                        56 => self.castle &= !8,
                        63 => self.castle &= !4,
                        _ => {}
                    }
                }
                if let Some(cp) = u.captured {
                    if cp.pt == PieceType::Rook {
                        match to.0 {
                            0 => self.castle &= !2,
                            7 => self.castle &= !1,
                            56 => self.castle &= !8,
                            63 => self.castle &= !4,
                            _ => {}
                        }
                    }
                }
            }
        }

        if m.kind() == F_DOUBLE {
            let mid_r = (from.rank() as i8 + to.rank() as i8) / 2;
            self.ep = Square::new(from.file(), mid_r as u8).0;
        }

        self.update_castle_hash(old_c, self.castle);
        self.stm = self.stm.flip();
        self.hash ^= *sidek;
        self.hash ^= ek[ep_zob_index(self.ep)];

        u
    }

    pub fn unmake(&mut self, u: Undo) {
        let (pk, sidek, _, ek) = zobrist_tables();
        let m = u.m;
        let from = m.from_sq();
        let to = m.to_sq();

        self.hash ^= ek[ep_zob_index(self.ep)];
        self.hash ^= *sidek;
        self.stm = self.stm.flip();

        let old_c = self.castle;
        self.castle = u.castle;
        self.update_castle_hash(old_c, self.castle);

        self.halfmove = u.halfmove;
        self.fullmove = u.fullmove;
        self.ep = u.ep;

        match m.kind() {
            F_OO => {
                let (kf, kt, rf, rt) = if self.stm == Color::White {
                    (Square::new(4, 0), Square::new(6, 0), Square::new(7, 0), Square::new(5, 0))
                } else {
                    (Square::new(4, 7), Square::new(6, 7), Square::new(7, 7), Square::new(5, 7))
                };
                let rk = Piece {
                    color: self.stm,
                    pt: PieceType::Rook,
                };
                self.hash ^= pk[kt.0 as usize][Piece {
                    color: self.stm,
                    pt: PieceType::King,
                }
                .idx()];
                self.hash ^= pk[rt.0 as usize][rk.idx()];
                self.remove_piece(
                    Piece {
                        color: self.stm,
                        pt: PieceType::King,
                    },
                    kt,
                );
                self.remove_piece(rk, rt);
                self.add_piece(
                    Piece {
                        color: self.stm,
                        pt: PieceType::King,
                    },
                    kf,
                );
                self.add_piece(rk, rf);
                self.hash ^= pk[kf.0 as usize][Piece {
                    color: self.stm,
                    pt: PieceType::King,
                }
                .idx()];
                self.hash ^= pk[rf.0 as usize][rk.idx()];
            }
            F_OOO => {
                let (kf, kt, rf, rt) = if self.stm == Color::White {
                    (Square::new(4, 0), Square::new(2, 0), Square::new(0, 0), Square::new(3, 0))
                } else {
                    (Square::new(4, 7), Square::new(2, 7), Square::new(0, 7), Square::new(3, 7))
                };
                let rk = Piece {
                    color: self.stm,
                    pt: PieceType::Rook,
                };
                let kpc = Piece {
                    color: self.stm,
                    pt: PieceType::King,
                };
                self.hash ^= pk[kt.0 as usize][kpc.idx()];
                self.hash ^= pk[rt.0 as usize][rk.idx()];
                self.remove_piece(kpc, kt);
                self.remove_piece(rk, rt);
                self.add_piece(kpc, kf);
                self.add_piece(rk, rf);
                self.hash ^= pk[kf.0 as usize][kpc.idx()];
                self.hash ^= pk[rf.0 as usize][rk.idx()];
            }
            _ => {
                let cur = self.piece_at(to).expect("unmake to");
                let mut restored = cur;
                if m.is_promotion() {
                    restored.pt = PieceType::Pawn;
                }
                self.hash ^= pk[to.0 as usize][cur.idx()];
                self.remove_piece(cur, to);
                self.add_piece(restored, from);
                self.hash ^= pk[from.0 as usize][restored.idx()];

                if m.kind() == F_EP {
                    let cap_sq = Square::new(to.file(), from.rank());
                    let cap = Piece {
                        color: self.stm.flip(),
                        pt: PieceType::Pawn,
                    };
                    self.add_piece(cap, cap_sq);
                    self.hash ^= pk[cap_sq.0 as usize][cap.idx()];
                } else if let Some(cp) = u.captured {
                    self.add_piece(cp, to);
                    self.hash ^= pk[to.0 as usize][cp.idx()];
                }
            }
        }

        self.hash ^= ek[ep_zob_index(self.ep)];
        debug_assert_eq!(self.hash, u.hash);
    }
}


#[derive(Clone, Copy, Debug)]
pub struct NullMoveUndo {
    pub ep: u8,
}

impl Board {
    /// Side to move passes; illegal in chess but used for null-move pruning.
    pub fn make_null(&mut self) -> NullMoveUndo {
        let (_, sidek, _, ek) = zobrist_tables();
        let old_ep = self.ep;
        self.hash ^= ek[ep_zob_index(self.ep)];
        self.hash ^= ek[ep_zob_index(NO_EP)];
        self.ep = NO_EP;
        self.stm = self.stm.flip();
        self.hash ^= *sidek;
        NullMoveUndo { ep: old_ep }
    }

    pub fn unmake_null(&mut self, u: NullMoveUndo) {
        let (_, sidek, _, ek) = zobrist_tables();
        self.hash ^= *sidek;
        self.stm = self.stm.flip();
        self.hash ^= ek[ep_zob_index(NO_EP)];
        self.ep = u.ep;
        self.hash ^= ek[ep_zob_index(self.ep)];
    }
}


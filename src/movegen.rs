//! Pseudo-legal and legal move generation, perft.

use crate::board::Board;
use crate::types::{Color, F_DOUBLE, F_EP, F_NONE, F_OO, F_OOO, Move, PieceType, Square};

pub static KNIGHT_ATTACKS: [u64; 64] = compute_knight_attacks();
pub static KING_ATTACKS: [u64; 64] = compute_king_attacks();

const fn compute_knight_attacks() -> [u64; 64] {
    let mut t = [0u64; 64];
    let mut sq = 0usize;
    while sq < 64 {
        let r = (sq / 8) as i32;
        let f = (sq % 8) as i32;
        let mut bits = 0u64;
        let deltas = [
            (2, 1),
            (2, -1),
            (-2, 1),
            (-2, -1),
            (1, 2),
            (1, -2),
            (-1, 2),
            (-1, -2),
        ];
        let mut i = 0;
        while i < 8 {
            let (dr, df) = deltas[i];
            let nr = r + dr;
            let nf = f + df;
            if nr >= 0 && nr < 8 && nf >= 0 && nf < 8 {
                bits |= 1u64 << (nr * 8 + nf);
            }
            i += 1;
        }
        t[sq] = bits;
        sq += 1;
    }
    t
}

const fn compute_king_attacks() -> [u64; 64] {
    let mut t = [0u64; 64];
    let mut sq = 0usize;
    while sq < 64 {
        let r = (sq / 8) as i32;
        let f = (sq % 8) as i32;
        let mut bits = 0u64;
        let mut dr = -1;
        while dr <= 1 {
            let mut df = -1;
            while df <= 1 {
                if dr == 0 && df == 0 {
                    df += 1;
                    continue;
                }
                let nr = r + dr;
                let nf = f + df;
                if nr >= 0 && nr < 8 && nf >= 0 && nf < 8 {
                    bits |= 1u64 << (nr * 8 + nf);
                }
                df += 1;
            }
            dr += 1;
        }
        t[sq] = bits;
        sq += 1;
    }
    t
}

#[inline]
fn bit(sq: Square) -> u64 {
    1u64 << sq.0
}

fn ray_attacks(occ: u64, sq: Square, dirs: &[(i8, i8)]) -> u64 {
    let pr = sq.rank() as i8;
    let pf = sq.file() as i8;
    let mut out = 0u64;
    for &(dr, df) in dirs {
        let mut r = pr + dr;
        let mut f = pf + df;
        while r >= 0 && r <= 7 && f >= 0 && f <= 7 {
            let s = Square::new(f as u8, r as u8);
            let b = bit(s);
            out |= b;
            if occ & b != 0 {
                break;
            }
            r += dr;
            f += df;
        }
    }
    out
}

pub struct MoveGen;

impl MoveGen {
    pub fn gen_pseudo_legal(out: &mut Vec<Move>, b: &Board) {
        out.clear();
        let us = b.stm;
        let them = us.flip();
        let occ = b.occupied;
        let ours = if us == Color::White { b.white } else { b.black };
        let theirs = if us == Color::White { b.black } else { b.white };

        let pawn_push_dir: i8 = if us == Color::White { 1 } else { -1 };
        let start_rank: i8 = if us == Color::White { 1 } else { 6 };
        // Destination rank that triggers promotion (0-based).
        let promo_rank: i8 = if us == Color::White { 7 } else { 0 };

        let mut pawns = b.piece_bb[us.idx()][PieceType::Pawn.idx()];
        while pawns != 0 {
            let from = Square(pawns.trailing_zeros() as u8);
            pawns &= pawns - 1;
            let fr = from.rank() as i8;
            let ff = from.file() as i8;
            let tr = fr + pawn_push_dir;

            if tr >= 0 && tr <= 7 {
                let to = Square::new(from.file(), tr as u8);
                let tb = bit(to);
                if occ & tb == 0 {
                    if tr == promo_rank {
                        for pc in [1u8, 2, 3, 4] {
                            out.push(Move::new(from, to, pc, F_NONE));
                        }
                    } else {
                        out.push(Move::new(from, to, 0, F_NONE));
                        if fr == start_rank {
                            let tr2 = fr + 2 * pawn_push_dir;
                            let to2 = Square::new(from.file(), tr2 as u8);
                            if occ & bit(to2) == 0 {
                                out.push(Move::new(from, to2, 0, F_DOUBLE));
                            }
                        }
                    }
                }
            }

            for df in [-1i8, 1] {
                let nf = ff + df;
                if nf < 0 || nf > 7 {
                    continue;
                }
                if tr < 0 || tr > 7 {
                    continue;
                }
                let to = Square::new(nf as u8, tr as u8);
                let target = bit(to);
                if theirs & target != 0 {
                    if tr == promo_rank {
                        for pc in [1u8, 2, 3, 4] {
                            out.push(Move::new(from, to, pc, F_NONE));
                        }
                    } else {
                        out.push(Move::new(from, to, 0, F_NONE));
                    }
                } else if b.ep_square_raw() < 64 && to.0 == b.ep_square_raw() {
                    let cap_sq = Square::new(nf as u8, fr as u8);
                    if let Some(cp) = b.piece_at(cap_sq) {
                        if cp.color == them && cp.pt == PieceType::Pawn {
                            out.push(Move::new(from, to, 0, F_EP));
                        }
                    }
                }
            }
        }

        let mut kn = b.piece_bb[us.idx()][PieceType::Knight.idx()];
        while kn != 0 {
            let from = Square(kn.trailing_zeros() as u8);
            kn &= kn - 1;
            let mut attacks = KNIGHT_ATTACKS[from.0 as usize] & !ours;
            while attacks != 0 {
                let to = Square(attacks.trailing_zeros() as u8);
                attacks &= attacks - 1;
                out.push(Move::new(from, to, 0, F_NONE));
            }
        }

        let diag_dirs = [(1i8, 1i8), (1, -1), (-1, 1), (-1, -1)];
        let orth_dirs = [(1i8, 0i8), (-1, 0), (0, 1), (0, -1)];

        let mut bi = b.piece_bb[us.idx()][PieceType::Bishop.idx()];
        while bi != 0 {
            let from = Square(bi.trailing_zeros() as u8);
            bi &= bi - 1;
            let mut attacks = ray_attacks(occ, from, &diag_dirs) & !ours;
            while attacks != 0 {
                let to = Square(attacks.trailing_zeros() as u8);
                attacks &= attacks - 1;
                out.push(Move::new(from, to, 0, F_NONE));
            }
        }

        let mut rk = b.piece_bb[us.idx()][PieceType::Rook.idx()];
        while rk != 0 {
            let from = Square(rk.trailing_zeros() as u8);
            rk &= rk - 1;
            let mut attacks = ray_attacks(occ, from, &orth_dirs) & !ours;
            while attacks != 0 {
                let to = Square(attacks.trailing_zeros() as u8);
                attacks &= attacks - 1;
                out.push(Move::new(from, to, 0, F_NONE));
            }
        }

        let mut qu = b.piece_bb[us.idx()][PieceType::Queen.idx()];
        while qu != 0 {
            let from = Square(qu.trailing_zeros() as u8);
            qu &= qu - 1;
            let mut attacks = (ray_attacks(occ, from, &diag_dirs) | ray_attacks(occ, from, &orth_dirs)) & !ours;
            while attacks != 0 {
                let to = Square(attacks.trailing_zeros() as u8);
                attacks &= attacks - 1;
                out.push(Move::new(from, to, 0, F_NONE));
            }
        }

        let ksq = b.king_sq(us);
        let mut katt = KING_ATTACKS[ksq.0 as usize] & !ours;
        while katt != 0 {
            let to = Square(katt.trailing_zeros() as u8);
            katt &= katt - 1;
            out.push(Move::new(ksq, to, 0, F_NONE));
        }

        if !b.in_check() {
            if us == Color::White {
                if b.castling_rights() & 1 != 0 {
                    let e1 = Square::new(4, 0);
                    let f1 = Square::new(5, 0);
                    let g1 = Square::new(6, 0);
                    let h1 = Square::new(7, 0);
                    if b.piece_at(e1).map(|p| p.pt == PieceType::King).unwrap_or(false)
                        && b.piece_at(h1).map(|p| p.pt == PieceType::Rook).unwrap_or(false)
                        && occ & bit(f1) == 0
                        && occ & bit(g1) == 0
                        && !b.sq_attacked(e1, them)
                        && !b.sq_attacked(f1, them)
                        && !b.sq_attacked(g1, them)
                    {
                        out.push(Move::new(e1, g1, 0, F_OO));
                    }
                }
                if b.castling_rights() & 2 != 0 {
                    let e1 = Square::new(4, 0);
                    let d1 = Square::new(3, 0);
                    let c1 = Square::new(2, 0);
                    let a1 = Square::new(0, 0);
                    if b.piece_at(e1).map(|p| p.pt == PieceType::King).unwrap_or(false)
                        && b.piece_at(a1).map(|p| p.pt == PieceType::Rook).unwrap_or(false)
                        && occ & bit(d1) == 0
                        && occ & bit(c1) == 0
                        && occ & bit(Square::new(1, 0)) == 0
                        && !b.sq_attacked(e1, them)
                        && !b.sq_attacked(d1, them)
                        && !b.sq_attacked(c1, them)
                    {
                        out.push(Move::new(e1, c1, 0, F_OOO));
                    }
                }
            } else {
                if b.castling_rights() & 4 != 0 {
                    let e8 = Square::new(4, 7);
                    let f8 = Square::new(5, 7);
                    let g8 = Square::new(6, 7);
                    let h8 = Square::new(7, 7);
                    if b.piece_at(e8).map(|p| p.pt == PieceType::King).unwrap_or(false)
                        && b.piece_at(h8).map(|p| p.pt == PieceType::Rook).unwrap_or(false)
                        && occ & bit(f8) == 0
                        && occ & bit(g8) == 0
                        && !b.sq_attacked(e8, them)
                        && !b.sq_attacked(f8, them)
                        && !b.sq_attacked(g8, them)
                    {
                        out.push(Move::new(e8, g8, 0, F_OO));
                    }
                }
                if b.castling_rights() & 8 != 0 {
                    let e8 = Square::new(4, 7);
                    let d8 = Square::new(3, 7);
                    let c8 = Square::new(2, 7);
                    let a8 = Square::new(0, 7);
                    if b.piece_at(e8).map(|p| p.pt == PieceType::King).unwrap_or(false)
                        && b.piece_at(a8).map(|p| p.pt == PieceType::Rook).unwrap_or(false)
                        && occ & bit(d8) == 0
                        && occ & bit(c8) == 0
                        && occ & bit(Square::new(1, 7)) == 0
                        && !b.sq_attacked(e8, them)
                        && !b.sq_attacked(d8, them)
                        && !b.sq_attacked(c8, them)
                    {
                        out.push(Move::new(e8, c8, 0, F_OOO));
                    }
                }
            }
        }
    }


    fn is_noisy(b: &Board, m: Move) -> bool {
        if m.is_promotion() || m.is_en_passant() {
            return true;
        }
        b.piece_at(m.to_sq()).is_some()
    }

    pub fn gen_noisy_legal(out: &mut Vec<Move>, b: &mut Board) {
        let us = b.stm;
        Self::gen_pseudo_legal(out, b);
        let mut i = 0usize;
        while i < out.len() {
            let m = out[i];
            if !Self::is_noisy(b, m) {
                out.swap_remove(i);
                continue;
            }
            let u = b.make_move(m);
            let illegal = b.sq_attacked(b.king_sq(us), us.flip());
            b.unmake(u);
            if illegal {
                out.swap_remove(i);
            } else {
                i += 1;
            }
        }
    }

    pub fn gen_legal(out: &mut Vec<Move>, b: &mut Board) {
        let us = b.stm;
        Self::gen_pseudo_legal(out, b);
        let mut i = 0usize;
        while i < out.len() {
            let m = out[i];
            let u = b.make_move(m);
            let illegal = b.sq_attacked(b.king_sq(us), us.flip());
            b.unmake(u);
            if illegal {
                out.swap_remove(i);
            } else {
                i += 1;
            }
        }
    }

    /// Divide perft: returns total nodes at `depth` from `b`.
    pub fn perft(b: &mut Board, depth: u32) -> u64 {
        if depth == 0 {
            return 1;
        }
        let mut buf = Vec::with_capacity(256);
        Self::gen_legal(&mut buf, b);
        let mut sum = 0u64;
        if depth == 1 {
            return buf.len() as u64;
        }
        for &m in &buf {
            let u = b.make_move(m);
            sum += Self::perft(b, depth - 1);
            b.unmake(u);
        }
        sum
    }
}

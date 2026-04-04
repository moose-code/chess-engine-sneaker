//! Negamax search: alpha-beta, ID, quiescence, TT, killers, history, null, LMR, time.

use crate::board::{Board, NullMoveUndo};
use crate::eval::{evaluate, EvalWeights};
use crate::movegen::MoveGen;
use crate::types::{Color, Move, PieceType};
use std::time::{Duration, Instant};

const TT_EXACT: u8 = 0;
const TT_LOWER: u8 = 1;
const TT_UPPER: u8 = 2;
/// Mate scores are ~±30_000; TT stores them adjusted by ply so probes stay correct.
const MATE_BOUND: i32 = 27_000;

#[inline]
fn score_to_tt(s: i32, ply: usize) -> i32 {
    if s > MATE_BOUND {
        s + ply as i32
    } else if s < -MATE_BOUND {
        s - ply as i32
    } else {
        s
    }
}

#[inline]
fn score_from_tt(s: i32, ply: usize) -> i32 {
    if s > MATE_BOUND {
        s - ply as i32
    } else if s < -MATE_BOUND {
        s + ply as i32
    } else {
        s
    }
}

#[derive(Clone, Copy)]
struct TtEntry {
    key: u64,
    depth: u8,
    score: i32,
    flag: u8,
    best: u32,
}

impl Default for TtEntry {
    fn default() -> Self {
        Self {
            key: 0,
            depth: 0,
            score: 0,
            flag: TT_EXACT,
            best: 0,
        }
    }
}

pub struct Search {
    pub weights: EvalWeights,
    tt: Vec<TtEntry>,
    tt_mask: usize,
    history: [[i32; 64]; 64],
    deadline: Option<Instant>,
    nodes: u64,
    stop: bool,
}

impl Default for Search {
    fn default() -> Self {
        Self::new()
    }
}

impl Search {
    pub fn new() -> Self {
        let n = 1 << 20;
        Self {
            weights: EvalWeights::default(),
            tt: vec![TtEntry::default(); n],
            tt_mask: n - 1,
            history: [[0; 64]; 64],
            deadline: None,
            nodes: 0,
            stop: false,
        }
    }

    #[inline]
    pub fn nodes(&self) -> u64 {
        self.nodes
    }

    pub fn set_deadline(&mut self, d: Option<Instant>) {
        self.deadline = d;
        self.stop = false;
    }

    pub fn set_deadline_after(&mut self, dur: Duration) {
        self.deadline = Some(Instant::now() + dur);
        self.stop = false;
    }

    /// Resize transposition table; `mb` is the UCI Hash option (megabytes hint).
    /// Entry count is `(mb.max(1) << 16)` capped at `1 << 24`, rounded up to a power of two for indexing.
    pub fn set_tt_size(&mut self, mb: usize) {
        let mut entries = (mb.max(1) as usize) << 16;
        entries = entries.min(1 << 24);
        let n = entries.next_power_of_two();
        self.tt = vec![TtEntry::default(); n];
        self.tt_mask = n - 1;
    }

    #[inline]
    fn timed_out(&mut self) -> bool {
        if self.stop {
            return true;
        }
        if let Some(t) = self.deadline {
            if Instant::now() >= t {
                self.stop = true;
                return true;
            }
        }
        false
    }

    fn tt_load(&self, i: usize) -> TtEntry {
        self.tt[i & self.tt_mask]
    }

    fn tt_store(&mut self, i: usize, e: TtEntry) {
        let idx = i & self.tt_mask;
        let cur = self.tt[idx];
        if e.depth >= cur.depth || cur.key != e.key {
            self.tt[idx] = e;
        }
    }

    fn has_big_piece(b: &Board, c: Color) -> bool {
        let i = c.idx();
        b.piece_bb[i][PieceType::Knight.idx()]
            | b.piece_bb[i][PieceType::Bishop.idx()]
            | b.piece_bb[i][PieceType::Rook.idx()]
            | b.piece_bb[i][PieceType::Queen.idx()]
            != 0
    }

    /// Quiescence: noisy moves only when not in check; all evasions when in check.
    /// `qdepth` limits capture chains; check evasions ignore the cutoff.
    fn quiesce(
        &mut self,
        b: &mut Board,
        mut alpha: i32,
        beta: i32,
        ply: usize,
        qdepth: i32,
    ) -> i32 {
        if self.timed_out() {
            return 0;
        }
        self.nodes += 1;

        let in_check = b.in_check();
        let mut buf = Vec::with_capacity(96);

        if in_check {
            MoveGen::gen_legal(&mut buf, b);
            if buf.is_empty() {
                return -30_000 + ply as i32;
            }
        } else {
            if qdepth <= 0 {
                return evaluate(b, &self.weights);
            }
            let stand = evaluate(b, &self.weights);
            if stand >= beta {
                return beta;
            }
            if stand > alpha {
                alpha = stand;
            }
            MoveGen::gen_noisy_legal(&mut buf, b);
            if buf.is_empty() {
                return alpha;
            }
        }

        buf.sort_by_cached_key(|m: &Move| {
            let mut s = 0i32;
            if m.is_en_passant() {
                s = 100;
            } else if let Some(v) = b.piece_at(m.to_sq()) {
                s = 100 * piece_cap_val(v.pt)
                    - piece_cap_val(
                        b.piece_at(m.from_sq())
                            .map(|p| p.pt)
                            .unwrap_or(PieceType::Pawn),
                    );
            }
            -s
        });

        let next_q = if in_check { qdepth } else { qdepth - 1 };
        for m in buf {
            let u = b.make_move(m);
            let sc = -self.quiesce(b, -beta, -alpha, ply + 1, next_q);
            b.unmake(u);
            if self.stop {
                return 0;
            }
            if sc >= beta {
                return beta;
            }
            if sc > alpha {
                alpha = sc;
            }
        }
        alpha
    }

    fn negamax(
        &mut self,
        b: &mut Board,
        depth: i32,
        mut alpha: i32,
        mut beta: i32,
        ply: usize,
        allow_null: bool,
        killers: &mut [[Option<Move>; 2]],
    ) -> i32 {
        if self.timed_out() {
            return 0;
        }
        self.nodes += 1;

        if depth <= 0 {
            return self.quiesce(b, alpha, beta, ply, 12);
        }

        let alpha0 = alpha;
        let key = b.hash();
        let ti = key as usize;
        let te = self.tt_load(ti);
        let mut tt_move: Option<Move> = None;
        if te.key == key && te.depth >= depth as u8 {
            let sc = score_from_tt(te.score, ply);
            if te.best != 0 {
                tt_move = Some(Move(te.best));
            }
            match te.flag {
                TT_EXACT => return sc,
                TT_LOWER => {
                    if sc > alpha {
                        alpha = sc;
                    }
                }
                TT_UPPER => {
                    if sc < beta {
                        beta = sc;
                    }
                }
                _ => {}
            }
            if alpha >= beta {
                return sc;
            }
        }

        let in_check = b.in_check();
        if !in_check && allow_null && depth >= 3 && Self::has_big_piece(b, b.side_to_move()) {
            let n: NullMoveUndo = b.make_null();
            let v = -self.negamax(b, depth - 1 - 2, -beta, -alpha, ply + 1, false, killers);
            b.unmake_null(n);
            if self.stop {
                return 0;
            }
            if v >= beta {
                return beta;
            }
        }

        let mut buf = Vec::with_capacity(220);
        MoveGen::gen_legal(&mut buf, b);
        if buf.is_empty() {
            if in_check {
                return -30000 + ply as i32;
            }
            return 0;
        }

        let kp = killers[ply.min(63)];
        buf.sort_by_cached_key(|m: &Move| {
            let mut score = 0i32;
            if Some(*m) == tt_move {
                score -= 1_000_000;
            } else if kp[0] == Some(*m) {
                score -= 500_000;
            } else if kp[1] == Some(*m) {
                score -= 400_000;
            } else {
                let f = m.from_sq().0 as usize;
                let t = m.to_sq().0 as usize;
                score -= self.history[f][t];
                if let Some(v) = b.piece_at(m.to_sq()) {
                    score -= 100 * piece_cap_val(v.pt);
                }
            }
            score
        });

        let mut best: Option<Move> = None;
        let mut best_sc = i32::MIN / 2;
        let mut move_no = 0usize;

        for m in buf {
            move_no += 1;
            let mut full_depth = depth - 1;
            let is_cap = b.piece_at(m.to_sq()).is_some() || m.is_en_passant();
            let gives_check = {
                let u = b.make_move(m);
                let c = b.in_check();
                b.unmake(u);
                c
            };
            if gives_check {
                full_depth += 1;
            }
            let mut search_depth = full_depth;
            let can_lmr = move_no > 3
                && depth >= 3
                && search_depth >= 1
                && !in_check
                && !is_cap
                && !gives_check
                && m.promo_code() == 0
                && !m.is_castle_oo()
                && !m.is_castle_ooo();
            if can_lmr {
                let r = 1 + (depth / 8).min(3);
                search_depth -= r;
                if search_depth < 0 {
                    search_depth = 0;
                }
            }

            let u = b.make_move(m);
            // Principal Variation Search: full window for first move, null window for the rest.
            let mut sc = if move_no == 1 {
                -self.negamax(b, search_depth, -beta, -alpha, ply + 1, true, killers)
            } else {
                let mut v =
                    -self.negamax(b, search_depth, -alpha - 1, -alpha, ply + 1, true, killers);
                if !self.stop && v > alpha && v < beta {
                    v = -self.negamax(b, search_depth, -beta, -alpha, ply + 1, true, killers);
                }
                v
            };
            // If a reduced move unexpectedly improves alpha, re-search at full depth.
            if can_lmr && !self.stop && sc > alpha {
                sc = -self.negamax(b, full_depth, -beta, -alpha, ply + 1, true, killers);
            }
            b.unmake(u);

            if self.stop {
                return 0;
            }

            if sc > best_sc {
                best_sc = sc;
                best = Some(m);
            }
            if sc > alpha {
                alpha = sc;
            }
            if alpha >= beta {
                if !is_cap {
                    let pi = ply.min(63);
                    killers[pi][1] = killers[pi][0];
                    killers[pi][0] = Some(m);
                    let f = m.from_sq().0 as usize;
                    let t = m.to_sq().0 as usize;
                    self.history[f][t] += (depth * depth) as i32;
                }
                break;
            }
        }

        let flag = if best_sc >= beta {
            TT_LOWER
        } else if best_sc <= alpha0 {
            TT_UPPER
        } else {
            TT_EXACT
        };
        self.tt_store(
            ti,
            TtEntry {
                key,
                depth: depth as u8,
                score: score_to_tt(best_sc, ply),
                flag,
                best: best.map(|m| m.0).unwrap_or(0),
            },
        );

        best_sc
    }

    pub fn best_move(&mut self, b: &mut Board, max_depth: i32) -> Option<(Move, i32)> {
        self.nodes = 0;
        self.stop = false;
        let mut global_best: Option<Move> = None;
        let mut global_sc = 0i32;
        let mut prev_sc = 0i32;
        let mut killers = [[None, None]; 64];
        const ASP: i32 = 36;
        const WIDE: i32 = 50_000;

        for d in 1..=max_depth {
            if self.timed_out() {
                break;
            }
            let mut buf = Vec::with_capacity(220);
            MoveGen::gen_legal(&mut buf, b);
            if buf.is_empty() {
                return None;
            }
            if let Some(mb) = global_best {
                if let Some(i) = buf.iter().position(|x| *x == mb) {
                    buf.swap(0, i);
                }
            }

            let mut local_best: Option<Move> = None;
            let mut local_sc = i32::MIN / 2;
            let mut alpha = if d >= 4 {
                prev_sc.saturating_sub(ASP)
            } else {
                -WIDE
            };
            let beta = if d >= 4 {
                prev_sc.saturating_add(ASP)
            } else {
                WIDE
            };
            let mut need_wide = false;

            for m in buf.iter().copied() {
                let u = b.make_move(m);
                let sc = -self.negamax(b, d - 1, -beta, -alpha, 1, true, &mut killers);
                b.unmake(u);
                if self.stop {
                    break;
                }
                if sc > local_sc {
                    local_sc = sc;
                    local_best = Some(m);
                }
                if sc > alpha {
                    alpha = sc;
                }
                if alpha >= beta {
                    need_wide = true;
                    break;
                }
            }

            if need_wide && d >= 4 && !self.stop {
                let mut wide_a = -WIDE;
                local_sc = i32::MIN / 2;
                local_best = None;
                for m in buf.iter().copied() {
                    let u = b.make_move(m);
                    let sc = -self.negamax(b, d - 1, -WIDE, -wide_a, 1, true, &mut killers);
                    b.unmake(u);
                    if self.stop {
                        break;
                    }
                    if sc > local_sc {
                        local_sc = sc;
                        local_best = Some(m);
                    }
                    if sc > wide_a {
                        wide_a = sc;
                    }
                }
            }

            if !self.stop {
                global_best = local_best;
                global_sc = local_sc;
                prev_sc = local_sc;
            }

            for row in &mut self.history {
                for h in row.iter_mut() {
                    *h = (*h * 7) / 8;
                }
            }

            if self.stop {
                break;
            }
        }
        global_best.map(|m| (m, global_sc))
    }
}

fn piece_cap_val(pt: PieceType) -> i32 {
    match pt {
        PieceType::Pawn => 1,
        PieceType::Knight | PieceType::Bishop => 3,
        PieceType::Rook => 5,
        PieceType::Queen => 9,
        PieceType::King => 0,
    }
}

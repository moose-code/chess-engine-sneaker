//! Lightweight NNUE-style inference (single hidden layer).
//! Format (whitespace-separated i32):
//!   hidden_size
//!   <hidden_bias x H>
//!   <output_weight x H>
//!   output_bias
//!   <feature_weights x (768*H)>  where feature index is piece(12)*square(64) + sq
//!
//! Output is centipawns from White perspective.

use crate::board::Board;
use crate::types::{Color, PieceType};
use std::fs;

#[derive(Clone)]
pub struct NnueModel {
    pub hidden: usize,
    pub hidden_bias: Vec<i32>,
    pub output_w: Vec<i32>,
    pub output_b: i32,
    pub feat_w: Vec<i32>,
}

impl NnueModel {
    pub fn load(path: &str) -> Option<Self> {
        let txt = fs::read_to_string(path).ok()?;
        let vals: Vec<i32> = txt
            .split_whitespace()
            .filter_map(|t| t.parse::<i32>().ok())
            .collect();
        if vals.len() < 4 {
            return None;
        }
        let h = vals[0].max(1) as usize;
        let need = 1 + h + h + 1 + 768 * h;
        if vals.len() < need {
            return None;
        }
        let mut i = 1usize;
        let hidden_bias = vals[i..i + h].to_vec();
        i += h;
        let output_w = vals[i..i + h].to_vec();
        i += h;
        let output_b = vals[i];
        i += 1;
        let feat_w = vals[i..i + 768 * h].to_vec();
        Some(Self {
            hidden: h,
            hidden_bias,
            output_w,
            output_b,
            feat_w,
        })
    }

    #[inline]
    fn feat_index(color: Color, pt: PieceType, sq: usize) -> usize {
        (color as usize) * 6 * 64 + pt.idx() * 64 + sq
    }

    pub fn eval_white_pov(&self, b: &Board) -> i32 {
        let h = self.hidden;
        let mut acc = self.hidden_bias.clone();
        for c in [Color::White, Color::Black] {
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
                    let fidx = Self::feat_index(c, pt, sq);
                    let base = fidx * h;
                    for j in 0..h {
                        acc[j] += self.feat_w[base + j];
                    }
                }
            }
        }
        let mut out = self.output_b;
        for (j, v) in acc.iter().enumerate() {
            let r = (*v).max(0);
            out += (self.output_w[j] * r) / 256;
        }
        out / 128
    }
}

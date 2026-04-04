//! Core types: colors, pieces, squares, packed moves, UCI.

use core::fmt;
use core::str::FromStr;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Color {
    White = 0,
    Black = 1,
}

impl Color {
    #[inline]
    pub const fn flip(self) -> Self {
        match self {
            Self::White => Self::Black,
            Self::Black => Self::White,
        }
    }

    #[inline]
    pub const fn idx(self) -> usize {
        self as usize
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum PieceType {
    Pawn = 0,
    Knight = 1,
    Bishop = 2,
    Rook = 3,
    Queen = 4,
    King = 5,
}

impl PieceType {
    #[inline]
    pub const fn idx(self) -> usize {
        self as usize
    }

    #[inline]
    pub const fn from_promo_code(c: u8) -> Option<Self> {
        match c {
            1 => Some(Self::Knight),
            2 => Some(Self::Bishop),
            3 => Some(Self::Rook),
            4 => Some(Self::Queen),
            _ => None,
        }
    }

    #[inline]
    pub const fn promo_code(self) -> u8 {
        match self {
            Self::Knight => 1,
            Self::Bishop => 2,
            Self::Rook => 3,
            Self::Queen => 4,
            _ => 0,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct Piece {
    pub color: Color,
    pub pt: PieceType,
}

impl Piece {
    #[inline]
    pub const fn idx(self) -> usize {
        self.color.idx() * 6 + self.pt.idx()
    }
}

/// a1 = 0, h8 = 63 (file a–h, rank 1–8).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Square(pub u8);

impl Square {
    pub const A1: Self = Self(0);
    pub const INVALID: Self = Self(64);

    #[inline]
    pub const fn new(file: u8, rank: u8) -> Self {
        Self(rank * 8 + file)
    }

    #[inline]
    pub const fn rank(self) -> u8 {
        self.0 >> 3
    }

    #[inline]
    pub const fn file(self) -> u8 {
        self.0 & 7
    }

    #[inline]
    pub const fn is_valid(self) -> bool {
        self.0 < 64
    }
}

impl FromStr for Square {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let b = s.as_bytes();
        if b.len() != 2 {
            return Err(());
        }
        let file = b[0].wrapping_sub(b'a');
        let rank = b[1].wrapping_sub(b'1');
        if file >= 8 || rank >= 8 {
            return Err(());
        }
        Ok(Self::new(file, rank))
    }
}

impl fmt::Display for Square {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.is_valid() {
            return write!(f, "-");
        }
        let file = (b'a' + self.file()) as char;
        let rank = (b'1' + self.rank()) as char;
        write!(f, "{}{}", file, rank)
    }
}

pub type MoveKind = u32;

pub const F_NONE: MoveKind = 0;
pub const F_EP: MoveKind = 1;
pub const F_OO: MoveKind = 2;
pub const F_OOO: MoveKind = 3;
pub const F_DOUBLE: MoveKind = 4;

/// MV_OOO uses F_OOO (not F_OO twice).
pub const MV_OO: Move = Move::new_raw(Square(4), Square(6), 0, F_OO);
pub const MV_OOO: Move = Move::new_raw(Square(4), Square(2), 0, F_OOO);

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Move(pub u32);

impl Move {
    const FROM_MASK: u32 = 0x3F;
    const TO_SHIFT: u32 = 6;
    const PROMO_SHIFT: u32 = 12;
    const KIND_SHIFT: u32 = 15;

    #[inline]
    pub const fn new(from: Square, to: Square, promo_code: u8, kind: MoveKind) -> Self {
        let mut w = from.0 as u32;
        w |= (to.0 as u32) << Self::TO_SHIFT;
        w |= (promo_code as u32 & 7) << Self::PROMO_SHIFT;
        w |= (kind & 0x1F) << Self::KIND_SHIFT;
        Self(w)
    }

    #[inline]
    pub const fn new_raw(from: Square, to: Square, promo_code: u8, kind: MoveKind) -> Self {
        Self::new(from, to, promo_code, kind)
    }

    #[inline]
    pub const fn from_sq(self) -> Square {
        Square((self.0 & Self::FROM_MASK) as u8)
    }

    #[inline]
    pub const fn to_sq(self) -> Square {
        Square(((self.0 >> Self::TO_SHIFT) & Self::FROM_MASK) as u8)
    }

    #[inline]
    pub const fn promo_code(self) -> u8 {
        ((self.0 >> Self::PROMO_SHIFT) & 7) as u8
    }

    #[inline]
    pub const fn kind(self) -> MoveKind {
        (self.0 >> Self::KIND_SHIFT) & 0x1F
    }

    #[inline]
    pub const fn is_promotion(self) -> bool {
        self.promo_code() != 0
    }

    #[inline]
    pub const fn is_en_passant(self) -> bool {
        self.kind() == F_EP
    }

    #[inline]
    pub const fn is_castle_oo(self) -> bool {
        self.kind() == F_OO
    }

    #[inline]
    pub const fn is_castle_ooo(self) -> bool {
        self.kind() == F_OOO
    }

    #[inline]
    pub const fn is_double_push(self) -> bool {
        self.kind() == F_DOUBLE
    }

    pub fn to_uci(self) -> String {
        let mut s = String::with_capacity(5);
        s.push_str(&self.from_sq().to_string());
        s.push_str(&self.to_sq().to_string());
        if let Some(pt) = PieceType::from_promo_code(self.promo_code()) {
            let c = match pt {
                PieceType::Knight => 'n',
                PieceType::Bishop => 'b',
                PieceType::Rook => 'r',
                PieceType::Queen => 'q',
                _ => 'q',
            };
            s.push(c);
        }
        s
    }

    pub fn from_uci(s: &str) -> Result<Self, ()> {
        let b = s.as_bytes();
        if b.len() != 4 && b.len() != 5 {
            return Err(());
        }
        let from: Square = core::str::from_utf8(&b[0..2]).ok().ok_or(())?.parse()?;
        let to: Square = core::str::from_utf8(&b[2..4]).ok().ok_or(())?.parse()?;
        let promo = if b.len() == 5 {
            match b[4] {
                b'n' | b'N' => 1,
                b'b' | b'B' => 2,
                b'r' | b'R' => 3,
                b'q' | b'Q' => 4,
                _ => return Err(()),
            }
        } else {
            0
        };
        Ok(Self::new(from, to, promo, F_NONE))
    }
}

impl fmt::Debug for Move {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Move({})", self.to_uci())
    }
}

impl fmt::Display for Move {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.to_uci().fmt(f)
    }
}

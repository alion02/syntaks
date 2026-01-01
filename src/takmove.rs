use crate::core::*;
use std::num::NonZeroU16;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct Move {
    raw: NonZeroU16,
}

impl Move {
    const SQUARE_BITS: usize = 6;
    const PATTERN_BITS: usize = 6;
    const FLAG_BITS: usize = 2;

    const SQUARE_SHIFT: usize = 0;
    const PATTERN_SHIFT: usize = Self::SQUARE_SHIFT + Self::SQUARE_BITS;
    const FLAG_SHIFT: usize = Self::PATTERN_SHIFT + Self::PATTERN_BITS;

    const SQUARE_MASK: u16 = (1 << Self::SQUARE_BITS) - 1;
    const PATTERN_MASK: u16 = (1 << Self::PATTERN_BITS) - 1;
    const FLAG_MASK: u16 = (1 << Self::FLAG_BITS) - 1;

    #[must_use]
    pub const fn placement(pt: PieceType, dst: Square) -> Self {
        let mut raw = 0;

        raw |= (pt.raw() as u16 + 1) << Self::FLAG_SHIFT;
        raw |= dst.raw() as u16;

        Self {
            raw: NonZeroU16::new(raw).unwrap(),
        }
    }

    #[must_use]
    pub const fn spread(src: Square, dir: Direction, pattern: u16) -> Self {
        assert!(pattern != 0);
        assert!((pattern & !Self::PATTERN_MASK) == 0);

        let mut raw = 0;

        raw |= (dir as u16) << Self::FLAG_SHIFT;
        raw |= pattern << Self::PATTERN_SHIFT;
        raw |= (src.raw() as u16) << Self::SQUARE_SHIFT;

        Self {
            raw: NonZeroU16::new(raw).unwrap(),
        }
    }

    #[must_use]
    pub const fn sq(self) -> Square {
        Square::from_raw((((self.raw.get()) >> Self::SQUARE_SHIFT) & Self::SQUARE_MASK) as u8)
            .unwrap()
    }

    #[must_use]
    pub const fn pattern(self) -> u16 {
        (self.raw.get() >> Self::PATTERN_SHIFT) & Self::PATTERN_MASK
    }

    #[must_use]
    pub const fn is_spread(self) -> bool {
        self.pattern() != 0
    }

    #[must_use]
    pub const fn pt(self) -> PieceType {
        assert!(!self.is_spread());
        PieceType::from_raw((((self.raw.get() >> Self::FLAG_SHIFT) & Self::FLAG_MASK) - 1) as u8)
            .unwrap()
    }

    #[must_use]
    pub const fn dir(self) -> Direction {
        assert!(self.is_spread());
        Direction::from_raw(((self.raw.get() >> Self::FLAG_SHIFT) & Self::FLAG_MASK) as u8).unwrap()
    }
}

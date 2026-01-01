use crate::bitboard::Bitboard;
use crate::core::*;
use crate::takmove::Move;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct Stacks {
    players: [u64; Square::COUNT],
    heights: [u8; Square::COUNT],
    tops: [Option<PieceType>; Square::COUNT],
}

impl Stacks {
    pub fn is_empty(&self, sq: Square) -> bool {
        self.tops[sq.idx()].is_none()
    }

    pub fn top(&self, sq: Square) -> Option<PieceType> {
        self.tops[sq.idx()]
    }

    pub fn height(&self, sq: Square) -> u8 {
        self.heights[sq.idx()]
    }

    fn push(&mut self, sq: Square, pt: PieceType, player: Player) {
        self.players[sq.idx()] |= (player.raw() as u64) << self.heights[sq.idx()];
        self.heights[sq.idx()] += 1;
        self.tops[sq.idx()] = Some(pt);
    }

    fn take(&mut self, sq: Square, count: u8) {
        debug_assert!(count <= self.heights[sq.idx()]);

        self.heights[sq.idx()] -= count;

        if self.heights[sq.idx()] == 0 {
            self.tops[sq.idx()] = None;
        } else {
            self.tops[sq.idx()] = Some(PieceType::Flat);
        }
    }

    pub fn iter(&self, sq: Square) -> StackIterator {
        StackIterator {
            players: self.players[sq.idx()],
            height: self.heights[sq.idx()],
            idx: 0,
        }
    }
}

impl Default for Stacks {
    fn default() -> Self {
        Self {
            players: [u64::default(); Square::COUNT],
            heights: [u8::default(); Square::COUNT],
            tops: [None; Square::COUNT],
        }
    }
}

pub struct StackIterator {
    players: u64,
    height: u8,
    idx: u8,
}

impl Iterator for StackIterator {
    type Item = Player;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx == self.height {
            return None;
        }

        let player = Player::from_raw(((self.players >> self.idx) & 0x1) as u8).unwrap();
        self.idx += 1;
        Some(player)
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct Position {
    stacks: Stacks,
    players: [Bitboard; Player::COUNT],
    pieces: [Bitboard; PieceType::COUNT],
    flats_in_hand: [u8; Player::COUNT],
    caps_in_hand: [u8; Player::COUNT],
    stm: Player,
    ply: u16,
}

impl Position {
    pub fn startpos() -> Self {
        Self {
            stacks: Stacks::default(),
            players: [Bitboard::empty(); Player::COUNT],
            pieces: [Bitboard::empty(); PieceType::COUNT],
            flats_in_hand: [30; Player::COUNT],
            caps_in_hand: [1; Player::COUNT],
            stm: Player::P1,
            ply: 0,
        }
    }

    pub fn stm(&self) -> Player {
        self.stm
    }

    pub fn stacks(&self) -> &Stacks {
        &self.stacks
    }

    pub fn ply(&self) -> u16 {
        self.ply
    }

    fn drop_piece(&mut self, pt: PieceType, sq: Square, player: Player) {
        self.stacks.push(sq, pt, player);

        let bb = sq.bb();
        self.players[player.idx()] |= bb;
        self.pieces[pt.idx()] |= bb;
    }

    pub fn apply_move(&self, mv: Move) -> Self {
        let mut new_pos = *self;

        if mv.is_spread() {
            debug_assert_ne!(self.stacks.top(mv.sq()), None);
            debug_assert!(self.ply() >= 2);

            todo!();
        } else {
            debug_assert_eq!(self.stacks.top(mv.sq()), None);
            debug_assert!(self.ply() >= 2 || mv.pt() == PieceType::Flat);

            let dropped_player = if self.ply() < 2 {
                self.stm().flip()
            } else {
                self.stm()
            };

            new_pos.drop_piece(mv.pt(), mv.sq(), dropped_player);

            match mv.pt() {
                PieceType::Capstone => new_pos.caps_in_hand[self.stm().idx()] -= 1,
                _ => new_pos.flats_in_hand[self.stm().idx()] -= 1,
            }
        }

        new_pos.stm = new_pos.stm.flip();
        new_pos.ply += 1;

        new_pos
    }

    pub fn tps(&self) -> String {
        let mut tps = String::with_capacity(21);

        for rank in (0..6).rev() {
            let mut groups = Vec::new();

            let mut file = 0;
            while file < 6 {
                let sq = Square::from_file_rank(file, rank).unwrap();

                if self.stacks.is_empty(sq) {
                    let mut empty = 1;

                    while file < 5
                        && self
                            .stacks
                            .is_empty(Square::from_file_rank(file + 1, rank).unwrap())
                    {
                        file += 1;
                        empty += 1;
                    }

                    if empty > 1 {
                        groups.push(format!("x{}", empty));
                    } else {
                        groups.push("x".to_owned());
                    }
                } else {
                    let mut stack_str = String::with_capacity(self.stacks.height(sq) as usize + 1);

                    for player in self.stacks.iter(sq) {
                        match player {
                            Player::P1 => stack_str.push('1'),
                            Player::P2 => stack_str.push('2'),
                        }
                    }

                    match self.stacks.top(sq).unwrap() {
                        PieceType::Flat => {}
                        PieceType::Wall => stack_str.push('S'),
                        PieceType::Capstone => stack_str.push('C'),
                    }

                    groups.push(stack_str);
                }

                file += 1;
            }

            tps.push_str(&groups.join(","));

            if rank > 0 {
                tps.push('/');
            }
        }

        match self.stm() {
            Player::P1 => tps.push_str(" 1"),
            Player::P2 => tps.push_str(" 2"),
        }

        tps.push_str(&format!(" {}", self.ply() / 2 + 1));

        tps
    }
}

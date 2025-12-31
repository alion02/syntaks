use crate::bitboard::Bitboard;
use crate::core::*;
use crate::takmove::Move;

#[derive(Copy, Clone, Eq, PartialEq, Debug, Default)]
pub struct Stack {
    players: u64,
    height: u8,
    top: Option<PieceType>,
}

impl Stack {
    pub fn is_empty(&self) -> bool {
        self.top.is_none()
    }

    pub fn top(&self) -> Option<PieceType> {
        self.top
    }

    pub fn height(&self) -> u8 {
        self.height
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

impl IntoIterator for Stack {
    type Item = Player;
    type IntoIter = StackIterator;

    fn into_iter(self) -> Self::IntoIter {
        StackIterator {
            players: self.players,
            height: self.height,
            idx: 0,
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct Position {
    stacks: [Stack; Square::COUNT],
    players: [Bitboard; Player::COUNT],
    flats: Bitboard,
    walls: Bitboard,
    caps: Bitboard,
    stm: Player,
    flats_in_hand: [u8; Player::COUNT],
    caps_in_hand: [u8; Player::COUNT],
    ply: u16,
}

pub const POS_SIZE: usize = std::mem::size_of::<Position>();

impl Position {
    pub fn startpos() -> Self {
        Self {
            stacks: [Stack::default(); Square::COUNT],
            players: [Bitboard::empty(); Player::COUNT],
            flats: Bitboard::empty(),
            walls: Bitboard::empty(),
            caps: Bitboard::empty(),
            stm: Player::P1,
            flats_in_hand: [30; Player::COUNT],
            caps_in_hand: [1; Player::COUNT],
            ply: 0,
        }
    }

    pub fn stm(&self) -> Player {
        self.stm
    }

    pub fn stack_on(&self, sq: Square) -> &Stack {
        &self.stacks[sq.idx()]
    }

    pub fn ply(&self) -> u16 {
        self.ply
    }

    pub fn apply_move(&self, mv: Move) -> Self {
        let mut new_pos = *self;

        //TODO

        new_pos
    }

    pub fn tps(&self) -> String {
        let mut tps = String::with_capacity(21);

        for rank in (0..6).rev() {
            let mut groups = Vec::new();

            let mut file = 0;
            while file < 6 {
                let sq = Square::from_file_rank(file, rank).unwrap();
                let stack = self.stack_on(sq);

                if stack.is_empty() {
                    let mut empty = 1;

                    while file < 5
                        && self
                            .stack_on(Square::from_file_rank(file + 1, rank).unwrap())
                            .is_empty()
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
                    let mut stack_str = String::with_capacity(stack.height() as usize + 1);

                    for player in stack.into_iter() {
                        match player {
                            Player::P1 => stack_str.push('1'),
                            Player::P2 => stack_str.push('2'),
                        }
                    }

                    match stack.top().unwrap() {
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

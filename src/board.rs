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

    pub fn top_player(&self, sq: Square) -> Option<Player> {
        if self.tops[sq.idx()].is_none() {
            None
        } else {
            Some(
                Player::from_raw((self.players[sq.idx()] >> (self.heights[sq.idx()] - 1)) as u8)
                    .unwrap(),
            )
        }
    }

    pub fn height(&self, sq: Square) -> u8 {
        self.heights[sq.idx()]
    }

    fn push(&mut self, sq: Square, pt: PieceType, player: Player) {
        debug_assert_ne!(self.tops[sq.idx()], Some(PieceType::Capstone));

        self.players[sq.idx()] |= (player.raw() as u64) << self.heights[sq.idx()];
        self.heights[sq.idx()] += 1;
        self.tops[sq.idx()] = Some(pt);
    }

    fn take(&mut self, sq: Square, count: u8) -> (u8, PieceType, Option<Player>) {
        debug_assert!(count <= self.heights[sq.idx()]);
        debug_assert!(count > 0);
        debug_assert!(count <= 6);

        let players =
            (self.players[sq.idx()] >> (self.heights[sq.idx()] - count)) & ((1 << count) - 1);
        let top = self.tops[sq.idx()].unwrap();

        self.heights[sq.idx()] -= count;

        if self.heights[sq.idx()] == 0 {
            self.tops[sq.idx()] = None;
            (players as u8, top, None)
        } else {
            self.tops[sq.idx()] = Some(PieceType::Flat);
            let new_top_player =
                Player::from_raw(((self.players[sq.idx()] >> self.heights[sq.idx()]) & 0x1) as u8)
                    .unwrap();
            (players as u8, top, Some(new_top_player))
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
    #[must_use]
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

    #[must_use]
    pub fn stm(&self) -> Player {
        self.stm
    }

    #[must_use]
    pub fn stacks(&self) -> &Stacks {
        &self.stacks
    }

    #[must_use]
    pub fn ply(&self) -> u16 {
        self.ply
    }

    #[must_use]
    pub fn apply_move(&self, mv: Move) -> Self {
        let mut new_pos = *self;

        if mv.is_spread() {
            debug_assert_ne!(self.stacks.top(mv.sq()), None);
            debug_assert_eq!(self.stacks.top_player(mv.sq()), Some(self.stm()));
            debug_assert!(self.ply() >= 2);

            let pattern = mv.pattern();
            let dir = mv.dir();

            let dropped = pattern.trailing_zeros();
            let taken = 6 - dropped;

            let mut pattern = pattern >> dropped;
            let (mut players, top, new_top_player) = new_pos.stacks.take(mv.sq(), taken as u8);

            let mut new_flats_bb = Bitboard::empty();
            let mut new_player_bbs = [Bitboard::empty(); Player::COUNT];

            if let Some(new_top_player) = new_top_player {
                new_player_bbs[new_top_player.idx()].set_sq(mv.sq());
            } else {
                new_pos.players[self.stm().idx()].toggle_sq(mv.sq());
            }

            if top != PieceType::Flat {
                new_pos.pieces[top.idx()].clear_sq(mv.sq());
            }

            let mut sq = mv.sq().shift(dir).unwrap();

            for idx in 0..taken {
                let player = Player::from_raw(players & 0x1).unwrap();
                let pt = if idx == taken - 1 {
                    top
                } else {
                    PieceType::Flat
                };

                new_pos.stacks.push(sq, pt, player);

                pattern >>= 1;
                players >>= 1;

                if (pattern & 0x1) != 0 {
                    new_player_bbs[player.idx()].set_sq(sq);
                    new_flats_bb.set_sq(sq);

                    sq = sq.shift(dir).unwrap();
                }
            }

            new_player_bbs[self.stm().idx()].set_sq(sq);

            debug_assert_eq!(new_player_bbs[0] & new_player_bbs[1], Bitboard::empty());

            for player in 0..Player::COUNT {
                new_pos.players[player] = (new_pos.players[player] | new_player_bbs[player])
                    & !new_player_bbs[player ^ 0x1];
            }

            new_pos.pieces[top.idx()].set_sq(sq);
            new_pos.pieces[PieceType::Flat.idx()] |= new_flats_bb;

            if let Some(prev_dst_top) = self.stacks.top(sq)
                && prev_dst_top != top
            {
                new_pos.pieces[prev_dst_top.idx()].clear_sq(sq);
                new_pos.pieces[top.idx()].set_sq(sq);
            }

            debug_assert_eq!(
                new_pos.pieces[PieceType::Flat.idx()]
                    & new_pos.pieces[PieceType::Wall.idx()]
                    & new_pos.pieces[PieceType::Capstone.idx()],
                Bitboard::empty()
            );

            debug_assert_eq!(
                new_pos.players[Player::P1.idx()] & new_pos.players[Player::P2.idx()],
                Bitboard::empty()
            );

            debug_assert_eq!(
                new_pos.pieces[PieceType::Flat.idx()]
                    | new_pos.pieces[PieceType::Wall.idx()]
                    | new_pos.pieces[PieceType::Capstone.idx()],
                new_pos.players[Player::P1.idx()] | new_pos.players[Player::P2.idx()]
            );
        } else {
            debug_assert_eq!(self.stacks.top(mv.sq()), None);
            debug_assert!(self.ply() >= 2 || mv.pt() == PieceType::Flat);

            let dropped_player = if self.ply() < 2 {
                self.stm().flip()
            } else {
                self.stm()
            };

            new_pos.stacks.push(mv.sq(), mv.pt(), dropped_player);

            new_pos.players[dropped_player.idx()].set_sq(mv.sq());
            new_pos.pieces[mv.pt().idx()].set_sq(mv.sq());

            match mv.pt() {
                PieceType::Capstone => new_pos.caps_in_hand[self.stm().idx()] -= 1,
                _ => new_pos.flats_in_hand[self.stm().idx()] -= 1,
            }
        }

        new_pos.stm = new_pos.stm.flip();
        new_pos.ply += 1;

        new_pos
    }

    #[must_use]
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

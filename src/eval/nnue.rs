/*
 * syntaks, a TEI Tak engine
 * Copyright (c) 2026 Ciekce
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */

use super::forward::forward;
use crate::board::{BoardObserver, Position};
use crate::core::{Piece, PieceType, Player, Square, normalizer};
use crate::eval::accumulator::Accumulator;
use arrayvec::ArrayVec;

cfg_select! {
    all(target_feature = "avx512f", target_feature = "avx512bw") => {
        #[path = "simd/avx512.rs"]
        pub(super) mod simd;
    }
    target_feature = "avx2" => {
        #[path = "simd/avx2.rs"]
        pub(super) mod simd;
    }
    target_feature = "sse4.1" => {
        #[path = "simd/sse41.rs"]
        pub(super) mod simd;
    }
    _ => {
        compiler_error!("No supported SIMD arch");
    }
}

pub const TOP_INPUT_SIZE: usize = Player::COUNT * PieceType::COUNT * Square::COUNT;

pub static STACK_INPUT_CHUNKS: [u32; 5] = [3, 3, 2, 2, 1];
pub static STACK_INPUT_INDICES: [usize; STACK_INPUT_CHUNKS.len() + 1] = {
    let mut indices = [0; _];
    let mut idx = 0;
    let mut i = 1;
    while i < indices.len() {
        let chunk_size = STACK_INPUT_CHUNKS[i - 1];
        assert!(chunk_size < u8::BITS - 1, "chunk bits should fit inside struct Chunk");
        idx += (Player::COUNT.pow(chunk_size + 1) - 2) * TOP_INPUT_SIZE;
        indices[i] = idx;
        i += 1;
    }
    indices
};
pub const STACK_INPUT_SIZE: usize = STACK_INPUT_INDICES[5];

pub const L1_SIZE: usize = 512;

pub const OUTPUT_BUCKETS: usize = 2;

pub const FT_Q: i32 = 255;
pub const L1_Q: i32 = 64;

pub const SCALE: i32 = 400;

pub struct Chunk {
    pub idx: u8,
    pub bits: u8,
}

pub struct StackChunks {
    depth_left: u8,
    idx: u8,
    bits: u32,
}

impl StackChunks {
    #[must_use]
    pub fn new(perspective: Player, height: u8, players: u64) -> Option<Self> {
        if height < 2 {
            return None;
        }
        Some(Self {
            depth_left: height.min(13) - 1,
            idx: 0,
            bits: (perspective.idx() as u32 * !0) ^ (players as u32 >> 1),
        })
    }
}

impl Iterator for StackChunks {
    type Item = Chunk;

    fn next(&mut self) -> Option<Self::Item> {
        if self.depth_left == 0 {
            return None;
        }
        let count = (self.depth_left).min(STACK_INPUT_CHUNKS[self.idx as usize] as u8);
        let top_bit = 1 << count;
        let chunk = Chunk {
            idx: self.idx,
            bits: top_bit | ((top_bit - 1) & self.bits as u8),
        };
        self.depth_left -= count;
        self.bits >>= count;
        self.idx += 1;
        Some(chunk)
    }
}

#[repr(C, align(64))]
pub(super) struct Network {
    pub ftw_top: [[i16; L1_SIZE]; TOP_INPUT_SIZE],
    pub ftw_stack: [[i16; L1_SIZE]; STACK_INPUT_SIZE],
    pub ftb: [i16; L1_SIZE],
    pub l1w: [[[i16; L1_SIZE]; 2]; OUTPUT_BUCKETS],
    pub l1b: [i16; OUTPUT_BUCKETS],
}

// pub(super) static NET: Network = unsafe { std::mem::transmute(*include_bytes!(env!("EVALFILE"))) };
pub(super) static NET: Network = unsafe { std::mem::transmute([0u8; size_of::<Network>()]) }; // TODO: temporary

#[must_use]
pub(super) fn evaluate_once(pos: &Position) -> i32 {
    let mut acc = Accumulator::default();
    acc.reset_both(pos);
    forward(&acc, pos.stm())
}

pub(super) fn top_feature_idx(perspective: Player, side: Player, pt: PieceType, sq: Square) -> usize {
    (usize::from(side != perspective) * PieceType::COUNT + pt.idx()) * Square::COUNT + sq.idx()
}

pub(super) fn stack_feature_idx(perspective: Player, top: Piece, sq: Square, chunk: Chunk) -> usize {
    debug_assert!(chunk.idx < STACK_INPUT_CHUNKS.len() as u8);
    debug_assert!(chunk.bits >= 0b10);
    let top_idx = top_feature_idx(perspective, top.player(), top.piece_type(), sq);
    let content = (((chunk.bits.isolate_highest_one() - 1) * perspective.idx() as u8) ^ chunk.bits) - 2;
    top_idx + TOP_INPUT_SIZE * content as usize + STACK_INPUT_INDICES[chunk.idx as usize]
}

#[derive(Clone, Debug, Default)]
pub(super) struct NnueUpdates {
    pub adds: ArrayVec<(Piece, Square), 6>,
    pub subs: ArrayVec<(Piece, Square), 6>,
    pub stacks: ArrayVec<(Square, Option<Piece>, Option<Piece>, u8, u8, u32, u32), 6>,
    pub refresh: [bool; Player::COUNT],
}

impl NnueUpdates {
    #[must_use]
    pub fn requires_refresh(&self, player: Player) -> bool {
        self.refresh[player.idx()]
    }
}

#[derive(Clone, Debug, Default)]
pub(super) struct UpdateContext {
    pub updates: NnueUpdates,
    pub caps: [Option<Square>; Player::COUNT],
}

pub struct NnueObserver<'a> {
    ctx: &'a mut UpdateContext,
}

impl<'a> NnueObserver<'a> {
    pub(super) fn new(ctx: &'a mut UpdateContext) -> Self {
        Self { ctx }
    }
}

impl<'a> BoardObserver for NnueObserver<'a> {
    fn top_added(&mut self, _pos: &Position, top: Piece, sq: Square) {
        self.ctx.updates.adds.push((top, sq));
    }

    fn top_removed(&mut self, _pos: &Position, top: Piece, sq: Square) {
        self.ctx.updates.subs.push((top, sq));
    }

    fn stack_changed(
        &mut self,
        _pos: &Position,
        sq: Square,
        top_before: Option<Piece>,
        top_after: Option<Piece>,
        height_before: u8,
        height_after: u8,
        players_before: u64,
        players_after: u64,
    ) {
        self.ctx.updates.stacks.push((
            sq,
            top_before,
            top_after,
            height_before,
            height_after,
            players_before as u32,
            players_after as u32,
        ));
    }

    fn finalize(&mut self, pos_before: &Position, pos_after: &Position) {
        let should_refresh = |side: Player| normalizer(pos_before.cap_sq(side)) != normalizer(pos_after.cap_sq(side));

        self.ctx.updates.refresh = [Player::P1, Player::P2].map(should_refresh);

        let p1_cap = pos_after.player_piece_bb(Piece::P1Capstone).lsb();
        let p2_cap = pos_after.player_piece_bb(Piece::P2Capstone).lsb();
        self.ctx.caps = [p1_cap, p2_cap];
    }
}

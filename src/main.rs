use crate::board::Position;
use crate::core::*;
use crate::takmove::Move;

mod bitboard;
mod board;
mod core;
mod takmove;

fn main() {
    let pos = Position::startpos();

    let pos = pos.apply_move(Move::placement(PieceType::Flat, Square::A1));
    let pos = pos.apply_move(Move::placement(PieceType::Flat, Square::B1));
    let pos = pos.apply_move(Move::placement(PieceType::Wall, Square::C1));
    let pos = pos.apply_move(Move::placement(PieceType::Capstone, Square::D1));

    println!("{}", pos.tps());
}

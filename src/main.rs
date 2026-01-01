use crate::board::Position;
use crate::core::*;
use crate::takmove::Move;

mod bitboard;
mod board;
mod core;
mod takmove;

fn main() {
    let pos = Position::startpos();

    assert_eq!(pos, pos.tps().parse().unwrap());
    let pos = pos.apply_move(Move::placement(PieceType::Flat, Square::A1));
    assert_eq!(pos, pos.tps().parse().unwrap());
    let pos = pos.apply_move(Move::placement(PieceType::Flat, Square::B1));
    assert_eq!(pos, pos.tps().parse().unwrap());
    let pos = pos.apply_move(Move::placement(PieceType::Wall, Square::C1));
    assert_eq!(pos, pos.tps().parse().unwrap());
    let pos = pos.apply_move(Move::placement(PieceType::Capstone, Square::D1));
    assert_eq!(pos, pos.tps().parse().unwrap());
    let pos = pos.apply_move(Move::spread(Square::C1, Direction::Up, 0b100000));
    assert_eq!(pos, pos.tps().parse().unwrap());

    println!("{}", pos.tps());
}

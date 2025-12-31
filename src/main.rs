use crate::board::Position;

mod bitboard;
mod board;
mod core;
mod takmove;

fn main() {
    println!("{}", Position::startpos().tps());
}

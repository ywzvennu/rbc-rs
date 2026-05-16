use std::str::FromStr;

use cozy_chess::{Board, File, Rank};

use crate::types::{Color, Error, Piece, PieceKind, Square};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct CastlingRights {
    pub(crate) white_kingside: bool,
    pub(crate) white_queenside: bool,
    pub(crate) black_kingside: bool,
    pub(crate) black_queenside: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct Position {
    squares: [Option<Piece>; 64],
    turn: Color,
    castling_rights: CastlingRights,
    en_passant: Option<Square>,
    halfmove_clock: u16,
    fullmove_number: u16,
}

impl Position {
    pub(crate) fn standard() -> Self {
        Self::from_cozy_board(&Board::default())
    }

    pub(crate) fn from_fen(fen: &str) -> Result<Self, Error> {
        let board = Board::from_str(fen).map_err(|err| Error::InvalidFen(err.to_string()))?;
        Ok(Self::from_cozy_board(&board))
    }

    pub(crate) fn piece_at(&self, square: Square) -> Option<Piece> {
        self.squares[square.index() as usize]
    }

    pub(crate) fn set_piece(&mut self, square: Square, piece: Option<Piece>) {
        self.squares[square.index() as usize] = piece;
    }

    pub(crate) fn remove_piece(&mut self, square: Square) -> Option<Piece> {
        let piece = self.piece_at(square);
        self.set_piece(square, None);
        piece
    }

    pub(crate) fn turn(&self) -> Color {
        self.turn
    }

    pub(crate) fn castling_rights(&self) -> CastlingRights {
        self.castling_rights
    }

    pub(crate) fn en_passant(&self) -> Option<Square> {
        self.en_passant
    }

    pub(crate) fn halfmove_clock(&self) -> u16 {
        self.halfmove_clock
    }

    pub(crate) fn fullmove_number(&self) -> u16 {
        self.fullmove_number
    }

    pub(crate) fn set_en_passant(&mut self, square: Option<Square>) {
        self.en_passant = square;
    }

    pub(crate) fn null_move(&mut self) {
        self.en_passant = None;
        self.halfmove_clock = self.halfmove_clock.saturating_add(1);
        if self.turn == Color::Black {
            self.fullmove_number = self.fullmove_number.saturating_add(1);
        }
        self.turn = self.turn.opposite();
    }

    pub(crate) fn finish_move(
        &mut self,
        moving_piece: Piece,
        captured_piece: Option<Piece>,
        en_passant: Option<Square>,
    ) {
        self.set_en_passant(en_passant);
        self.halfmove_clock = if moving_piece.kind == PieceKind::Pawn || captured_piece.is_some() {
            0
        } else {
            self.halfmove_clock.saturating_add(1)
        };
        if self.turn == Color::Black {
            self.fullmove_number = self.fullmove_number.saturating_add(1);
        }
        self.turn = self.turn.opposite();
    }

    pub(crate) fn disable_castling_for_color(&mut self, color: Color) {
        match color {
            Color::White => {
                self.castling_rights.white_kingside = false;
                self.castling_rights.white_queenside = false;
            }
            Color::Black => {
                self.castling_rights.black_kingside = false;
                self.castling_rights.black_queenside = false;
            }
        }
    }

    pub(crate) fn disable_rook_castling_right(&mut self, color: Color, square: Square) {
        match (color, square.file(), square.rank()) {
            (Color::White, 0, 0) => self.castling_rights.white_queenside = false,
            (Color::White, 7, 0) => self.castling_rights.white_kingside = false,
            (Color::Black, 0, 7) => self.castling_rights.black_queenside = false,
            (Color::Black, 7, 7) => self.castling_rights.black_kingside = false,
            _ => {}
        }
    }

    pub(crate) fn to_fen(&self) -> String {
        let mut ranks = Vec::with_capacity(8);
        for rank in (0..8).rev() {
            let mut row = String::new();
            let mut empty_count = 0;
            for file in 0..8 {
                let square = Square::from_coords(file, rank).expect("valid square");
                if let Some(piece) = self.piece_at(square) {
                    if empty_count > 0 {
                        row.push(char::from_digit(empty_count, 10).expect("single digit"));
                        empty_count = 0;
                    }
                    row.push(piece_to_fen(piece));
                } else {
                    empty_count += 1;
                }
            }
            if empty_count > 0 {
                row.push(char::from_digit(empty_count, 10).expect("single digit"));
            }
            ranks.push(row);
        }

        let active_color = match self.turn {
            Color::White => 'w',
            Color::Black => 'b',
        };
        let en_passant = self
            .en_passant
            .map_or_else(|| "-".to_string(), Square::to_algebraic);

        format!(
            "{} {} {} {} {} {}",
            ranks.join("/"),
            active_color,
            self.castling_rights.to_fen(),
            en_passant,
            self.halfmove_clock,
            self.fullmove_number
        )
    }

    fn from_cozy_board(board: &Board) -> Self {
        let mut squares = [None; 64];
        for index in 0..64 {
            let square = Square::from_index(index).expect("valid square");
            let cozy_square = to_cozy_square(square);
            let Some(kind) = board.piece_on(cozy_square).map(from_cozy_piece) else {
                continue;
            };
            let color = board
                .color_on(cozy_square)
                .map(from_cozy_color)
                .expect("piece has color");
            squares[index as usize] = Some(Piece { color, kind });
        }

        let turn = from_cozy_color(board.side_to_move());
        let white_rights = board.castle_rights(cozy_chess::Color::White);
        let black_rights = board.castle_rights(cozy_chess::Color::Black);
        let en_passant = board.en_passant().map(|file| {
            let rank = match turn {
                Color::White => 5,
                Color::Black => 2,
            };
            Square::from_coords(file as u8, rank).expect("valid en passant square")
        });

        Self {
            squares,
            turn,
            castling_rights: CastlingRights {
                white_kingside: white_rights.short.is_some(),
                white_queenside: white_rights.long.is_some(),
                black_kingside: black_rights.short.is_some(),
                black_queenside: black_rights.long.is_some(),
            },
            en_passant,
            halfmove_clock: board.halfmove_clock().into(),
            fullmove_number: board.fullmove_number(),
        }
    }
}

impl CastlingRights {
    fn to_fen(self) -> String {
        let mut result = String::new();
        if self.white_kingside {
            result.push('K');
        }
        if self.white_queenside {
            result.push('Q');
        }
        if self.black_kingside {
            result.push('k');
        }
        if self.black_queenside {
            result.push('q');
        }
        if result.is_empty() {
            "-".to_string()
        } else {
            result
        }
    }
}

fn to_cozy_square(square: Square) -> cozy_chess::Square {
    cozy_chess::Square::new(
        File::index(square.file() as usize),
        Rank::index(square.rank() as usize),
    )
}

fn from_cozy_color(color: cozy_chess::Color) -> Color {
    match color {
        cozy_chess::Color::White => Color::White,
        cozy_chess::Color::Black => Color::Black,
    }
}

fn from_cozy_piece(piece: cozy_chess::Piece) -> PieceKind {
    match piece {
        cozy_chess::Piece::King => PieceKind::King,
        cozy_chess::Piece::Queen => PieceKind::Queen,
        cozy_chess::Piece::Rook => PieceKind::Rook,
        cozy_chess::Piece::Bishop => PieceKind::Bishop,
        cozy_chess::Piece::Knight => PieceKind::Knight,
        cozy_chess::Piece::Pawn => PieceKind::Pawn,
    }
}

fn piece_to_fen(piece: Piece) -> char {
    let symbol = match piece.kind {
        PieceKind::King => 'k',
        PieceKind::Queen => 'q',
        PieceKind::Rook => 'r',
        PieceKind::Bishop => 'b',
        PieceKind::Knight => 'n',
        PieceKind::Pawn => 'p',
    };
    match piece.color {
        Color::White => symbol.to_ascii_uppercase(),
        Color::Black => symbol,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sq(file: u8, rank: u8) -> Square {
        Square::from_coords(file, rank).unwrap()
    }

    #[test]
    fn standard_position_round_trips_as_fen() {
        let position = Position::standard();
        assert_eq!(
            position.to_fen(),
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"
        );
    }

    #[test]
    fn null_move_flips_turn_and_clears_en_passant() {
        let mut position = Position::from_fen("4k3/8/8/8/4P3/8/8/4K3 b - e3 0 1").unwrap();
        position.null_move();
        assert_eq!(position.turn(), Color::White);
        assert_eq!(position.en_passant(), None);
        assert_eq!(position.halfmove_clock, 1);
        assert_eq!(position.fullmove_number, 2);
    }

    #[test]
    fn en_passant_square_can_be_updated() {
        let mut position = Position::from_fen("4k3/8/8/8/8/8/8/4K3 w - - 0 1").unwrap();
        position.set_en_passant(Some(sq(4, 2)));
        assert_eq!(position.to_fen(), "4k3/8/8/8/8/8/8/4K3 w - e3 0 1");
    }

    #[test]
    fn position_can_render_after_king_capture() {
        let mut position = Position::from_fen("4k3/8/8/8/8/8/8/4K3 w - - 0 1").unwrap();
        position.set_piece(sq(4, 7), None);
        assert_eq!(position.to_fen(), "8/8/8/8/8/8/8/4K3 w - - 0 1");
    }
}

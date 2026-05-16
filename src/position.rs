use crate::types::{Color, Error, Piece, PieceKind, Square};

const STANDARD_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

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
        Self::from_fen(STANDARD_FEN).expect("standard starting FEN is valid")
    }

    pub(crate) fn from_fen(fen: &str) -> Result<Self, Error> {
        let fields: Vec<_> = fen.split_whitespace().collect();
        if fields.len() != 6 {
            return Err(invalid_fen("expected six FEN fields"));
        }

        let squares = parse_piece_placement(fields[0])?;
        let turn = parse_active_color(fields[1])?;
        let castling_rights = parse_castling_rights(fields[2])?;
        let en_passant = parse_en_passant(fields[3])?;
        let halfmove_clock = fields[4]
            .parse()
            .map_err(|_| invalid_fen("invalid halfmove clock"))?;
        let fullmove_number = fields[5]
            .parse()
            .map_err(|_| invalid_fen("invalid fullmove number"))?;
        if fullmove_number == 0 {
            return Err(invalid_fen("fullmove number must be at least one"));
        }

        Ok(Self {
            squares,
            turn,
            castling_rights,
            en_passant,
            halfmove_clock,
            fullmove_number,
        })
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

    pub(crate) fn has_king(&self, color: Color) -> bool {
        self.squares.iter().any(|piece| {
            *piece
                == Some(Piece {
                    color,
                    kind: PieceKind::King,
                })
        })
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

fn parse_piece_placement(placement: &str) -> Result<[Option<Piece>; 64], Error> {
    let ranks: Vec<_> = placement.split('/').collect();
    if ranks.len() != 8 {
        return Err(invalid_fen("expected eight ranks"));
    }

    let mut squares = [None; 64];
    for (fen_rank, row) in ranks.into_iter().enumerate() {
        let rank = 7 - fen_rank as u8;
        let mut file = 0_u8;
        for symbol in row.chars() {
            if let Some(empty_count) = symbol.to_digit(10) {
                if empty_count == 0 || empty_count > 8 {
                    return Err(invalid_fen("invalid empty-square count"));
                }
                file = file
                    .checked_add(empty_count as u8)
                    .ok_or_else(|| invalid_fen("rank exceeds eight files"))?;
                if file > 8 {
                    return Err(invalid_fen("rank exceeds eight files"));
                }
                continue;
            }

            if file >= 8 {
                return Err(invalid_fen("rank exceeds eight files"));
            }
            let square = Square::from_coords(file, rank).expect("validated square");
            squares[square.index() as usize] = Some(parse_piece(symbol)?);
            file += 1;
        }

        if file != 8 {
            return Err(invalid_fen("rank does not contain eight files"));
        }
    }

    Ok(squares)
}

fn parse_piece(symbol: char) -> Result<Piece, Error> {
    let color = if symbol.is_ascii_uppercase() {
        Color::White
    } else {
        Color::Black
    };
    let kind = match symbol.to_ascii_lowercase() {
        'k' => PieceKind::King,
        'q' => PieceKind::Queen,
        'r' => PieceKind::Rook,
        'b' => PieceKind::Bishop,
        'n' => PieceKind::Knight,
        'p' => PieceKind::Pawn,
        _ => return Err(invalid_fen("invalid piece symbol")),
    };
    Ok(Piece { color, kind })
}

fn parse_active_color(field: &str) -> Result<Color, Error> {
    match field {
        "w" => Ok(Color::White),
        "b" => Ok(Color::Black),
        _ => Err(invalid_fen("invalid active color")),
    }
}

fn parse_castling_rights(field: &str) -> Result<CastlingRights, Error> {
    if field == "-" {
        return Ok(CastlingRights {
            white_kingside: false,
            white_queenside: false,
            black_kingside: false,
            black_queenside: false,
        });
    }

    let mut rights = CastlingRights {
        white_kingside: false,
        white_queenside: false,
        black_kingside: false,
        black_queenside: false,
    };
    for symbol in field.chars() {
        let right = match symbol {
            'K' => &mut rights.white_kingside,
            'Q' => &mut rights.white_queenside,
            'k' => &mut rights.black_kingside,
            'q' => &mut rights.black_queenside,
            _ => return Err(invalid_fen("invalid castling rights")),
        };
        if *right {
            return Err(invalid_fen("duplicate castling right"));
        }
        *right = true;
    }
    Ok(rights)
}

fn parse_en_passant(field: &str) -> Result<Option<Square>, Error> {
    if field == "-" {
        return Ok(None);
    }
    let bytes = field.as_bytes();
    if bytes.len() != 2 || !(b'a'..=b'h').contains(&bytes[0]) || !matches!(bytes[1], b'3' | b'6') {
        return Err(invalid_fen("invalid en passant square"));
    }
    Ok(Square::from_coords(bytes[0] - b'a', bytes[1] - b'1'))
}

fn invalid_fen(message: &str) -> Error {
    Error::InvalidFen(message.to_string())
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

    #[test]
    fn fen_parser_accepts_rbc_positions_with_king_in_check() {
        let position = Position::from_fen("4k3/8/8/8/8/6q1/8/4K3 w - - 0 1").unwrap();
        assert_eq!(position.to_fen(), "4k3/8/8/8/8/6q1/8/4K3 w - - 0 1");
    }

    #[test]
    fn fen_parser_accepts_positions_after_king_capture() {
        let position = Position::from_fen("8/8/8/8/8/8/8/4K3 b - - 0 1").unwrap();
        assert_eq!(position.to_fen(), "8/8/8/8/8/8/8/4K3 b - - 0 1");
    }

    #[test]
    fn fen_parser_rejects_invalid_structure() {
        assert_eq!(
            Position::from_fen("8/8/8/8/8/8/8/8 w - - 0").unwrap_err(),
            invalid_fen("expected six FEN fields")
        );
        assert_eq!(
            Position::from_fen("8/8/8/8/8/8/8/7 w - - 0 1").unwrap_err(),
            invalid_fen("rank does not contain eight files")
        );
        assert_eq!(
            Position::from_fen("8/8/8/8/8/8/8/8 x - - 0 1").unwrap_err(),
            invalid_fen("invalid active color")
        );
    }
}

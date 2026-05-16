use crate::types::{CastlingPolicy, Color, Error, Piece, PieceKind, Square};

const STANDARD_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

/// Castling rights stored per side, per direction.
///
/// `Some(file)` records the file of the rook that would participate
/// in this castling. `None` means the right is no longer available.
///
/// For standard chess the rook files are always 0 (queenside) and 7
/// (kingside). For Chess960 / X-FEN positions the rook may start on
/// any file flanking the king — the stored file lets move generation
/// look up the right rook without re-scanning the board.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct CastlingRights {
    pub(crate) white_kingside: Option<u8>,
    pub(crate) white_queenside: Option<u8>,
    pub(crate) black_kingside: Option<u8>,
    pub(crate) black_queenside: Option<u8>,
}

impl CastlingRights {
    pub(crate) const fn none() -> Self {
        Self {
            white_kingside: None,
            white_queenside: None,
            black_kingside: None,
            black_queenside: None,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Position {
    bitboards: [[u64; 6]; 2],
    occupied_by: [u64; 2],
    occupied: u64,
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

    /// Assembles a starting [`Position`] from white's and black's
    /// back-rank arrangements.
    ///
    /// Ranks 2 / 7 are filled with pawns, ranks 3–6 are empty. The
    /// castling-rights field is derived from the king + rook
    /// positions on each back rank (the king's nearest h-side rook
    /// gives kingside rights; nearest a-side rook gives queenside),
    /// intersected with `policy`. White is to move; halfmove clock
    /// 0; fullmove 1.
    ///
    /// Returns `Err` if either back rank lacks a king (the function
    /// otherwise tolerates any 8-piece arrangement).
    pub(crate) fn from_starting_backranks(
        white_backrank: &[crate::types::PieceKind; 8],
        black_backrank: &[crate::types::PieceKind; 8],
        policy: &CastlingPolicy,
    ) -> Result<Self, Error> {
        let mut bitboards = [[0u64; 6]; 2];
        let mut occupied_by = [0u64; 2];
        let mut occupied = 0u64;

        // Rank 1 — white back rank.
        for (file, kind) in white_backrank.iter().enumerate() {
            let bit = 1u64 << (file as u32);
            bitboards[Color::White.index()][kind.index()] |= bit;
            occupied_by[Color::White.index()] |= bit;
            occupied |= bit;
        }
        // Rank 2 — white pawns.
        for file in 0..8 {
            let bit = 1u64 << (8 + file);
            bitboards[Color::White.index()][PieceKind::Pawn.index()] |= bit;
            occupied_by[Color::White.index()] |= bit;
            occupied |= bit;
        }
        // Rank 7 — black pawns.
        for file in 0..8 {
            let bit = 1u64 << (48 + file);
            bitboards[Color::Black.index()][PieceKind::Pawn.index()] |= bit;
            occupied_by[Color::Black.index()] |= bit;
            occupied |= bit;
        }
        // Rank 8 — black back rank.
        for (file, kind) in black_backrank.iter().enumerate() {
            let bit = 1u64 << (56 + file as u32);
            bitboards[Color::Black.index()][kind.index()] |= bit;
            occupied_by[Color::Black.index()] |= bit;
            occupied |= bit;
        }

        // Derive structural castling rights from king + rook positions.
        let derive = |color: Color, kingside: bool| -> Option<u8> {
            find_rook_file(&bitboards, color, kingside)
        };
        let mut rights = CastlingRights {
            white_kingside: derive(Color::White, true),
            white_queenside: derive(Color::White, false),
            black_kingside: derive(Color::Black, true),
            black_queenside: derive(Color::Black, false),
        };
        if !policy.white_kingside {
            rights.white_kingside = None;
        }
        if !policy.white_queenside {
            rights.white_queenside = None;
        }
        if !policy.black_kingside {
            rights.black_kingside = None;
        }
        if !policy.black_queenside {
            rights.black_queenside = None;
        }

        // Both back ranks must contain a king for the position to be
        // valid for play.
        if bitboards[Color::White.index()][PieceKind::King.index()] == 0
            || bitboards[Color::Black.index()][PieceKind::King.index()] == 0
        {
            return Err(invalid_fen("starting back rank missing a king"));
        }

        Ok(Self {
            bitboards,
            occupied_by,
            occupied,
            turn: Color::White,
            castling_rights: rights,
            en_passant: None,
            halfmove_clock: 0,
            fullmove_number: 1,
        })
    }

    pub(crate) fn from_fen(fen: &str) -> Result<Self, Error> {
        let fields: Vec<_> = fen.split_whitespace().collect();
        if fields.len() != 6 {
            return Err(invalid_fen("expected six FEN fields"));
        }

        let placement = parse_piece_placement(fields[0])?;
        let bitboards = placement.bitboards;
        let occupied_by = placement.occupied_by;
        let occupied = placement.occupied;
        let turn = parse_active_color(fields[1])?;
        let castling_rights = parse_castling_rights(fields[2], &bitboards)?;
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
            bitboards,
            occupied_by,
            occupied,
            turn,
            castling_rights,
            en_passant,
            halfmove_clock,
            fullmove_number,
        })
    }

    pub(crate) fn piece_at(&self, square: Square) -> Option<Piece> {
        let bit = 1u64 << square.index();
        if self.occupied & bit == 0 {
            return None;
        }
        let color = if self.occupied_by[Color::White.index()] & bit != 0 {
            Color::White
        } else {
            Color::Black
        };
        let kinds = &self.bitboards[color.index()];
        for (idx, &bb) in kinds.iter().enumerate() {
            if bb & bit != 0 {
                return Some(Piece {
                    color,
                    kind: PieceKind::from_index(idx),
                });
            }
        }
        unreachable!("occupied bit set but no piece kind matched")
    }

    pub(crate) fn set_piece(&mut self, square: Square, piece: Option<Piece>) {
        let bit = 1u64 << square.index();
        let not_bit = !bit;
        if self.occupied & bit != 0 {
            // Clear whatever piece was here.
            for color_idx in 0..2 {
                if self.occupied_by[color_idx] & bit != 0 {
                    for bb in &mut self.bitboards[color_idx] {
                        *bb &= not_bit;
                    }
                    self.occupied_by[color_idx] &= not_bit;
                    break;
                }
            }
            self.occupied &= not_bit;
        }
        if let Some(p) = piece {
            self.bitboards[p.color.index()][p.kind.index()] |= bit;
            self.occupied_by[p.color.index()] |= bit;
            self.occupied |= bit;
        }
    }

    pub(crate) fn remove_piece(&mut self, square: Square) -> Option<Piece> {
        let piece = self.piece_at(square);
        if piece.is_some() {
            self.set_piece(square, None);
        }
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
        self.bitboards[color.index()][PieceKind::King.index()] != 0
    }

    pub(crate) fn piece_bitboard(&self, color: Color, kind: PieceKind) -> u64 {
        self.bitboards[color.index()][kind.index()]
    }

    pub(crate) fn occupied_by(&self, color: Color) -> u64 {
        self.occupied_by[color.index()]
    }

    pub(crate) fn occupied(&self) -> u64 {
        self.occupied
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
                self.castling_rights.white_kingside = None;
                self.castling_rights.white_queenside = None;
            }
            Color::Black => {
                self.castling_rights.black_kingside = None;
                self.castling_rights.black_queenside = None;
            }
        }
    }

    /// Called when a rook moves or is captured. Clears the castling
    /// right whose rook starting file matches `square.file()` on
    /// `color`'s home rank.
    pub(crate) fn disable_rook_castling_right(&mut self, color: Color, square: Square) {
        if square.rank() != color.home_rank() {
            return;
        }
        let file = square.file();
        match color {
            Color::White => {
                if self.castling_rights.white_kingside == Some(file) {
                    self.castling_rights.white_kingside = None;
                }
                if self.castling_rights.white_queenside == Some(file) {
                    self.castling_rights.white_queenside = None;
                }
            }
            Color::Black => {
                if self.castling_rights.black_kingside == Some(file) {
                    self.castling_rights.black_kingside = None;
                }
                if self.castling_rights.black_queenside == Some(file) {
                    self.castling_rights.black_queenside = None;
                }
            }
        }
    }

    pub(crate) fn to_fen(&self) -> String {
        let mut chars = [0u8; 64];
        for color_idx in 0..2 {
            let color = if color_idx == 0 {
                Color::White
            } else {
                Color::Black
            };
            for kind_idx in 0..6 {
                let kind = PieceKind::from_index(kind_idx);
                let mut bb = self.bitboards[color_idx][kind_idx];
                while bb != 0 {
                    let idx = bb.trailing_zeros() as usize;
                    bb &= bb - 1;
                    chars[idx] = piece_to_fen(Piece { color, kind }) as u8;
                }
            }
        }

        let mut ranks = Vec::with_capacity(8);
        for rank in (0..8).rev() {
            let mut row = String::new();
            let mut empty_count = 0;
            for file in 0..8 {
                let idx = (rank * 8 + file) as usize;
                let c = chars[idx];
                if c == 0 {
                    empty_count += 1;
                } else {
                    if empty_count > 0 {
                        row.push(char::from_digit(empty_count, 10).expect("single digit"));
                        empty_count = 0;
                    }
                    row.push(c as char);
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
            castling_rights_to_fen(self.castling_rights),
            en_passant,
            self.halfmove_clock,
            self.fullmove_number
        )
    }
}

/// Renders castling rights as a FEN field.
///
/// Emits the standard `KQkq` form when every available right's rook
/// is on the conventional outer file (a / h on its side of the king).
/// Otherwise emits the Shredder-FEN form using the rook file letter
/// (uppercase for white, lowercase for black).
fn castling_rights_to_fen(rights: CastlingRights) -> String {
    let king_file = |bitboards: Option<u8>, _name: &str| -> Option<u8> { bitboards };
    let _ = king_file;
    let mut result = String::new();

    // We need the king file to decide if a rook file is "conventional"
    // (a-file/h-file relative to king). We don't have direct access
    // here, so we use a simpler heuristic: standard FEN if and only
    // if every rook file is 0 or 7.
    let is_standard = [
        rights.white_kingside,
        rights.white_queenside,
        rights.black_kingside,
        rights.black_queenside,
    ]
    .iter()
    .all(|r| matches!(r, None | Some(0) | Some(7)));

    if is_standard {
        if rights.white_kingside.is_some() {
            result.push('K');
        }
        if rights.white_queenside.is_some() {
            result.push('Q');
        }
        if rights.black_kingside.is_some() {
            result.push('k');
        }
        if rights.black_queenside.is_some() {
            result.push('q');
        }
    } else {
        if let Some(f) = rights.white_kingside {
            result.push((b'A' + f) as char);
        }
        if let Some(f) = rights.white_queenside {
            result.push((b'A' + f) as char);
        }
        if let Some(f) = rights.black_kingside {
            result.push((b'a' + f) as char);
        }
        if let Some(f) = rights.black_queenside {
            result.push((b'a' + f) as char);
        }
    }

    if result.is_empty() {
        "-".to_string()
    } else {
        result
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

struct PiecePlacement {
    bitboards: [[u64; 6]; 2],
    occupied_by: [u64; 2],
    occupied: u64,
}

fn parse_piece_placement(placement: &str) -> Result<PiecePlacement, Error> {
    let ranks: Vec<_> = placement.split('/').collect();
    if ranks.len() != 8 {
        return Err(invalid_fen("expected eight ranks"));
    }

    let mut bitboards = [[0u64; 6]; 2];
    let mut occupied_by = [0u64; 2];
    let mut occupied = 0u64;
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
            let bit = 1u64 << square.index();
            let piece = parse_piece(symbol)?;
            bitboards[piece.color.index()][piece.kind.index()] |= bit;
            occupied_by[piece.color.index()] |= bit;
            occupied |= bit;
            file += 1;
        }

        if file != 8 {
            return Err(invalid_fen("rank does not contain eight files"));
        }
    }

    Ok(PiecePlacement {
        bitboards,
        occupied_by,
        occupied,
    })
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

/// Find the rook file for a single castling right on `color`'s home
/// rank. `prefer_h_side` = `true` for kingside (look right of king),
/// `false` for queenside (look left of king).
fn find_rook_file(bitboards: &[[u64; 6]; 2], color: Color, prefer_h_side: bool) -> Option<u8> {
    let home_rank = color.home_rank() as u32;
    let king_file = {
        let king_bb = bitboards[color.index()][PieceKind::King.index()];
        // Find the king on its home rank.
        let rank_mask = 0xffu64 << (home_rank * 8);
        let king_on_home = king_bb & rank_mask;
        if king_on_home == 0 {
            return None;
        }
        (king_on_home.trailing_zeros() as u8) % 8
    };
    let rook_bb = bitboards[color.index()][PieceKind::Rook.index()];
    let rank_mask = 0xffu64 << (home_rank * 8);
    let rooks_on_home = rook_bb & rank_mask;

    let mut candidates: Vec<u8> = Vec::new();
    let mut bits = rooks_on_home;
    while bits != 0 {
        let idx = bits.trailing_zeros() as u8;
        bits &= bits - 1;
        candidates.push(idx % 8);
    }
    if prefer_h_side {
        // Nearest h-side rook = max file > king_file.
        candidates.into_iter().filter(|&f| f > king_file).max()
    } else {
        // Nearest a-side rook = min file < king_file.
        candidates.into_iter().filter(|&f| f < king_file).min()
    }
}

/// Parse a castling-rights field. Supports both standard `KQkq` form
/// (rook files inferred relative to the king) and Shredder-FEN form
/// `AHah` (explicit rook file letters).
fn parse_castling_rights(field: &str, bitboards: &[[u64; 6]; 2]) -> Result<CastlingRights, Error> {
    if field == "-" {
        return Ok(CastlingRights::none());
    }

    let mut rights = CastlingRights::none();
    for symbol in field.chars() {
        match symbol {
            'K' => {
                if rights.white_kingside.is_some() {
                    return Err(invalid_fen("duplicate castling right"));
                }
                rights.white_kingside = Some(
                    find_rook_file(bitboards, Color::White, true)
                        .ok_or_else(|| invalid_fen("no kingside rook for white"))?,
                );
            }
            'Q' => {
                if rights.white_queenside.is_some() {
                    return Err(invalid_fen("duplicate castling right"));
                }
                rights.white_queenside = Some(
                    find_rook_file(bitboards, Color::White, false)
                        .ok_or_else(|| invalid_fen("no queenside rook for white"))?,
                );
            }
            'k' => {
                if rights.black_kingside.is_some() {
                    return Err(invalid_fen("duplicate castling right"));
                }
                rights.black_kingside = Some(
                    find_rook_file(bitboards, Color::Black, true)
                        .ok_or_else(|| invalid_fen("no kingside rook for black"))?,
                );
            }
            'q' => {
                if rights.black_queenside.is_some() {
                    return Err(invalid_fen("duplicate castling right"));
                }
                rights.black_queenside = Some(
                    find_rook_file(bitboards, Color::Black, false)
                        .ok_or_else(|| invalid_fen("no queenside rook for black"))?,
                );
            }
            // Shredder form: explicit rook file. Determine kingside /
            // queenside by comparing to the king's file.
            'A'..='H' | 'a'..='h' => {
                let (color, file) = if symbol.is_ascii_uppercase() {
                    (Color::White, symbol as u8 - b'A')
                } else {
                    (Color::Black, symbol as u8 - b'a')
                };
                let home_rank = color.home_rank() as u32;
                let king_bb = bitboards[color.index()][PieceKind::King.index()];
                let rank_mask = 0xffu64 << (home_rank * 8);
                let king_on_home = king_bb & rank_mask;
                if king_on_home == 0 {
                    return Err(invalid_fen(
                        "Shredder castling field without king on home rank",
                    ));
                }
                let king_file = (king_on_home.trailing_zeros() as u8) % 8;
                if file > king_file {
                    if (match color {
                        Color::White => rights.white_kingside,
                        Color::Black => rights.black_kingside,
                    })
                    .is_some()
                    {
                        return Err(invalid_fen("duplicate castling right"));
                    }
                    match color {
                        Color::White => rights.white_kingside = Some(file),
                        Color::Black => rights.black_kingside = Some(file),
                    }
                } else if file < king_file {
                    if (match color {
                        Color::White => rights.white_queenside,
                        Color::Black => rights.black_queenside,
                    })
                    .is_some()
                    {
                        return Err(invalid_fen("duplicate castling right"));
                    }
                    match color {
                        Color::White => rights.white_queenside = Some(file),
                        Color::Black => rights.black_queenside = Some(file),
                    }
                } else {
                    return Err(invalid_fen("castling rook file equals king file"));
                }
            }
            _ => return Err(invalid_fen("invalid castling rights")),
        }
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

    #[test]
    fn standard_castling_field_round_trips() {
        let position =
            Position::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
        let rights = position.castling_rights();
        assert_eq!(rights.white_kingside, Some(7));
        assert_eq!(rights.white_queenside, Some(0));
        assert_eq!(rights.black_kingside, Some(7));
        assert_eq!(rights.black_queenside, Some(0));
        assert_eq!(
            position.to_fen(),
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"
        );
    }

    #[test]
    fn shredder_castling_field_parses_and_round_trips() {
        // Non-standard position: king on c1 (file 2), rooks on b1
        // (file 1) and h1 (file 7). Shredder field: HBhb.
        let fen = "nrkbnbqr/pppppppp/8/8/8/8/PPPPPPPP/NRKBNBQR w HBhb - 0 1";
        let position = Position::from_fen(fen).unwrap();
        let rights = position.castling_rights();
        assert_eq!(rights.white_kingside, Some(7));
        assert_eq!(rights.white_queenside, Some(1));
        assert_eq!(rights.black_kingside, Some(7));
        assert_eq!(rights.black_queenside, Some(1));
        // Emits Shredder form because queenside rook is on file 1.
        assert!(position.to_fen().contains(" HBhb "));
    }

    #[test]
    fn castling_dash_means_no_rights() {
        let position = Position::from_fen("4k3/8/8/8/8/8/8/4K3 w - - 0 1").unwrap();
        let rights = position.castling_rights();
        assert_eq!(rights.white_kingside, None);
        assert_eq!(rights.white_queenside, None);
        assert_eq!(rights.black_kingside, None);
        assert_eq!(rights.black_queenside, None);
    }
}

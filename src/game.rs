use std::str::FromStr;

use cozy_chess::{Board, File, Rank};

use crate::types::{
    Color, Error, GameConfig, GameStatus, HistoryEntry, Piece, PieceKind, SenseResult,
    SensedSquare, Square,
};

/// Reconnaissance Blind Chess game state.
#[derive(Clone, Debug)]
pub struct Game {
    board: Board,
    config: GameConfig,
    status: GameStatus,
    history: Vec<HistoryEntry>,
    pending_capture: [Option<Square>; 2],
}

impl Game {
    /// Creates a game in the standard chess starting position.
    #[must_use]
    pub fn new(config: GameConfig) -> Self {
        Self {
            board: Board::default(),
            config,
            status: GameStatus::Ongoing { turn: Color::White },
            history: Vec::new(),
            pending_capture: [None, None],
        }
    }

    /// Creates a game from a FEN string.
    pub fn from_fen(fen: &str, config: GameConfig) -> Result<Self, Error> {
        let board = Board::from_str(fen).map_err(|err| Error::InvalidFen(err.to_string()))?;
        let turn = from_cozy_color(board.side_to_move());
        Ok(Self {
            board,
            config,
            status: GameStatus::Ongoing { turn },
            history: Vec::new(),
            pending_capture: [None, None],
        })
    }

    /// Returns the current FEN string.
    #[must_use]
    pub fn to_fen(&self) -> String {
        self.board.to_string()
    }

    /// Returns static game configuration.
    #[must_use]
    pub fn config(&self) -> &GameConfig {
        &self.config
    }

    /// Returns current game status.
    #[must_use]
    pub fn status(&self) -> &GameStatus {
        &self.status
    }

    /// Returns recorded history entries.
    #[must_use]
    pub fn history(&self) -> &[HistoryEntry] {
        &self.history
    }

    /// Returns the side to move if the game is ongoing.
    #[must_use]
    pub fn turn(&self) -> Option<Color> {
        match self.status {
            GameStatus::Ongoing { turn } => Some(turn),
            GameStatus::Won(_) | GameStatus::Draw { .. } => None,
        }
    }

    /// Returns all legal sense center squares.
    #[must_use]
    pub fn sense_actions(&self) -> Vec<Square> {
        if self.turn().is_none() {
            return Vec::new();
        }
        (0..64).filter_map(Square::from_index).collect()
    }

    /// Returns the square where the opponent captured a piece before this turn.
    #[must_use]
    pub fn opponent_capture_square(&self, color: Color) -> Option<Square> {
        self.pending_capture[color_index(color)]
    }

    /// Performs a sense action.
    pub fn sense(&self, center: Option<Square>) -> SenseResult {
        let Some(center) = center else {
            return SenseResult {
                center: None,
                squares: Vec::new(),
            };
        };

        let mut squares = Vec::with_capacity(9);
        let rank = center.rank() as i8;
        let file = center.file() as i8;
        for delta_rank in [1, 0, -1] {
            for delta_file in [-1, 0, 1] {
                let next_rank = rank + delta_rank;
                let next_file = file + delta_file;
                if (0..=7).contains(&next_rank) && (0..=7).contains(&next_file) {
                    let square =
                        Square::from_coords(next_file as u8, next_rank as u8).expect("in bounds");
                    squares.push(SensedSquare {
                        square,
                        piece: self.piece_at(square),
                    });
                }
            }
        }

        SenseResult {
            center: Some(center),
            squares,
        }
    }

    /// Returns the piece at a square.
    #[must_use]
    pub fn piece_at(&self, square: Square) -> Option<Piece> {
        let cozy_square = to_cozy_square(square);
        let kind = self.board.piece_on(cozy_square).map(from_cozy_piece)?;
        let color = self.board.color_on(cozy_square).map(from_cozy_color)?;
        Some(Piece { color, kind })
    }
}

fn color_index(color: Color) -> usize {
    match color {
        Color::White => 0,
        Color::Black => 1,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn sq(file: u8, rank: u8) -> Square {
        Square::from_coords(file, rank).unwrap()
    }

    #[test]
    fn standard_position_round_trips_as_fen() {
        let game = Game::new(GameConfig::default());
        assert_eq!(
            game.to_fen(),
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"
        );
    }

    #[test]
    fn parses_fen_and_exposes_turn() {
        let game = Game::from_fen("4k3/8/8/8/8/8/8/4K3 b - - 0 12", GameConfig::default()).unwrap();
        assert_eq!(game.turn(), Some(Color::Black));
        assert_eq!(game.to_fen(), "4k3/8/8/8/8/8/8/4K3 b - - 0 12");
    }

    #[test]
    fn sense_center_returns_rank_descending_file_ascending_window() {
        let game = Game::new(GameConfig::default());
        let result = game.sense(Some(sq(1, 1)));
        let squares: Vec<Square> = result.squares.iter().map(|entry| entry.square).collect();
        assert_eq!(
            squares,
            vec![
                sq(0, 2),
                sq(1, 2),
                sq(2, 2),
                sq(0, 1),
                sq(1, 1),
                sq(2, 1),
                sq(0, 0),
                sq(1, 0),
                sq(2, 0),
            ]
        );
    }

    #[test]
    fn sense_corner_is_clipped() {
        let game = Game::new(GameConfig::default());
        let result = game.sense(Some(sq(0, 7)));
        let squares: Vec<Square> = result.squares.iter().map(|entry| entry.square).collect();
        assert_eq!(squares, vec![sq(0, 7), sq(1, 7), sq(0, 6), sq(1, 6)]);
    }

    #[test]
    fn pass_sense_returns_empty_result() {
        let game = Game::new(GameConfig::default());
        assert!(game.sense(None).squares.is_empty());
    }
}

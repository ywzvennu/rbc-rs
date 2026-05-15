use crate::position::Position;
use crate::types::{
    Color, Error, GameConfig, GameStatus, HistoryEntry, Move, Piece, PieceKind, SenseResult,
    SensedSquare, Square,
};

/// Reconnaissance Blind Chess game state.
#[derive(Clone, Debug)]
pub struct Game {
    position: Position,
    config: GameConfig,
    status: GameStatus,
    history: Vec<HistoryEntry>,
    pending_capture: [Option<Square>; 2],
}

impl Game {
    /// Creates a game in the standard chess starting position.
    #[must_use]
    pub fn new(config: GameConfig) -> Self {
        let position = Position::standard();
        Self {
            status: GameStatus::Ongoing {
                turn: position.turn(),
            },
            position,
            config,
            history: Vec::new(),
            pending_capture: [None, None],
        }
    }

    /// Creates a game from a FEN string.
    pub fn from_fen(fen: &str, config: GameConfig) -> Result<Self, Error> {
        let position = Position::from_fen(fen)?;
        Ok(Self {
            status: GameStatus::Ongoing {
                turn: position.turn(),
            },
            position,
            config,
            history: Vec::new(),
            pending_capture: [None, None],
        })
    }

    /// Returns the current FEN string.
    #[must_use]
    pub fn to_fen(&self) -> String {
        self.position.to_fen()
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
        self.pending_capture[color.index()]
    }

    /// Performs a sense action.
    #[must_use]
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
        self.position.piece_at(square)
    }

    /// Returns move requests available from the acting player's information.
    #[must_use]
    pub fn move_actions(&self) -> Vec<Move> {
        let Some(turn) = self.turn() else {
            return Vec::new();
        };

        let mut moves = Vec::new();
        for index in 0..64 {
            let from = Square::from_index(index).expect("valid square");
            let Some(piece) = self.piece_at(from) else {
                continue;
            };
            if piece.color != turn {
                continue;
            }
            self.add_piece_move_actions(from, piece, &mut moves);
        }
        moves.sort_by_key(move_sort_key);
        moves.dedup();
        moves
    }

    fn add_piece_move_actions(&self, from: Square, piece: Piece, moves: &mut Vec<Move>) {
        match piece.kind {
            PieceKind::Pawn => self.add_pawn_move_actions(from, piece.color, moves),
            PieceKind::Knight => {
                for (df, dr) in [
                    (1, 2),
                    (2, 1),
                    (2, -1),
                    (1, -2),
                    (-1, -2),
                    (-2, -1),
                    (-2, 1),
                    (-1, 2),
                ] {
                    self.add_step_move(from, piece.color, df, dr, moves);
                }
            }
            PieceKind::Bishop => self.add_ray_move_actions(
                from,
                piece.color,
                &[(1, 1), (1, -1), (-1, 1), (-1, -1)],
                moves,
            ),
            PieceKind::Rook => self.add_ray_move_actions(
                from,
                piece.color,
                &[(1, 0), (-1, 0), (0, 1), (0, -1)],
                moves,
            ),
            PieceKind::Queen => self.add_ray_move_actions(
                from,
                piece.color,
                &[
                    (1, 0),
                    (-1, 0),
                    (0, 1),
                    (0, -1),
                    (1, 1),
                    (1, -1),
                    (-1, 1),
                    (-1, -1),
                ],
                moves,
            ),
            PieceKind::King => {
                for df in -1..=1 {
                    for dr in -1..=1 {
                        if df != 0 || dr != 0 {
                            self.add_step_move(from, piece.color, df, dr, moves);
                        }
                    }
                }
                self.add_castling_move_actions(from, piece.color, moves);
            }
        }
    }

    fn add_pawn_move_actions(&self, from: Square, color: Color, moves: &mut Vec<Move>) {
        let dir = pawn_dir(color);
        let promotion_rank = pawn_promotion_rank(color);
        if let Some(one_step) = offset(from, 0, dir) {
            if !self.has_own_piece(one_step, color) {
                add_promotion_moves(from, one_step, promotion_rank, moves);
                if from.rank() == pawn_start_rank(color) {
                    if let Some(two_step) = offset(one_step, 0, dir) {
                        if !self.has_own_piece(two_step, color) {
                            moves.push(Move {
                                from,
                                to: two_step,
                                promotion: None,
                            });
                        }
                    }
                }
            }
        }

        for df in [-1, 1] {
            if let Some(to) = offset(from, df, dir) {
                if !self.has_own_piece(to, color) {
                    add_promotion_moves(from, to, promotion_rank, moves);
                }
            }
        }
    }

    fn add_step_move(&self, from: Square, color: Color, df: i8, dr: i8, moves: &mut Vec<Move>) {
        if let Some(to) = offset(from, df, dr) {
            if !self.has_own_piece(to, color) {
                moves.push(Move {
                    from,
                    to,
                    promotion: None,
                });
            }
        }
    }

    fn add_ray_move_actions(
        &self,
        from: Square,
        color: Color,
        directions: &[(i8, i8)],
        moves: &mut Vec<Move>,
    ) {
        for &(df, dr) in directions {
            let mut current = from;
            while let Some(to) = offset(current, df, dr) {
                if self.has_own_piece(to, color) {
                    break;
                }
                moves.push(Move {
                    from,
                    to,
                    promotion: None,
                });
                current = to;
            }
        }
    }

    fn add_castling_move_actions(&self, from: Square, color: Color, moves: &mut Vec<Move>) {
        let home_rank = home_rank(color);
        if from != Square::from_coords(4, home_rank).expect("valid square") {
            return;
        }

        let rights = self.position.castling_rights();
        let (kingside, queenside) = match color {
            Color::White => (rights.white_kingside, rights.white_queenside),
            Color::Black => (rights.black_kingside, rights.black_queenside),
        };
        if kingside && self.castle_path_clear_of_own_pieces(color, 5..=6) {
            moves.push(Move {
                from,
                to: Square::from_coords(6, home_rank).expect("valid square"),
                promotion: None,
            });
        }
        if queenside && self.castle_path_clear_of_own_pieces(color, 1..=3) {
            moves.push(Move {
                from,
                to: Square::from_coords(2, home_rank).expect("valid square"),
                promotion: None,
            });
        }
    }

    fn castle_path_clear_of_own_pieces(
        &self,
        color: Color,
        mut files: std::ops::RangeInclusive<u8>,
    ) -> bool {
        let rank = home_rank(color);
        files.all(|file| {
            !self.has_own_piece(
                Square::from_coords(file, rank).expect("valid square"),
                color,
            )
        })
    }

    fn has_own_piece(&self, square: Square, color: Color) -> bool {
        self.piece_at(square)
            .map(|piece| piece.color == color)
            .unwrap_or(false)
    }
}

fn offset(square: Square, df: i8, dr: i8) -> Option<Square> {
    let file = square.file() as i8 + df;
    let rank = square.rank() as i8 + dr;
    if (0..=7).contains(&file) && (0..=7).contains(&rank) {
        Square::from_coords(file as u8, rank as u8)
    } else {
        None
    }
}

fn pawn_dir(color: Color) -> i8 {
    match color {
        Color::White => 1,
        Color::Black => -1,
    }
}

fn pawn_start_rank(color: Color) -> u8 {
    match color {
        Color::White => 1,
        Color::Black => 6,
    }
}

fn pawn_promotion_rank(color: Color) -> u8 {
    match color {
        Color::White => 7,
        Color::Black => 0,
    }
}

fn home_rank(color: Color) -> u8 {
    match color {
        Color::White => 0,
        Color::Black => 7,
    }
}

fn add_promotion_moves(from: Square, to: Square, promotion_rank: u8, moves: &mut Vec<Move>) {
    if to.rank() == promotion_rank {
        for promotion in [
            PieceKind::Queen,
            PieceKind::Rook,
            PieceKind::Bishop,
            PieceKind::Knight,
        ] {
            moves.push(Move {
                from,
                to,
                promotion: Some(promotion),
            });
        }
    } else {
        moves.push(Move {
            from,
            to,
            promotion: None,
        });
    }
}

fn move_sort_key(mv: &Move) -> (u8, u8, u8) {
    (
        mv.from.index(),
        mv.to.index(),
        mv.promotion.map(piece_sort_key).unwrap_or(0),
    )
}

fn piece_sort_key(piece: PieceKind) -> u8 {
    match piece {
        PieceKind::Queen => 1,
        PieceKind::Rook => 2,
        PieceKind::Bishop => 3,
        PieceKind::Knight => 4,
        PieceKind::King => 5,
        PieceKind::Pawn => 6,
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

    #[test]
    fn starting_move_actions_include_pawn_capture_attempts() {
        let game = Game::new(GameConfig::default());
        let actions = game.move_actions();
        assert!(actions.contains(&Move {
            from: sq(0, 1),
            to: sq(1, 2),
            promotion: None,
        }));
        assert!(actions.contains(&Move {
            from: sq(1, 1),
            to: sq(0, 2),
            promotion: None,
        }));
        assert!(actions.contains(&Move {
            from: sq(1, 1),
            to: sq(2, 2),
            promotion: None,
        }));
    }

    #[test]
    fn pawn_forward_actions_ignore_unseen_opponent_piece() {
        let game =
            Game::from_fen("4k3/8/8/8/4p3/8/4P3/4K3 w - - 0 1", GameConfig::default()).unwrap();
        assert!(game.move_actions().contains(&Move {
            from: sq(4, 1),
            to: sq(4, 2),
            promotion: None,
        }));
        assert!(game.move_actions().contains(&Move {
            from: sq(4, 1),
            to: sq(4, 3),
            promotion: None,
        }));
    }

    #[test]
    fn slider_actions_ignore_unseen_opponent_piece() {
        let game =
            Game::from_fen("4k3/8/8/3R1p2/8/8/8/4K3 w - - 0 1", GameConfig::default()).unwrap();
        assert!(game.move_actions().contains(&Move {
            from: sq(3, 4),
            to: sq(7, 4),
            promotion: None,
        }));
    }
}

use crate::position::Position;
use crate::types::{
    Capture, Color, Error, GameConfig, GameResult, GameStatus, HistoryEntry, Move, MoveOutcome,
    MoveStatus, Piece, PieceKind, SenseResult, SensedSquare, Square, WinReason,
};
#[cfg(feature = "serde")]
use serde::{Deserialize, Deserializer, Serialize, Serializer};

const PROMOTION_PIECES: [PieceKind; 4] = [
    PieceKind::Queen,
    PieceKind::Rook,
    PieceKind::Bishop,
    PieceKind::Knight,
];

/// Reconnaissance Blind Chess game state.
#[derive(Clone, Debug)]
pub struct Game {
    position: Position,
    config: GameConfig,
    status: GameStatus,
    history: Vec<HistoryEntry>,
    pending_capture: [Option<Square>; 2],
    pending_sense: Option<SenseResult>,
}

#[cfg(feature = "serde")]
#[derive(Serialize, Deserialize)]
struct SerializedGame {
    fen: String,
    config: GameConfig,
    status: GameStatus,
    history: Vec<HistoryEntry>,
    pending_capture: [Option<Square>; 2],
    pending_sense: Option<SenseResult>,
}

#[cfg(feature = "serde")]
impl Serialize for Game {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        SerializedGame {
            fen: self.to_fen(),
            config: self.config.clone(),
            status: self.status.clone(),
            history: self.history.clone(),
            pending_capture: self.pending_capture,
            pending_sense: self.pending_sense.clone(),
        }
        .serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for Game {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let serialized = SerializedGame::deserialize(deserializer)?;
        let position = Position::from_fen(&serialized.fen).map_err(serde::de::Error::custom)?;
        Ok(Self {
            position,
            config: serialized.config,
            status: serialized.status,
            history: serialized.history,
            pending_capture: serialized.pending_capture,
            pending_sense: serialized.pending_sense,
        })
    }
}

impl Game {
    /// Creates a game in the standard chess starting position.
    #[must_use]
    pub fn new(config: GameConfig) -> Self {
        let position = Position::standard();
        Self {
            status: initial_status(&position, &config),
            position,
            config,
            history: Vec::new(),
            pending_capture: [None, None],
            pending_sense: None,
        }
    }

    /// Creates a game from a FEN string.
    pub fn from_fen(fen: &str, config: GameConfig) -> Result<Self, Error> {
        let position = Position::from_fen(fen)?;
        Ok(Self {
            status: initial_status(&position, &config),
            position,
            config,
            history: Vec::new(),
            pending_capture: [None, None],
            pending_sense: None,
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
    pub fn sense(&mut self, center: Option<Square>) -> SenseResult {
        let Some(center) = center else {
            let result = SenseResult {
                center: None,
                squares: Vec::new(),
            };
            self.pending_sense = Some(result.clone());
            return result;
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

        let result = SenseResult {
            center: Some(center),
            squares,
        };
        self.pending_sense = Some(result.clone());
        result
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
        for kind in [
            PieceKind::King,
            PieceKind::Queen,
            PieceKind::Rook,
            PieceKind::Bishop,
            PieceKind::Knight,
            PieceKind::Pawn,
        ] {
            let mut bb = self.position.piece_bitboard(turn, kind);
            while bb != 0 {
                let idx = bb.trailing_zeros() as u8;
                bb &= bb - 1;
                let from = Square::from_index(idx).expect("valid square");
                self.add_piece_move_actions(from, Piece { color: turn, kind }, &mut moves);
            }
        }
        moves.sort_by_key(move_sort_key);
        moves.dedup();
        moves
    }

    /// Applies a requested move or pass.
    pub fn apply_move(&mut self, requested: Option<Move>) -> Result<MoveOutcome, Error> {
        let color = self.turn().ok_or(Error::GameOver)?;
        let fen_before_move = self.to_fen();
        let Some(requested_move) = requested else {
            self.complete_turn_without_move(color);
            let outcome = MoveOutcome {
                requested: None,
                taken: None,
                status: MoveStatus::Pass,
                capture: None,
            };
            self.record_history(color, outcome.clone(), fen_before_move);
            return Ok(outcome);
        };

        let requested_move = self.add_pawn_queen_promotion(requested_move);
        if !self.is_move_action(requested_move) {
            return Err(Error::InvalidMove);
        }

        let Some(taken_move) = self.revise_move(requested_move) else {
            self.complete_turn_without_move(color);
            let outcome = MoveOutcome {
                requested,
                taken: None,
                status: MoveStatus::Illegal,
                capture: None,
            };
            self.record_history(color, outcome.clone(), fen_before_move);
            return Ok(outcome);
        };

        let moving_piece = self
            .piece_at(taken_move.from)
            .expect("validated move has moving piece");
        let capture = self.capture_for_move(taken_move, moving_piece);
        self.apply_taken_move(taken_move, moving_piece, capture);
        self.pending_capture[color.opposite().index()] = capture.map(|capture| capture.square);
        self.status = if capture.map(|capture| capture.piece.kind) == Some(PieceKind::King) {
            GameStatus::Won(GameResult {
                winner: color,
                reason: WinReason::KingCapture,
            })
        } else {
            self.status_after_non_winning_turn()
        };

        let outcome = MoveOutcome {
            requested,
            taken: Some(taken_move),
            status: if taken_move == requested_move {
                MoveStatus::Taken
            } else {
                MoveStatus::Revised
            },
            capture,
        };
        self.record_history(color, outcome.clone(), fen_before_move);
        Ok(outcome)
    }

    /// Records a resignation by the given color.
    pub fn resign(&mut self, color: Color) -> Result<GameResult, Error> {
        self.ensure_ongoing()?;
        let result = GameResult {
            winner: color.opposite(),
            reason: WinReason::Resignation,
        };
        self.status = GameStatus::Won(result);
        Ok(result)
    }

    /// Records a timeout for the given color.
    pub fn declare_timeout(&mut self, color: Color) -> Result<GameResult, Error> {
        self.ensure_ongoing()?;
        let result = GameResult {
            winner: color.opposite(),
            reason: WinReason::Timeout,
        };
        self.status = GameStatus::Won(result);
        Ok(result)
    }

    fn add_piece_move_actions(&self, from: Square, piece: Piece, moves: &mut Vec<Move>) {
        let own = self.position.occupied_by(piece.color);
        match piece.kind {
            PieceKind::Pawn => self.add_pawn_move_actions(from, piece.color, moves),
            PieceKind::Knight => {
                push_targets_from_bitboard(
                    from,
                    crate::attack_tables::KNIGHT_ATTACKS[from.index() as usize] & !own,
                    moves,
                );
            }
            PieceKind::Bishop => {
                push_targets_from_bitboard(
                    from,
                    crate::attack_tables::bishop_attacks(from.index(), own),
                    moves,
                );
            }
            PieceKind::Rook => {
                push_targets_from_bitboard(
                    from,
                    crate::attack_tables::rook_attacks(from.index(), own),
                    moves,
                );
            }
            PieceKind::Queen => {
                push_targets_from_bitboard(
                    from,
                    crate::attack_tables::queen_attacks(from.index(), own),
                    moves,
                );
            }
            PieceKind::King => {
                push_targets_from_bitboard(
                    from,
                    crate::attack_tables::KING_ATTACKS[from.index() as usize] & !own,
                    moves,
                );
                self.add_castling_move_actions(from, piece.color, moves);
            }
        }
    }

    fn add_pawn_move_actions(&self, from: Square, color: Color, moves: &mut Vec<Move>) {
        let dir = color.pawn_dir();
        let promotion_rank = color.pawn_promotion_rank();
        let own = self.position.occupied_by(color);
        let from_idx = from.index() as usize;

        // Forward push (single + optional double).
        let push_bb = crate::attack_tables::PAWN_SINGLE_PUSH[color.index()][from_idx];
        if push_bb != 0 && push_bb & own == 0 {
            let one_step =
                Square::from_index(push_bb.trailing_zeros() as u8).expect("valid square");
            add_promotion_moves(from, one_step, promotion_rank, moves);
            if from.rank() == color.pawn_start_rank() {
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

        // Diagonal captures (blind — emitted whether or not opponent is there).
        let mut captures = crate::attack_tables::PAWN_ATTACKS[color.index()][from_idx] & !own;
        while captures != 0 {
            let to_idx = captures.trailing_zeros() as u8;
            captures &= captures - 1;
            let to = Square::from_index(to_idx).expect("valid square");
            add_pawn_capture_moves(from, to, promotion_rank, moves);
        }
    }

    fn add_castling_move_actions(&self, from: Square, color: Color, moves: &mut Vec<Move>) {
        let home_rank = color.home_rank();
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

    fn is_move_action(&self, mv: Move) -> bool {
        let Some(turn) = self.turn() else {
            return false;
        };
        let Some(piece) = self.piece_at(mv.from) else {
            return false;
        };
        if piece.color != turn {
            return false;
        }

        // Pawn and king have promotion / castling rules that don't fit a
        // pure bitboard membership check; dispatch to dedicated helpers.
        match piece.kind {
            PieceKind::Pawn => return self.is_pawn_move_action(mv, piece.color),
            PieceKind::King => return self.is_king_move_action(mv, piece.color),
            _ => {}
        }

        if mv.promotion.is_some() {
            return false;
        }

        let from_idx = mv.from.index();
        let own = self.position.occupied_by(piece.color);
        let attacks = match piece.kind {
            PieceKind::Knight => crate::attack_tables::KNIGHT_ATTACKS[from_idx as usize] & !own,
            PieceKind::Bishop => crate::attack_tables::bishop_attacks(from_idx, own),
            PieceKind::Rook => crate::attack_tables::rook_attacks(from_idx, own),
            PieceKind::Queen => crate::attack_tables::queen_attacks(from_idx, own),
            PieceKind::Pawn | PieceKind::King => unreachable!("handled above"),
        };

        let to_bit = 1u64 << mv.to.index();
        attacks & to_bit != 0
    }

    fn is_pawn_move_action(&self, mv: Move, color: Color) -> bool {
        let dx = mv.to.file() as i8 - mv.from.file() as i8;
        let dy = mv.to.rank() as i8 - mv.from.rank() as i8;
        let dir = color.pawn_dir();
        let promotion_rank = color.pawn_promotion_rank();

        if dx == 0 && dy == dir {
            if self.has_own_piece(mv.to, color) {
                return false;
            }
            if mv.to.rank() == promotion_rank {
                matches!(
                    mv.promotion,
                    Some(
                        PieceKind::Queen | PieceKind::Rook | PieceKind::Bishop | PieceKind::Knight
                    )
                )
            } else {
                mv.promotion.is_none()
            }
        } else if dx == 0 && dy == 2 * dir && mv.from.rank() == color.pawn_start_rank() {
            if mv.promotion.is_some() {
                return false;
            }
            let Some(middle) = offset(mv.from, 0, dir) else {
                return false;
            };
            !self.has_own_piece(middle, color) && !self.has_own_piece(mv.to, color)
        } else if dx.abs() == 1 && dy == dir {
            if self.has_own_piece(mv.to, color) {
                return false;
            }
            if mv.to.rank() == promotion_rank {
                matches!(
                    mv.promotion,
                    None | Some(
                        PieceKind::Queen | PieceKind::Rook | PieceKind::Bishop | PieceKind::Knight
                    )
                )
            } else {
                mv.promotion.is_none()
            }
        } else {
            false
        }
    }

    fn is_king_move_action(&self, mv: Move, color: Color) -> bool {
        if mv.promotion.is_some() {
            return false;
        }
        let dx = mv.to.file() as i8 - mv.from.file() as i8;
        let dy = mv.to.rank() as i8 - mv.from.rank() as i8;

        if dy == 0 && dx.abs() == 2 {
            let home_rank = color.home_rank();
            if mv.from != Square::from_coords(4, home_rank).expect("valid square") {
                return false;
            }
            let rights = self.position.castling_rights();
            let (kingside, queenside) = match color {
                Color::White => (rights.white_kingside, rights.white_queenside),
                Color::Black => (rights.black_kingside, rights.black_queenside),
            };
            return if dx == 2 {
                kingside && self.castle_path_clear_of_own_pieces(color, 5..=6)
            } else {
                queenside && self.castle_path_clear_of_own_pieces(color, 1..=3)
            };
        }

        let from_idx = mv.from.index() as usize;
        let own = self.position.occupied_by(color);
        let attacks = crate::attack_tables::KING_ATTACKS[from_idx] & !own;
        let to_bit = 1u64 << mv.to.index();
        attacks & to_bit != 0
    }

    fn castle_path_clear_of_own_pieces(
        &self,
        color: Color,
        mut files: std::ops::RangeInclusive<u8>,
    ) -> bool {
        let rank = color.home_rank();
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

    fn add_pawn_queen_promotion(&self, requested: Move) -> Move {
        if requested.promotion.is_none()
            && self
                .piece_at(requested.from)
                .map(|piece| {
                    piece.kind == PieceKind::Pawn
                        && requested.to.rank() == piece.color.pawn_promotion_rank()
                })
                .unwrap_or(false)
        {
            return Move {
                promotion: Some(PieceKind::Queen),
                ..requested
            };
        }
        requested
    }

    fn revise_move(&self, requested: Move) -> Option<Move> {
        let moving_piece = self.piece_at(requested.from)?;
        match moving_piece.kind {
            PieceKind::Pawn => self.revise_pawn_move(requested, moving_piece.color),
            PieceKind::Knight => self.revise_knight_move(requested, moving_piece.color),
            PieceKind::Bishop | PieceKind::Rook | PieceKind::Queen => {
                self.revise_slider_move(requested, moving_piece)
            }
            PieceKind::King => self.revise_king_move(requested, moving_piece.color),
        }
    }

    fn revise_pawn_move(&self, mv: Move, color: Color) -> Option<Move> {
        let dx = mv.to.file() as i8 - mv.from.file() as i8;
        let dy = mv.to.rank() as i8 - mv.from.rank() as i8;
        let dir = color.pawn_dir();
        if dx == 0 && dy == dir && self.piece_at(mv.to).is_none() {
            return valid_promotion(mv);
        }
        if dx == 0 && dy == 2 * dir && mv.from.rank() == color.pawn_start_rank() {
            let middle = offset(mv.from, 0, dir)?;
            if self.piece_at(middle).is_none() && self.piece_at(mv.to).is_none() {
                return Some(mv);
            }
            if self.piece_at(middle).is_none()
                && self
                    .piece_at(mv.to)
                    .map(|piece| piece.color != color)
                    .unwrap_or(false)
            {
                return Some(Move {
                    from: mv.from,
                    to: middle,
                    promotion: None,
                });
            }
        }
        if dx.abs() == 1
            && dy == dir
            && (self
                .piece_at(mv.to)
                .map(|piece| piece.color != color)
                .unwrap_or(false)
                || self.is_en_passant_capture(mv, color))
        {
            return valid_promotion(mv);
        }
        None
    }

    fn revise_knight_move(&self, mv: Move, color: Color) -> Option<Move> {
        let dx = (mv.to.file() as i8 - mv.from.file() as i8).abs();
        let dy = (mv.to.rank() as i8 - mv.from.rank() as i8).abs();
        if ((dx == 1 && dy == 2) || (dx == 2 && dy == 1)) && !self.has_own_piece(mv.to, color) {
            Some(mv)
        } else {
            None
        }
    }

    fn revise_slider_move(&self, mv: Move, piece: Piece) -> Option<Move> {
        let (df, dr) = slider_direction(mv, piece.kind)?;
        let dir_idx = crate::attack_tables::direction_index(df, dr)?;
        let from_idx = mv.from.index() as usize;
        let to_idx = mv.to.index() as usize;
        let to_bit = 1u64 << to_idx;

        let ray = crate::attack_tables::RAY_FROM[dir_idx][from_idx];
        if ray & to_bit == 0 {
            return None;
        }

        let occupied = self.position.occupied();
        let blockers = ray & occupied;
        if blockers == 0 {
            return Some(mv);
        }

        let blocker_idx = crate::attack_tables::closest_blocker(blockers, dir_idx);
        let beyond_blocker =
            crate::attack_tables::RAY_FROM[dir_idx][blocker_idx] | (1u64 << blocker_idx);
        let reachable = ray & !beyond_blocker;

        if to_bit & reachable != 0 {
            return Some(mv);
        }

        let blocker_sq = Square::from_index(blocker_idx as u8).expect("valid square");
        let blocker_piece = self.piece_at(blocker_sq).expect("blocker bit was set");

        if blocker_piece.color == piece.color {
            return None;
        }

        if to_idx == blocker_idx {
            Some(mv)
        } else {
            Some(Move {
                to: blocker_sq,
                ..mv
            })
        }
    }

    fn revise_king_move(&self, mv: Move, color: Color) -> Option<Move> {
        let dx = (mv.to.file() as i8 - mv.from.file() as i8).abs();
        let dy = (mv.to.rank() as i8 - mv.from.rank() as i8).abs();
        if dx == 2 && dy == 0 {
            return self.revise_castling_move(mv, color);
        }
        if dx <= 1 && dy <= 1 && !self.has_own_piece(mv.to, color) {
            Some(mv)
        } else {
            None
        }
    }

    fn revise_castling_move(&self, mv: Move, color: Color) -> Option<Move> {
        let home_rank = color.home_rank();
        if mv.from != Square::from_coords(4, home_rank).expect("valid square") {
            return None;
        }
        let kingside = mv.to.file() == 6;
        let queenside = mv.to.file() == 2;
        if !kingside && !queenside {
            return None;
        }

        let rights = self.position.castling_rights();
        let allowed = match (color, kingside) {
            (Color::White, true) => rights.white_kingside,
            (Color::White, false) => rights.white_queenside,
            (Color::Black, true) => rights.black_kingside,
            (Color::Black, false) => rights.black_queenside,
        };
        if !allowed {
            return None;
        }

        let rook_file = if kingside { 7 } else { 0 };
        let rook_square = Square::from_coords(rook_file, home_rank).expect("valid square");
        if self.piece_at(rook_square)
            != Some(Piece {
                color,
                kind: PieceKind::Rook,
            })
        {
            return None;
        }

        let between = if kingside { 5..=6 } else { 1..=3 };
        for file in between {
            if self
                .piece_at(Square::from_coords(file, home_rank).expect("valid square"))
                .is_some()
            {
                return None;
            }
        }
        Some(mv)
    }

    fn capture_for_move(&self, mv: Move, moving_piece: Piece) -> Option<Capture> {
        if self.is_en_passant_capture(mv, moving_piece.color) {
            let capture_square = offset(mv.to, 0, -moving_piece.color.pawn_dir())?;
            let piece = self.piece_at(capture_square)?;
            return Some(Capture {
                square: capture_square,
                piece,
            });
        }

        let piece = self.piece_at(mv.to)?;
        Some(Capture {
            square: mv.to,
            piece,
        })
    }

    fn is_en_passant_capture(&self, mv: Move, color: Color) -> bool {
        let Some(moving_piece) = self.piece_at(mv.from) else {
            return false;
        };
        if moving_piece.kind != PieceKind::Pawn
            || mv.from.file().abs_diff(mv.to.file()) != 1
            || self.position.en_passant() != Some(mv.to)
        {
            return false;
        }
        let Some(capture_square) = offset(mv.to, 0, -color.pawn_dir()) else {
            return false;
        };
        self.piece_at(capture_square)
            == Some(Piece {
                color: color.opposite(),
                kind: PieceKind::Pawn,
            })
    }

    fn apply_taken_move(&mut self, mv: Move, moving_piece: Piece, capture: Option<Capture>) {
        if let Some(capture) = capture {
            self.position.remove_piece(capture.square);
            if capture.piece.kind == PieceKind::Rook {
                self.position
                    .disable_rook_castling_right(capture.piece.color, capture.square);
            }
        }

        self.position.remove_piece(mv.from);
        let placed_piece = Piece {
            kind: mv.promotion.unwrap_or(moving_piece.kind),
            ..moving_piece
        };
        self.position.set_piece(mv.to, Some(placed_piece));

        if is_castling_move(mv, moving_piece) {
            let rank = moving_piece.color.home_rank();
            let (rook_from_file, rook_to_file) = if mv.to.file() == 6 { (7, 5) } else { (0, 3) };
            let rook_from = Square::from_coords(rook_from_file, rank).expect("valid square");
            let rook_to = Square::from_coords(rook_to_file, rank).expect("valid square");
            let rook = self.position.remove_piece(rook_from);
            self.position.set_piece(rook_to, rook);
        }

        if moving_piece.kind == PieceKind::King {
            self.position.disable_castling_for_color(moving_piece.color);
        }
        if moving_piece.kind == PieceKind::Rook {
            self.position
                .disable_rook_castling_right(moving_piece.color, mv.from);
        }

        let en_passant = if moving_piece.kind == PieceKind::Pawn
            && mv.from.file() == mv.to.file()
            && mv.from.rank().abs_diff(mv.to.rank()) == 2
        {
            offset(mv.from, 0, moving_piece.color.pawn_dir())
        } else {
            None
        };
        self.position.finish_move(
            moving_piece,
            capture.map(|capture| capture.piece),
            en_passant,
        );
    }

    fn ensure_ongoing(&self) -> Result<(), Error> {
        match self.status {
            GameStatus::Ongoing { .. } => Ok(()),
            GameStatus::Won(_) | GameStatus::Draw { .. } => Err(Error::GameOver),
        }
    }

    fn update_status_after_turn(&mut self) {
        self.status = self.status_after_non_winning_turn();
    }

    fn complete_turn_without_move(&mut self, color: Color) {
        self.position.null_move();
        self.update_status_after_turn();
        self.pending_capture[color.opposite().index()] = None;
    }

    fn status_after_non_winning_turn(&self) -> GameStatus {
        if self
            .config
            .full_turn_limit
            .map(|limit| self.position.fullmove_number() > limit)
            .unwrap_or(false)
        {
            return GameStatus::Draw {
                reason: crate::types::DrawReason::TurnLimit,
            };
        }
        if self
            .config
            .reversible_moves_limit
            .map(|limit| self.position.halfmove_clock() >= limit)
            .unwrap_or(false)
        {
            return GameStatus::Draw {
                reason: crate::types::DrawReason::MoveLimit,
            };
        }
        GameStatus::Ongoing {
            turn: self.position.turn(),
        }
    }

    fn record_history(&mut self, color: Color, move_outcome: MoveOutcome, fen_before_move: String) {
        let sense = self.pending_sense.take().unwrap_or(SenseResult {
            center: None,
            squares: Vec::new(),
        });
        self.history.push(HistoryEntry {
            color,
            sense,
            move_outcome,
            fen_before_move,
            fen_after_move: self.to_fen(),
        });
    }
}

fn push_targets_from_bitboard(from: Square, mut bb: u64, moves: &mut Vec<Move>) {
    while bb != 0 {
        let to_idx = bb.trailing_zeros() as u8;
        bb &= bb - 1;
        moves.push(Move {
            from,
            to: Square::from_index(to_idx).expect("valid square"),
            promotion: None,
        });
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

fn add_promotion_moves(from: Square, to: Square, promotion_rank: u8, moves: &mut Vec<Move>) {
    if to.rank() == promotion_rank {
        for promotion in PROMOTION_PIECES {
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

fn add_pawn_capture_moves(from: Square, to: Square, promotion_rank: u8, moves: &mut Vec<Move>) {
    moves.push(Move {
        from,
        to,
        promotion: None,
    });
    if to.rank() == promotion_rank {
        for promotion in PROMOTION_PIECES {
            moves.push(Move {
                from,
                to,
                promotion: Some(promotion),
            });
        }
    }
}

fn valid_promotion(mv: Move) -> Option<Move> {
    match mv.promotion {
        None | Some(PieceKind::Queen | PieceKind::Rook | PieceKind::Bishop | PieceKind::Knight) => {
            Some(mv)
        }
        Some(PieceKind::King | PieceKind::Pawn) => None,
    }
}

fn slider_direction(mv: Move, kind: PieceKind) -> Option<(i8, i8)> {
    let dx = mv.to.file() as i8 - mv.from.file() as i8;
    let dy = mv.to.rank() as i8 - mv.from.rank() as i8;
    let direction = match kind {
        PieceKind::Rook if dx == 0 || dy == 0 => (dx.signum(), dy.signum()),
        PieceKind::Bishop if dx.abs() == dy.abs() => (dx.signum(), dy.signum()),
        PieceKind::Queen if dx == 0 || dy == 0 || dx.abs() == dy.abs() => {
            (dx.signum(), dy.signum())
        }
        _ => return None,
    };
    if direction == (0, 0) {
        None
    } else {
        Some(direction)
    }
}

fn is_castling_move(mv: Move, moving_piece: Piece) -> bool {
    moving_piece.kind == PieceKind::King
        && mv.from.rank() == mv.to.rank()
        && mv.from.file().abs_diff(mv.to.file()) == 2
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

fn initial_status(position: &Position, config: &GameConfig) -> GameStatus {
    if !position.has_king(Color::White) {
        return GameStatus::Won(GameResult {
            winner: Color::Black,
            reason: WinReason::KingCapture,
        });
    }
    if !position.has_king(Color::Black) {
        return GameStatus::Won(GameResult {
            winner: Color::White,
            reason: WinReason::KingCapture,
        });
    }
    if config
        .full_turn_limit
        .map(|limit| position.fullmove_number() > limit)
        .unwrap_or(false)
    {
        return GameStatus::Draw {
            reason: crate::types::DrawReason::TurnLimit,
        };
    }
    if config
        .reversible_moves_limit
        .map(|limit| position.halfmove_clock() >= limit)
        .unwrap_or(false)
    {
        return GameStatus::Draw {
            reason: crate::types::DrawReason::MoveLimit,
        };
    }
    GameStatus::Ongoing {
        turn: position.turn(),
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
        let mut game = Game::new(GameConfig::default());
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
        let mut game = Game::new(GameConfig::default());
        let result = game.sense(Some(sq(0, 7)));
        let squares: Vec<Square> = result.squares.iter().map(|entry| entry.square).collect();
        assert_eq!(squares, vec![sq(0, 7), sq(1, 7), sq(0, 6), sq(1, 6)]);
    }

    #[test]
    fn pass_sense_returns_empty_result() {
        let mut game = Game::new(GameConfig::default());
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

    #[test]
    fn pass_move_flips_turn() {
        let mut game = Game::new(GameConfig::default());
        let outcome = game.apply_move(None).unwrap();
        assert_eq!(outcome.status, MoveStatus::Pass);
        assert_eq!(
            game.to_fen(),
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 1 1"
        );
    }

    #[test]
    fn illegal_truth_move_consumes_turn() {
        let mut game = Game::new(GameConfig::default());
        let outcome = game
            .apply_move(Some(Move {
                from: sq(4, 1),
                to: sq(5, 2),
                promotion: None,
            }))
            .unwrap();
        assert_eq!(outcome.status, MoveStatus::Illegal);
        assert_eq!(
            game.to_fen(),
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 1 1"
        );
    }

    #[test]
    fn slider_move_revises_to_first_opponent_piece() {
        let mut game = Game::from_fen(
            "4k3/3p4/8/1p1R1p2/8/8/8/4K3 w - - 0 1",
            GameConfig::default(),
        )
        .unwrap();
        let outcome = game
            .apply_move(Some(Move {
                from: sq(3, 4),
                to: sq(7, 4),
                promotion: None,
            }))
            .unwrap();
        assert_eq!(outcome.status, MoveStatus::Revised);
        assert_eq!(
            outcome.taken,
            Some(Move {
                from: sq(3, 4),
                to: sq(5, 4),
                promotion: None,
            })
        );
        assert_eq!(outcome.capture.unwrap().square, sq(5, 4));
    }

    #[test]
    fn own_piece_blocker_is_invalid_request() {
        let mut game =
            Game::from_fen("4k3/8/8/3R4/8/8/3P4/4K3 w - - 0 1", GameConfig::default()).unwrap();
        let result = game.apply_move(Some(Move {
            from: sq(3, 4),
            to: sq(3, 0),
            promotion: None,
        }));
        assert_eq!(result, Err(Error::InvalidMove));
    }

    #[test]
    fn pawn_auto_promotes_to_queen_when_omitted() {
        let mut game =
            Game::from_fen("7k/3P4/8/8/8/8/8/4K3 w - - 0 1", GameConfig::default()).unwrap();
        let outcome = game
            .apply_move(Some(Move {
                from: sq(3, 6),
                to: sq(3, 7),
                promotion: None,
            }))
            .unwrap();
        assert_eq!(
            outcome.taken,
            Some(Move {
                from: sq(3, 6),
                to: sq(3, 7),
                promotion: Some(PieceKind::Queen),
            })
        );
        assert_eq!(
            game.piece_at(sq(3, 7)),
            Some(Piece {
                color: Color::White,
                kind: PieceKind::Queen,
            })
        );
    }

    #[test]
    fn promotion_capture_actions_include_omitted_request() {
        let game =
            Game::from_fen("1r5k/P7/8/8/8/8/8/4K3 w - - 0 1", GameConfig::default()).unwrap();
        assert!(game.move_actions().contains(&Move {
            from: sq(0, 6),
            to: sq(1, 7),
            promotion: None,
        }));
    }

    #[test]
    fn en_passant_reports_captured_pawn_square() {
        let mut game =
            Game::from_fen("4k3/8/8/8/1p6/8/P7/4K3 w - - 0 1", GameConfig::default()).unwrap();
        game.apply_move(Some(Move {
            from: sq(0, 1),
            to: sq(0, 3),
            promotion: None,
        }))
        .unwrap();
        let outcome = game
            .apply_move(Some(Move {
                from: sq(1, 3),
                to: sq(0, 2),
                promotion: None,
            }))
            .unwrap();
        assert_eq!(outcome.capture.unwrap().square, sq(0, 3));
    }

    #[test]
    fn castling_ignores_check_but_not_between_pieces() {
        let mut game =
            Game::from_fen("4k3/8/8/8/8/8/8/4K2R w K - 0 1", GameConfig::default()).unwrap();
        game.position.set_piece(
            sq(6, 3),
            Some(Piece {
                color: Color::Black,
                kind: PieceKind::Queen,
            }),
        );
        let outcome = game
            .apply_move(Some(Move {
                from: sq(4, 0),
                to: sq(6, 0),
                promotion: None,
            }))
            .unwrap();
        assert_eq!(outcome.status, MoveStatus::Taken);
        assert_eq!(game.to_fen(), "4k3/8/8/8/6q1/8/8/5RK1 b - - 1 1");

        let mut blocked =
            Game::from_fen("4k3/8/8/8/8/8/8/4K2R w K - 0 1", GameConfig::default()).unwrap();
        blocked.position.set_piece(
            sq(6, 0),
            Some(Piece {
                color: Color::Black,
                kind: PieceKind::Knight,
            }),
        );
        let outcome = blocked
            .apply_move(Some(Move {
                from: sq(4, 0),
                to: sq(6, 0),
                promotion: None,
            }))
            .unwrap();
        assert_eq!(outcome.status, MoveStatus::Illegal);
    }

    #[test]
    fn king_capture_ends_game() {
        let mut game =
            Game::from_fen("4k3/8/8/8/8/8/8/4K3 w - - 0 1", GameConfig::default()).unwrap();
        game.position.set_piece(
            sq(4, 6),
            Some(Piece {
                color: Color::White,
                kind: PieceKind::Queen,
            }),
        );
        let outcome = game
            .apply_move(Some(Move {
                from: sq(4, 6),
                to: sq(4, 7),
                promotion: None,
            }))
            .unwrap();
        assert_eq!(outcome.capture.unwrap().piece.kind, PieceKind::King);
        assert_eq!(
            game.status(),
            &GameStatus::Won(GameResult {
                winner: Color::White,
                reason: WinReason::KingCapture,
            })
        );
        assert_eq!(game.to_fen(), "4Q3/8/8/8/8/8/8/4K3 b - - 0 1");
    }

    #[test]
    fn reversible_move_limit_causes_draw() {
        let mut game = Game::from_fen(
            "4k3/8/8/8/8/8/8/4K3 w - - 0 1",
            GameConfig {
                reversible_moves_limit: Some(2),
                full_turn_limit: None,
            },
        )
        .unwrap();
        game.apply_move(None).unwrap();
        assert_eq!(game.status(), &GameStatus::Ongoing { turn: Color::Black });
        game.apply_move(None).unwrap();
        assert_eq!(
            game.status(),
            &GameStatus::Draw {
                reason: crate::types::DrawReason::MoveLimit,
            }
        );
    }

    #[test]
    fn full_turn_limit_causes_draw_after_black_turn() {
        let mut game = Game::from_fen(
            "4k3/8/8/8/8/8/8/4K3 w - - 0 1",
            GameConfig {
                reversible_moves_limit: None,
                full_turn_limit: Some(1),
            },
        )
        .unwrap();
        game.apply_move(None).unwrap();
        assert_eq!(game.status(), &GameStatus::Ongoing { turn: Color::Black });
        game.apply_move(None).unwrap();
        assert_eq!(
            game.status(),
            &GameStatus::Draw {
                reason: crate::types::DrawReason::TurnLimit,
            }
        );
    }

    #[test]
    fn resign_and_timeout_set_winner() {
        let mut resigned = Game::new(GameConfig::default());
        assert_eq!(
            resigned.resign(Color::White).unwrap(),
            GameResult {
                winner: Color::Black,
                reason: WinReason::Resignation,
            }
        );

        let mut timed_out = Game::new(GameConfig::default());
        assert_eq!(
            timed_out.declare_timeout(Color::Black).unwrap(),
            GameResult {
                winner: Color::White,
                reason: WinReason::Timeout,
            }
        );
    }

    #[test]
    fn completed_game_rejects_further_actions() {
        let mut game = Game::new(GameConfig::default());
        game.resign(Color::White).unwrap();
        assert_eq!(game.apply_move(None), Err(Error::GameOver));
        assert_eq!(game.resign(Color::Black), Err(Error::GameOver));
    }

    #[test]
    fn constructors_apply_terminal_position_status() {
        let no_white_king =
            Game::from_fen("4k3/8/8/8/8/8/8/8 w - - 0 1", GameConfig::default()).unwrap();
        assert_eq!(
            no_white_king.status(),
            &GameStatus::Won(GameResult {
                winner: Color::Black,
                reason: WinReason::KingCapture,
            })
        );

        let move_limit = Game::new(GameConfig {
            reversible_moves_limit: Some(0),
            full_turn_limit: None,
        });
        assert_eq!(
            move_limit.status(),
            &GameStatus::Draw {
                reason: crate::types::DrawReason::MoveLimit,
            }
        );

        let turn_limit = Game::from_fen(
            "4k3/8/8/8/8/8/8/4K3 w - - 0 2",
            GameConfig {
                reversible_moves_limit: None,
                full_turn_limit: Some(1),
            },
        )
        .unwrap();
        assert_eq!(
            turn_limit.status(),
            &GameStatus::Draw {
                reason: crate::types::DrawReason::TurnLimit,
            }
        );
    }

    #[test]
    fn history_records_sense_and_move_outcome() {
        let mut game = Game::new(GameConfig::default());
        let sensed = game.sense(Some(sq(1, 1)));
        let outcome = game
            .apply_move(Some(Move {
                from: sq(4, 1),
                to: sq(4, 3),
                promotion: None,
            }))
            .unwrap();
        let entry = &game.history()[0];
        assert_eq!(entry.color, Color::White);
        assert_eq!(entry.sense, sensed);
        assert_eq!(entry.move_outcome, outcome);
        assert_eq!(
            entry.fen_before_move,
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"
        );
        assert_eq!(
            entry.fen_after_move,
            "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1"
        );
    }

    #[test]
    fn history_defaults_to_pass_sense_when_none_recorded() {
        let mut game = Game::new(GameConfig::default());
        game.apply_move(None).unwrap();
        assert_eq!(game.history()[0].sense.center, None);
        assert!(game.history()[0].sense.squares.is_empty());
    }

    #[cfg(feature = "serde")]
    #[test]
    fn game_serializes_with_history() {
        let mut game = Game::new(GameConfig::default());
        let _ = game.sense(None);
        game.apply_move(None).unwrap();
        let json = serde_json::to_string(&game).unwrap();
        let decoded: Game = serde_json::from_str(&json).unwrap();
        assert!(json.contains("\"history\""));
        assert!(json.contains("\"Pass\""));
        assert_eq!(decoded.to_fen(), game.to_fen());
        assert_eq!(decoded.status(), game.status());
        assert_eq!(decoded.history(), game.history());
    }
}

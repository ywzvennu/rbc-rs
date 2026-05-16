#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Player color.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum Color {
    /// White moves first.
    White,
    /// Black moves second.
    Black,
}

impl Color {
    /// Returns the opposite color.
    #[must_use]
    pub const fn opposite(self) -> Self {
        match self {
            Self::White => Self::Black,
            Self::Black => Self::White,
        }
    }

    /// Returns a stable array index for the color.
    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Self::White => 0,
            Self::Black => 1,
        }
    }

    pub(crate) const fn pawn_dir(self) -> i8 {
        match self {
            Self::White => 1,
            Self::Black => -1,
        }
    }

    pub(crate) const fn pawn_start_rank(self) -> u8 {
        match self {
            Self::White => 1,
            Self::Black => 6,
        }
    }

    pub(crate) const fn pawn_promotion_rank(self) -> u8 {
        match self {
            Self::White => 7,
            Self::Black => 0,
        }
    }

    pub(crate) const fn home_rank(self) -> u8 {
        match self {
            Self::White => 0,
            Self::Black => 7,
        }
    }
}

/// A board square, indexed from `a1 = 0` to `h8 = 63`.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Square(u8);

impl Square {
    /// Creates a square from zero-based file and rank coordinates.
    #[must_use]
    pub const fn from_coords(file: u8, rank: u8) -> Option<Self> {
        if file < 8 && rank < 8 {
            Some(Self(rank * 8 + file))
        } else {
            None
        }
    }

    /// Creates a square from a zero-based index.
    #[must_use]
    pub const fn from_index(index: u8) -> Option<Self> {
        if index < 64 {
            Some(Self(index))
        } else {
            None
        }
    }

    /// Returns the zero-based square index.
    #[must_use]
    pub const fn index(self) -> u8 {
        self.0
    }

    /// Returns the zero-based file.
    #[must_use]
    pub const fn file(self) -> u8 {
        self.0 % 8
    }

    /// Returns the zero-based rank.
    #[must_use]
    pub const fn rank(self) -> u8 {
        self.0 / 8
    }

    /// Returns the algebraic coordinate, such as `e4`.
    #[must_use]
    pub fn to_algebraic(self) -> String {
        let file = char::from(b'a' + self.file());
        let rank = char::from(b'1' + self.rank());
        format!("{file}{rank}")
    }
}

impl std::fmt::Display for Square {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_algebraic())
    }
}

/// A chess piece kind.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum PieceKind {
    /// King.
    King,
    /// Queen.
    Queen,
    /// Rook.
    Rook,
    /// Bishop.
    Bishop,
    /// Knight.
    Knight,
    /// Pawn.
    Pawn,
}

impl PieceKind {
    pub(crate) const fn index(self) -> usize {
        match self {
            Self::King => 0,
            Self::Queen => 1,
            Self::Rook => 2,
            Self::Bishop => 3,
            Self::Knight => 4,
            Self::Pawn => 5,
        }
    }

    pub(crate) const fn from_index(idx: usize) -> Self {
        match idx {
            0 => Self::King,
            1 => Self::Queen,
            2 => Self::Rook,
            3 => Self::Bishop,
            4 => Self::Knight,
            5 => Self::Pawn,
            _ => panic!("invalid piece kind index"),
        }
    }
}

/// A colored chess piece.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct Piece {
    /// Piece color.
    pub color: Color,
    /// Piece kind.
    pub kind: PieceKind,
}

/// A chess move request.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct Move {
    /// Origin square.
    pub from: Square,
    /// Destination square.
    pub to: Square,
    /// Optional promotion piece.
    pub promotion: Option<PieceKind>,
}

/// Static game configuration.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GameConfig {
    /// Maximum half-moves without a pawn move or capture before a draw.
    pub reversible_moves_limit: Option<u16>,
    /// Maximum full turns before a draw.
    pub full_turn_limit: Option<u16>,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            reversible_moves_limit: Some(100),
            full_turn_limit: None,
        }
    }
}

/// A sensed square and its current piece, if any.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct SensedSquare {
    /// Square included in the sense result.
    pub square: Square,
    /// Piece on the square.
    pub piece: Option<Piece>,
}

/// Result of a sense action.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SenseResult {
    /// Requested center square. `None` represents a pass sense.
    pub center: Option<Square>,
    /// Sensed squares in rank-descending, file-ascending order.
    pub squares: Vec<SensedSquare>,
}

/// Information about a capture.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct Capture {
    /// Captured square.
    pub square: Square,
    /// Captured piece.
    pub piece: Piece,
}

/// Move execution status.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum MoveStatus {
    /// The requested move was taken unchanged.
    Taken,
    /// The requested move was revised before being taken.
    Revised,
    /// The requested move was illegal and consumed the turn.
    Illegal,
    /// The player passed the move phase.
    Pass,
}

/// Result of a move action.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MoveOutcome {
    /// Requested move, if any.
    pub requested: Option<Move>,
    /// Move that was actually applied, if any.
    pub taken: Option<Move>,
    /// Move status.
    pub status: MoveStatus,
    /// Capture information, if a piece was captured.
    pub capture: Option<Capture>,
}

/// Game status.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GameStatus {
    /// Game is still running.
    Ongoing {
        /// Color to move.
        turn: Color,
    },
    /// Game ended with a winner.
    Won(GameResult),
    /// Game ended in a draw.
    Draw {
        /// Reason the game was adjudicated as drawn.
        reason: DrawReason,
    },
}

/// Winning result.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct GameResult {
    /// Winning color.
    pub winner: Color,
    /// Win reason.
    pub reason: WinReason,
}

/// Win reason.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum WinReason {
    /// Opponent king was captured.
    KingCapture,
    /// Opponent resigned.
    Resignation,
    /// Opponent lost on time. Clocks are external to this crate.
    Timeout,
}

/// Draw reason.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum DrawReason {
    /// Reversible move limit was reached.
    MoveLimit,
    /// Full turn limit was reached.
    TurnLimit,
}

/// History entry for a completed player turn.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryEntry {
    /// Acting color.
    pub color: Color,
    /// Sense result.
    pub sense: SenseResult,
    /// Move outcome.
    pub move_outcome: MoveOutcome,
    /// FEN before the move.
    pub fen_before_move: String,
    /// FEN after the move.
    pub fen_after_move: String,
}

/// Library error type.
///
/// Marked `#[non_exhaustive]` so future variant additions are
/// non-breaking. Downstream `match` arms should include a `_ => ...`
/// catch-all.
#[non_exhaustive]
#[derive(Debug, Error, Eq, PartialEq)]
pub enum Error {
    /// The game has already ended.
    #[error("game is over")]
    GameOver,
    /// The requested move is invalid for move generation.
    #[error("invalid move")]
    InvalidMove,
    /// FEN parsing failed.
    #[error("invalid FEN: {0}")]
    InvalidFen(String),
}

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

/// Starting-position variant for the game.
///
/// `Standard` (the default) preserves classical FIDE chess. The
/// shuffle variants draw a back-rank arrangement from
/// [`chess_startpos_rs`]:
///
/// - `Mirrored` — both sides start with the same arrangement (FIDE
///   convention; the rank-8 setup mirrors rank-1).
/// - `Independent` — the two sides draw arrangements independently
///   from the same [`chess_startpos_rs::Problem`]. RBC-flavoured;
///   removes the ability to infer the opponent's setup from your own.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub enum Variant {
    /// Classical FIDE chess starting position. Default.
    #[default]
    Standard,
    /// Both sides start with the same back-rank arrangement, drawn at
    /// the given lexicographic index from `problem`.
    Mirrored {
        /// The constraint problem defining the starting-position
        /// alphabet (typically one of the `chess_startpos_rs::chess`
        /// presets).
        problem: chess_startpos_rs::Problem<chess_startpos_rs::chess::Piece>,
        /// Lexicographic index into the problem's solution set.
        index: u64,
    },
    /// White and black draw their back-rank arrangements
    /// independently from the same problem.
    Independent {
        /// The constraint problem defining the starting-position
        /// alphabet.
        problem: chess_startpos_rs::Problem<chess_startpos_rs::chess::Piece>,
        /// Lexicographic index for white's rank-1 arrangement.
        white_index: u64,
        /// Lexicographic index for black's rank-8 arrangement.
        black_index: u64,
    },
}

/// Per-side, per-direction castling-right toggles.
///
/// Applied as an intersection with the structural castling rights
/// derived from the chosen back-rank arrangement: a side that
/// doesn't have a rook on the relevant flank cannot castle that way
/// regardless of policy.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct CastlingPolicy {
    /// White is allowed to castle kingside (h-side).
    pub white_kingside: bool,
    /// White is allowed to castle queenside (a-side).
    pub white_queenside: bool,
    /// Black is allowed to castle kingside (h-side).
    pub black_kingside: bool,
    /// Black is allowed to castle queenside (a-side).
    pub black_queenside: bool,
}

impl Default for CastlingPolicy {
    fn default() -> Self {
        Self {
            white_kingside: true,
            white_queenside: true,
            black_kingside: true,
            black_queenside: true,
        }
    }
}

/// Static game configuration.
///
/// Marked `#[non_exhaustive]` so future fields can be added without a
/// semver break — construct via [`Default::default`] and spread or
/// mutate as needed. Does not derive `Eq` / `PartialEq` because
/// [`Variant`] embeds a [`chess_startpos_rs::Problem`] which does not
/// (constraint-tree equality is not currently a defined operation
/// upstream).
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct GameConfig {
    /// Maximum half-moves without a pawn move or capture before a draw.
    pub reversible_moves_limit: Option<u16>,
    /// Maximum full turns before a draw.
    pub full_turn_limit: Option<u16>,
    /// Starting-position variant. Defaults to [`Variant::Standard`].
    pub variant: Variant,
    /// Per-side, per-direction castling-right toggles, intersected
    /// with the structural rights derived from the chosen back rank.
    /// Defaults to all four directions allowed.
    pub castling_policy: CastlingPolicy,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            reversible_moves_limit: Some(100),
            full_turn_limit: None,
            variant: Variant::default(),
            castling_policy: CastlingPolicy::default(),
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

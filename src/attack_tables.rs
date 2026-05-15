//! Precomputed attack tables.
//!
//! All tables are `const`-computed at compile time so there is no runtime
//! initialization cost. The slider helpers use direction-ray tables plus a
//! blocker-subtract trick — not magic bitboards, but O(1) per direction
//! instead of walking square-by-square.
//!
//! For the slider helpers, "blockers" should be the side-to-move's own
//! occupancy: in RBC, opponent pieces are transparent for blind move
//! generation. The returned bitboard is the set of squares reachable
//! along each ray up to but not including the first own piece, mirroring
//! `Game::add_ray_move_actions`.

// Direction index layout. Positive directions (lowest set bit = closest)
// come first; negative directions (highest set bit = closest) come second.
const E: usize = 0;
const N: usize = 1;
const NE: usize = 2;
const NW: usize = 3;
const W: usize = 4;
const S: usize = 5;
const SE: usize = 6;
const SW: usize = 7;

const DIRECTIONS: [(i8, i8); 8] = [
    (1, 0),   // E
    (0, 1),   // N
    (1, 1),   // NE
    (-1, 1),  // NW
    (-1, 0),  // W
    (0, -1),  // S
    (1, -1),  // SE
    (-1, -1), // SW
];

const fn step_attacks(offsets: &[(i8, i8)]) -> [u64; 64] {
    let mut table = [0u64; 64];
    let mut sq = 0usize;
    while sq < 64 {
        let file = (sq % 8) as i8;
        let rank = (sq / 8) as i8;
        let mut i = 0;
        while i < offsets.len() {
            let (df, dr) = offsets[i];
            let nf = file + df;
            let nr = rank + dr;
            if nf >= 0 && nf < 8 && nr >= 0 && nr < 8 {
                table[sq] |= 1u64 << ((nr * 8 + nf) as usize);
            }
            i += 1;
        }
        sq += 1;
    }
    table
}

const KNIGHT_OFFSETS: [(i8, i8); 8] = [
    (1, 2),
    (2, 1),
    (2, -1),
    (1, -2),
    (-1, -2),
    (-2, -1),
    (-2, 1),
    (-1, 2),
];

const KING_OFFSETS: [(i8, i8); 8] = [
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
    (-1, -1),
    (0, -1),
    (1, -1),
];

pub(crate) const KNIGHT_ATTACKS: [u64; 64] = step_attacks(&KNIGHT_OFFSETS);
pub(crate) const KING_ATTACKS: [u64; 64] = step_attacks(&KING_OFFSETS);

const fn pawn_attacks_for(color_idx: usize) -> [u64; 64] {
    let mut table = [0u64; 64];
    let dir: i8 = if color_idx == 0 { 1 } else { -1 };
    let mut sq = 0usize;
    while sq < 64 {
        let file = (sq % 8) as i8;
        let rank = (sq / 8) as i8;
        let nr = rank + dir;
        if nr >= 0 && nr < 8 {
            let lf = file - 1;
            if lf >= 0 {
                table[sq] |= 1u64 << ((nr * 8 + lf) as usize);
            }
            let rf = file + 1;
            if rf < 8 {
                table[sq] |= 1u64 << ((nr * 8 + rf) as usize);
            }
        }
        sq += 1;
    }
    table
}

pub(crate) const PAWN_ATTACKS: [[u64; 64]; 2] = [pawn_attacks_for(0), pawn_attacks_for(1)];

const fn pawn_single_push_for(color_idx: usize) -> [u64; 64] {
    let mut table = [0u64; 64];
    let dir: i8 = if color_idx == 0 { 1 } else { -1 };
    let mut sq = 0usize;
    while sq < 64 {
        let rank = (sq / 8) as i8;
        let nr = rank + dir;
        if nr >= 0 && nr < 8 {
            table[sq] = 1u64 << (sq as isize + dir as isize * 8) as usize;
        }
        sq += 1;
    }
    table
}

pub(crate) const PAWN_SINGLE_PUSH: [[u64; 64]; 2] =
    [pawn_single_push_for(0), pawn_single_push_for(1)];

const fn ray_for(dir_idx: usize) -> [u64; 64] {
    let mut table = [0u64; 64];
    let (df, dr) = DIRECTIONS[dir_idx];
    let mut sq = 0usize;
    while sq < 64 {
        let file = (sq % 8) as i8;
        let rank = (sq / 8) as i8;
        let mut step: i8 = 1;
        while step <= 7 {
            let nf = file + df * step;
            let nr = rank + dr * step;
            if nf < 0 || nf >= 8 || nr < 0 || nr >= 8 {
                break;
            }
            table[sq] |= 1u64 << ((nr * 8 + nf) as usize);
            step += 1;
        }
        sq += 1;
    }
    table
}

pub(crate) const RAY_FROM: [[u64; 64]; 8] = [
    ray_for(0),
    ray_for(1),
    ray_for(2),
    ray_for(3),
    ray_for(4),
    ray_for(5),
    ray_for(6),
    ray_for(7),
];

/// Whether moving in `dir_idx` increases the linear square index.
/// E, N, NE, NW are positive (closest blocker = lowest set bit).
/// W, S, SE, SW are negative (closest blocker = highest set bit).
const fn dir_is_positive(dir_idx: usize) -> bool {
    matches!(dir_idx, E | N | NE | NW)
}

#[inline]
fn ray_attacks_one_dir(from_idx: usize, own: u64, dir_idx: usize) -> u64 {
    let ray = RAY_FROM[dir_idx][from_idx];
    let blockers = ray & own;
    if blockers == 0 {
        return ray;
    }
    let blocker_sq = if dir_is_positive(dir_idx) {
        blockers.trailing_zeros() as usize
    } else {
        63 - (blockers.leading_zeros() as usize)
    };
    let blocker_and_beyond = RAY_FROM[dir_idx][blocker_sq] | (1u64 << blocker_sq);
    ray & !blocker_and_beyond
}

pub(crate) fn rook_attacks(from_idx: u8, own: u64) -> u64 {
    let from = from_idx as usize;
    ray_attacks_one_dir(from, own, E)
        | ray_attacks_one_dir(from, own, N)
        | ray_attacks_one_dir(from, own, W)
        | ray_attacks_one_dir(from, own, S)
}

pub(crate) fn bishop_attacks(from_idx: u8, own: u64) -> u64 {
    let from = from_idx as usize;
    ray_attacks_one_dir(from, own, NE)
        | ray_attacks_one_dir(from, own, NW)
        | ray_attacks_one_dir(from, own, SE)
        | ray_attacks_one_dir(from, own, SW)
}

pub(crate) fn queen_attacks(from_idx: u8, own: u64) -> u64 {
    rook_attacks(from_idx, own) | bishop_attacks(from_idx, own)
}

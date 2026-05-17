//! Integration tests for mid-game sense token grants and revocations
//! (issue #87).

use rbc_rs::{Color, Error, Game, GameConfig, SenseAction, SenseShape, SenseToken, Square};

fn sq(file: u8, rank: u8) -> Square {
    Square::from_coords(file, rank).unwrap()
}

#[test]
fn default_game_has_one_token_per_side() {
    let game = Game::new(GameConfig::default());
    assert_eq!(game.sense_actions().len(), 64);
}

#[test]
fn grant_makes_extra_token_available_same_turn() {
    let mut game = Game::new(GameConfig::default());
    let _new_id = game.grant_sense_token(Color::White, SenseToken::new(SenseShape::point()));
    // 2 tokens × 64 centers = 128 actions.
    assert_eq!(game.sense_actions().len(), 128);
}

#[test]
fn grant_on_one_side_does_not_affect_other() {
    let mut game = Game::new(GameConfig::default());
    let _id = game.grant_sense_token(Color::Black, SenseToken::new(SenseShape::full_board()));
    // White's actions still 64.
    assert_eq!(game.sense_actions().len(), 64);

    game.apply_move(None).unwrap();
    assert_eq!(game.turn(), Some(Color::Black));
    assert_eq!(game.sense_actions().len(), 128);
}

#[test]
fn grant_after_using_default_token_reopens_sense() {
    let mut game = Game::new(GameConfig::default());
    let first: SenseAction = game.sense_actions().into_iter().next().unwrap();
    let _ = game.sense_with(first).unwrap();
    assert!(game.sense_actions().is_empty());

    let _id = game.grant_sense_token(Color::White, SenseToken::new(SenseShape::point()));
    assert_eq!(game.sense_actions().len(), 64);
}

#[test]
fn revoke_removes_unused_token_from_sense_actions() {
    let mut game = Game::new(GameConfig::default());
    let granted = game.grant_sense_token(Color::White, SenseToken::new(SenseShape::point()));
    assert_eq!(game.sense_actions().len(), 128);

    let removed = game.revoke_sense_token(Color::White, granted);
    assert!(removed);
    assert_eq!(game.sense_actions().len(), 64);
}

#[test]
fn sense_with_on_revoked_token_returns_invalid_sense() {
    let mut game = Game::new(GameConfig::default());
    let granted = game.grant_sense_token(Color::White, SenseToken::new(SenseShape::point()));
    let action = SenseAction {
        token: granted,
        center: sq(4, 4),
    };
    assert!(game.revoke_sense_token(Color::White, granted));
    assert_eq!(game.sense_with(action), Err(Error::InvalidSense));
}

#[test]
fn revoke_used_token_returns_true_but_makes_no_difference() {
    let mut game = Game::new(GameConfig::default());
    let granted = game.grant_sense_token(Color::White, SenseToken::new(SenseShape::point()));
    let action = SenseAction {
        token: granted,
        center: sq(4, 4),
    };
    let _ = game.sense_with(action).unwrap();
    assert!(game.revoke_sense_token(Color::White, granted));
    assert_eq!(game.sense_with(action), Err(Error::InvalidSense));
}

#[test]
fn revoke_returns_false_for_unknown_id() {
    let mut game = Game::new(GameConfig::default());
    // Grant on black side to obtain a SenseTokenId we know doesn't
    // exist on the white side (per-side IDs are independent — both
    // start at 0).
    let black_id = game.grant_sense_token(Color::Black, SenseToken::new(SenseShape::point()));
    let removed = game.revoke_sense_token(Color::White, black_id);
    // Maybe-or-maybe-not — depends on whether white happens to have
    // an ID that collides numerically. Both sides start at next_id=1
    // after their default-policy token; after the black grant the
    // black side has IDs 0 and 1; white only has 0. So black_id=1
    // does NOT match any white token.
    assert!(!removed);
}

#[test]
fn per_turn_state_replenishes_for_granted_tokens() {
    let mut game = Game::new(GameConfig::default());
    let granted = game.grant_sense_token(Color::White, SenseToken::new(SenseShape::point()));
    let action = SenseAction {
        token: granted,
        center: sq(4, 4),
    };
    let _ = game.sense_with(action).unwrap();

    // Use white's default token too so we don't get blocked, then
    // pass moves until back to white.
    let default_action: SenseAction = game.sense_actions().into_iter().next().unwrap();
    let _ = game.sense_with(default_action).unwrap();
    game.apply_move(None).unwrap();
    game.apply_move(None).unwrap();
    assert_eq!(game.turn(), Some(Color::White));

    // Granted token's per-turn flag is reset.
    let _ = game.sense_with(action).unwrap();
}

#[test]
fn ids_are_monotonic_across_grants_and_revokes() {
    let mut game = Game::new(GameConfig::default());
    let first = game.grant_sense_token(Color::White, SenseToken::new(SenseShape::point()));
    let _removed = game.revoke_sense_token(Color::White, first);
    let second = game.grant_sense_token(Color::White, SenseToken::new(SenseShape::point()));
    assert_ne!(first, second, "IDs must not be recycled after revoke");
}

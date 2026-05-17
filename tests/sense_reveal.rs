//! Integration tests for the per-token sense reveal mode
//! ([`SenseRevealMode`] — issue #84).

use rbc_rs::{
    Color, Game, GameConfig, SenseAction, SensePolicy, SenseRevealMode, SenseShape, SenseToken,
    Square,
};

fn sq(file: u8, rank: u8) -> Square {
    Square::from_coords(file, rank).unwrap()
}

fn sense_at(game: &mut Game, center: Square) -> Result<Option<rbc_rs::SenseResult>, rbc_rs::Error> {
    let action: SenseAction = game
        .sense_actions()
        .into_iter()
        .find(|a| a.center == center)
        .expect("center available");
    game.sense_with(action)
}

#[test]
fn default_token_is_immediate_reveal() {
    let token = SenseToken::new(SenseShape::point());
    assert_eq!(token.reveal_mode, SenseRevealMode::Immediate);
}

#[test]
fn immediate_sense_returns_result_directly() {
    let mut game = Game::new(GameConfig::default());
    let result = sense_at(&mut game, sq(4, 3))
        .expect("valid action")
        .expect("immediate token must return Some");
    assert_eq!(result.squares.len(), 9);
}

#[test]
fn deferred_sense_returns_none_until_revealed() {
    let mut config = GameConfig::default();
    config.white_sense_policy =
        SensePolicy::from_tokens(vec![
            SenseToken::new(SenseShape::window(1)).with_reveal_mode(SenseRevealMode::Deferred)
        ]);
    let mut game = Game::new(config);

    let sense_return = sense_at(&mut game, sq(4, 3)).expect("valid action");
    assert!(sense_return.is_none(), "deferred sense must hide result");

    let revealed = game.reveal_senses();
    assert_eq!(revealed.len(), 1, "exactly one deferred sense was buffered");
    assert_eq!(revealed[0].squares.len(), 9);
}

#[test]
fn reveal_senses_is_empty_when_no_deferred_buffered() {
    let mut game = Game::new(GameConfig::default());
    let revealed = game.reveal_senses();
    assert!(revealed.is_empty());

    // Immediate sense also doesn't buffer anything.
    let _ = sense_at(&mut game, sq(4, 3)).expect("valid").expect("Some");
    let revealed = game.reveal_senses();
    assert!(revealed.is_empty());
}

#[test]
fn reveal_senses_is_idempotent_after_drain() {
    let mut config = GameConfig::default();
    config.white_sense_policy =
        SensePolicy::from_tokens(vec![
            SenseToken::new(SenseShape::window(1)).with_reveal_mode(SenseRevealMode::Deferred)
        ]);
    let mut game = Game::new(config);
    let _ = sense_at(&mut game, sq(4, 3)).expect("valid");
    assert_eq!(game.reveal_senses().len(), 1);
    assert!(game.reveal_senses().is_empty());
}

#[test]
fn mixed_immediate_and_deferred_tokens_in_one_phase() {
    let mut config = GameConfig::default();
    config.white_sense_policy = SensePolicy::from_tokens(vec![
        SenseToken::new(SenseShape::window(1)), // immediate default
        SenseToken::new(SenseShape::point()).with_reveal_mode(SenseRevealMode::Deferred),
    ]);
    let mut game = Game::new(config);

    let actions = game.sense_actions();
    assert_eq!(actions.len(), 128);

    // Use whichever comes first; check both Some-then-None and
    // None-then-Some patterns are sensible.
    let first = actions[0];
    let second = *actions.iter().find(|a| a.token != first.token).unwrap();
    let first_ret = game.sense_with(first).expect("valid");
    let second_ret = game.sense_with(second).expect("valid");

    let returned = [first_ret.is_some(), second_ret.is_some()];
    // Exactly one Some (immediate), one None (deferred).
    assert_eq!(returned.iter().filter(|b| **b).count(), 1);

    let revealed = game.reveal_senses();
    assert_eq!(revealed.len(), 1);
}

#[test]
fn reveal_senses_propagates_revealed_into_history() {
    let mut config = GameConfig::default();
    config.white_sense_policy =
        SensePolicy::from_tokens(vec![
            SenseToken::new(SenseShape::window(1)).with_reveal_mode(SenseRevealMode::Deferred)
        ]);
    let mut game = Game::new(config);
    let _ = sense_at(&mut game, sq(4, 3)).expect("valid");
    let _ = game.reveal_senses();
    game.apply_move(None).expect("ok");

    let entry = game.history().first().expect("history entry");
    assert_eq!(entry.senses.len(), 1);
}

#[test]
fn apply_move_auto_reveals_forgotten_deferred_senses() {
    let mut config = GameConfig::default();
    config.white_sense_policy =
        SensePolicy::from_tokens(vec![
            SenseToken::new(SenseShape::window(1)).with_reveal_mode(SenseRevealMode::Deferred)
        ]);
    let mut game = Game::new(config);
    let ret = sense_at(&mut game, sq(4, 3)).expect("valid");
    assert!(ret.is_none());

    // Player forgets to call reveal_senses — apply_move proceeds
    // anyway and the deferred result still lands in history.
    game.apply_move(None).expect("ok");

    let entry = game.history().first().expect("history entry");
    assert_eq!(
        entry.senses.len(),
        1,
        "deferred sense must reach history even without explicit reveal"
    );
}

#[test]
fn deferred_grants_mid_game_buffer_correctly() {
    let mut game = Game::new(GameConfig::default());
    let granted = game.grant_sense_token(
        Color::White,
        SenseToken::new(SenseShape::point()).with_reveal_mode(SenseRevealMode::Deferred),
    );
    let action = SenseAction {
        token: granted,
        center: sq(0, 0),
    };
    let ret = game.sense_with(action).expect("valid");
    assert!(ret.is_none());
    let revealed = game.reveal_senses();
    assert_eq!(revealed.len(), 1);
    assert_eq!(revealed[0].action.token, granted);
}

#[test]
fn per_side_asymmetric_reveal_modes() {
    let mut config = GameConfig::default();
    config.white_sense_policy =
        SensePolicy::from_tokens(vec![
            SenseToken::new(SenseShape::window(1)).with_reveal_mode(SenseRevealMode::Immediate)
        ]);
    config.black_sense_policy =
        SensePolicy::from_tokens(vec![
            SenseToken::new(SenseShape::window(1)).with_reveal_mode(SenseRevealMode::Deferred)
        ]);
    let mut game = Game::new(config);

    let white_ret = sense_at(&mut game, sq(4, 3)).expect("valid");
    assert!(white_ret.is_some(), "white token is immediate");

    game.apply_move(None).expect("ok");
    assert_eq!(game.turn(), Some(Color::Black));

    let black_ret = sense_at(&mut game, sq(4, 4)).expect("valid");
    assert!(black_ret.is_none(), "black token is deferred");
    assert_eq!(game.reveal_senses().len(), 1);
}

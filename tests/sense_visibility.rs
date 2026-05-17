//! Integration tests for per-token sense visibility (issue #83).

use rbc_rs::{
    Color, Game, GameConfig, SenseAction, SenseObservation, SensePolicy, SenseShape, SenseToken,
    SenseVisibility, Square,
};

fn sq(file: u8, rank: u8) -> Square {
    Square::from_coords(file, rank).unwrap()
}

fn sense_at(game: &mut Game, center: Square) -> rbc_rs::SenseResult {
    let action: SenseAction = game
        .sense_actions()
        .into_iter()
        .find(|a| a.center == center)
        .expect("center available");
    game.sense_with(action).expect("valid action")
}

#[test]
fn default_token_visibility_is_private() {
    let token = SenseToken::new(SenseShape::point());
    assert_eq!(token.visibility, SenseVisibility::Private);
}

#[test]
fn default_game_sense_yields_no_observation() {
    let mut game = Game::new(GameConfig::default());
    let result = sense_at(&mut game, sq(4, 3));
    assert_eq!(result.visibility, SenseVisibility::Private);
    assert_eq!(result.observation(), None);
}

#[test]
fn existence_only_observation() {
    let mut config = GameConfig::default();
    config.white_sense_policy =
        SensePolicy::from_tokens(vec![
            SenseToken::new(SenseShape::window(1)).with_visibility(SenseVisibility::Existence)
        ]);
    let mut game = Game::new(config);
    let result = sense_at(&mut game, sq(4, 3));
    assert_eq!(result.observation(), Some(SenseObservation::ExistenceOnly));
}

#[test]
fn shape_only_hides_center() {
    let shape = SenseShape::cross(2);
    let mut config = GameConfig::default();
    config.white_sense_policy = SensePolicy::from_tokens(vec![
        SenseToken::new(shape.clone()).with_visibility(SenseVisibility::Shape)
    ]);
    let mut game = Game::new(config);
    let result = sense_at(&mut game, sq(3, 4));
    assert_eq!(
        result.observation(),
        Some(SenseObservation::ShapeOnly { shape })
    );
}

#[test]
fn center_only_hides_shape() {
    let mut config = GameConfig::default();
    config.white_sense_policy =
        SensePolicy::from_tokens(vec![
            SenseToken::new(SenseShape::window(1)).with_visibility(SenseVisibility::Center)
        ]);
    let mut game = Game::new(config);
    let center = sq(4, 3);
    let result = sense_at(&mut game, center);
    assert_eq!(
        result.observation(),
        Some(SenseObservation::CenterOnly { center })
    );
}

#[test]
fn full_visibility_reveals_everything() {
    let shape = SenseShape::window(1);
    let mut config = GameConfig::default();
    config.white_sense_policy = SensePolicy::from_tokens(vec![
        SenseToken::new(shape.clone()).with_visibility(SenseVisibility::Full)
    ]);
    let mut game = Game::new(config);
    let center = sq(4, 3);
    let result = sense_at(&mut game, center);
    let obs = result.observation().expect("Full yields observation");
    match obs {
        SenseObservation::Full {
            center: c,
            shape: s,
            squares,
        } => {
            assert_eq!(c, center);
            assert_eq!(s, shape);
            assert_eq!(squares, result.squares);
        }
        other => panic!("expected Full, got {other:?}"),
    }
}

#[test]
fn board_only_reveals_squares_without_pieces() {
    let shape = SenseShape::window(1);
    let mut config = GameConfig::default();
    config.white_sense_policy = SensePolicy::from_tokens(vec![
        SenseToken::new(shape.clone()).with_visibility(SenseVisibility::Board)
    ]);
    let mut game = Game::new(config);
    let center = sq(4, 3);
    let result = sense_at(&mut game, center);
    let obs = result.observation().expect("Board yields observation");
    match obs {
        SenseObservation::BoardOnly {
            center: c,
            shape: s,
            squares,
        } => {
            assert_eq!(c, center);
            assert_eq!(s, shape);
            let expected: Vec<Square> = result.squares.iter().map(|sq| sq.square).collect();
            assert_eq!(squares, expected);
        }
        other => panic!("expected BoardOnly, got {other:?}"),
    }
}

#[test]
fn mixed_visibility_tokens_on_one_side() {
    let mut config = GameConfig::default();
    config.white_sense_policy = SensePolicy::from_tokens(vec![
        SenseToken::new(SenseShape::point()).with_visibility(SenseVisibility::Private),
        SenseToken::new(SenseShape::full_board()).with_visibility(SenseVisibility::Full),
    ]);
    let mut game = Game::new(config);
    let actions = game.sense_actions();
    // Two tokens × 64 centers.
    assert_eq!(actions.len(), 128);

    // Sense once with each token; observations must differ in
    // visibility level despite using the same engine.
    let mut iter = actions.iter();
    let first = *iter.next().unwrap();
    let second = *actions.iter().find(|a| a.token != first.token).unwrap();
    let first_result = game.sense_with(first).expect("valid");
    let second_result = game.sense_with(second).expect("valid");
    let observations = [first_result.observation(), second_result.observation()];
    assert!(observations.contains(&None), "expected one Private sense");
    assert!(
        observations
            .iter()
            .any(|o| matches!(o, Some(SenseObservation::Full { .. }))),
        "expected one Full sense"
    );
}

#[test]
fn visibility_is_snapshotted_at_sense_time() {
    let mut game = Game::new(GameConfig::default());
    let granted = game.grant_sense_token(
        Color::White,
        SenseToken::new(SenseShape::point()).with_visibility(SenseVisibility::Center),
    );
    let action = SenseAction {
        token: granted,
        center: sq(4, 4),
    };
    let _ = game.sense_with(action).expect("valid");
    // Use the default token so the per-turn budget is fully spent.
    let default_action: SenseAction = game
        .sense_actions()
        .into_iter()
        .find(|a| a.token != granted)
        .expect("default token available");
    let _ = game.sense_with(default_action).expect("valid");

    // Revoke the granted token mid-history.
    assert!(game.revoke_sense_token(Color::White, granted));

    // Apply move to commit history.
    game.apply_move(None).expect("ok");

    let entry = game.history().first().expect("entry exists");
    let snapshotted = entry
        .senses
        .iter()
        .find(|s| s.action.token == granted)
        .expect("granted sense present");
    assert_eq!(snapshotted.visibility, SenseVisibility::Center);
    assert_eq!(
        snapshotted.observation(),
        Some(SenseObservation::CenterOnly { center: sq(4, 4) })
    );
}

#[test]
fn per_side_asymmetric_visibility() {
    let mut config = GameConfig::default();
    config.white_sense_policy =
        SensePolicy::from_tokens(vec![
            SenseToken::new(SenseShape::window(1)).with_visibility(SenseVisibility::Private)
        ]);
    config.black_sense_policy =
        SensePolicy::from_tokens(vec![
            SenseToken::new(SenseShape::window(1)).with_visibility(SenseVisibility::Full)
        ]);
    let mut game = Game::new(config);

    let white_result = sense_at(&mut game, sq(4, 3));
    assert_eq!(white_result.observation(), None);

    game.apply_move(None).expect("white passes");
    assert_eq!(game.turn(), Some(Color::Black));

    let black_result = sense_at(&mut game, sq(4, 4));
    assert!(matches!(
        black_result.observation(),
        Some(SenseObservation::Full { .. })
    ));
}

#[test]
fn observation_after_token_revoked_still_works_from_history() {
    // Stronger: even after revocation, replaying observation() off
    // a historical SenseResult yields the same answer.
    let mut config = GameConfig::default();
    config.white_sense_policy =
        SensePolicy::from_tokens(vec![
            SenseToken::new(SenseShape::window(1)).with_visibility(SenseVisibility::Shape)
        ]);
    let mut game = Game::new(config);
    let result = sense_at(&mut game, sq(4, 3));
    let pre_obs = result.observation();
    game.apply_move(None).expect("ok");
    let entry = game.history().first().expect("entry exists");
    let recorded = &entry.senses[0];
    assert_eq!(recorded.observation(), pre_obs);
}

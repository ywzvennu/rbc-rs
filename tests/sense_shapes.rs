//! Integration tests for per-side custom sense shapes
//! (issue #85; updated for the `sense_with` API).

use rbc_rs::{Color, Game, GameConfig, SenseAction, SensePolicy, SenseResult, SenseShape, Square};

fn sq(file: u8, rank: u8) -> Square {
    Square::from_coords(file, rank).unwrap()
}

fn config_with(white: SenseShape, black: SenseShape) -> GameConfig {
    let mut config = GameConfig::default();
    config.white_sense_policy = SensePolicy::single(white);
    config.black_sense_policy = SensePolicy::single(black);
    config
}

fn sense_at(game: &mut Game, center: Square) -> SenseResult {
    let action: SenseAction = game
        .sense_actions()
        .into_iter()
        .find(|a| a.center == center)
        .expect("center available among actions");
    game.sense_with(action).expect("valid sense action")
}

#[test]
fn default_sense_shape_is_3x3() {
    let mut game = Game::new(GameConfig::default());
    let result = sense_at(&mut game, sq(4, 4));
    assert_eq!(result.squares.len(), 9);
}

#[test]
fn custom_5x5_center_returns_25_squares() {
    let mut game = Game::new(config_with(SenseShape::window(2), SenseShape::default()));
    let result = sense_at(&mut game, sq(4, 4));
    assert_eq!(result.squares.len(), 25);
}

#[test]
fn custom_5x5_corner_is_clipped_to_3x3() {
    let mut game = Game::new(config_with(SenseShape::window(2), SenseShape::default()));
    let result = sense_at(&mut game, sq(0, 0));
    assert_eq!(result.squares.len(), 9);
}

#[test]
fn cross_shape_returns_plus_pattern() {
    let mut game = Game::new(config_with(SenseShape::cross(2), SenseShape::default()));
    let result = sense_at(&mut game, sq(4, 4));
    assert_eq!(result.squares.len(), 9);
    let squares: Vec<Square> = result.squares.iter().map(|s| s.square).collect();
    assert_eq!(
        squares,
        vec![
            sq(4, 6),
            sq(4, 5),
            sq(2, 4),
            sq(3, 4),
            sq(4, 4),
            sq(5, 4),
            sq(6, 4),
            sq(4, 3),
            sq(4, 2),
        ],
    );
}

#[test]
fn full_board_shape_returns_64_squares() {
    let mut game = Game::new(config_with(SenseShape::full_board(), SenseShape::default()));
    let result = sense_at(&mut game, sq(0, 0));
    assert_eq!(result.squares.len(), 64);
}

#[test]
fn empty_shape_returns_zero_squares_but_records_action() {
    let mut game = Game::new(config_with(SenseShape::empty(), SenseShape::default()));
    let result = sense_at(&mut game, sq(4, 4));
    assert!(result.squares.is_empty());
    assert_eq!(result.action.center, sq(4, 4));
}

#[test]
fn point_shape_returns_single_center_square() {
    let mut game = Game::new(config_with(SenseShape::point(), SenseShape::default()));
    let result = sense_at(&mut game, sq(3, 3));
    assert_eq!(result.squares.len(), 1);
    assert_eq!(result.squares[0].square, sq(3, 3));
}

#[test]
fn shape_is_per_side() {
    let mut game = Game::new(config_with(SenseShape::point(), SenseShape::window(2)));

    assert_eq!(game.turn(), Some(Color::White));
    let white_sense = sense_at(&mut game, sq(4, 4));
    assert_eq!(white_sense.squares.len(), 1);

    game.apply_move(None).unwrap();
    assert_eq!(game.turn(), Some(Color::Black));
    let black_sense = sense_at(&mut game, sq(4, 4));
    assert_eq!(black_sense.squares.len(), 25);
}

#[test]
fn custom_shape_with_arbitrary_offsets() {
    let shape = SenseShape::custom(vec![(0, 1), (0, -1)]);
    let mut game = Game::new(config_with(shape, SenseShape::default()));
    let result = sense_at(&mut game, sq(4, 4));
    let squares: Vec<Square> = result.squares.iter().map(|s| s.square).collect();
    assert_eq!(squares, vec![sq(4, 5), sq(4, 3)]);
}

#[test]
fn rectangle_3x1_horizontal() {
    let mut game = Game::new(config_with(
        SenseShape::rectangle(1, 0),
        SenseShape::default(),
    ));
    let result = sense_at(&mut game, sq(4, 4));
    let squares: Vec<Square> = result.squares.iter().map(|s| s.square).collect();
    assert_eq!(squares, vec![sq(3, 4), sq(4, 4), sq(5, 4)]);
}

#[test]
fn rectangle_1x3_vertical() {
    let mut game = Game::new(config_with(
        SenseShape::rectangle(0, 1),
        SenseShape::default(),
    ));
    let result = sense_at(&mut game, sq(4, 4));
    let squares: Vec<Square> = result.squares.iter().map(|s| s.square).collect();
    assert_eq!(squares, vec![sq(4, 5), sq(4, 4), sq(4, 3)]);
}

#[test]
fn sense_actions_includes_all_64_squares_for_default() {
    let game = Game::new(GameConfig::default());
    assert_eq!(game.sense_actions().len(), 64);
}

#[test]
fn sense_actions_empty_after_using_the_per_turn_token() {
    let mut game = Game::new(GameConfig::default());
    let _ = sense_at(&mut game, sq(4, 4));
    assert!(game.sense_actions().is_empty());
}

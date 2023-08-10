// Ajuna Node
// Copyright (C) 2022 BlogaTech AG

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.

// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use crate::dot4gravity::*;

const ALICE: u8 = 11;
const BOB: u8 = 22;
const CHARLIE: u8 = 33;

#[test]
fn should_create_a_new_board() {
	fn is_empty(board: &Board) -> bool {
		let mut empty = true;
		for row in board.cells {
			for cell in row {
				if cell != Cell::Empty {
					empty = false;
				}
			}
		}
		empty
	}

	let board = Board::new();
	assert_eq!(board.cells.len() as u8, BOARD_HEIGHT);
	assert_eq!(board.cells[0].len() as u8, BOARD_WIDTH);
	assert!(is_empty(&board))
}

#[test]
fn board_cell_can_be_changed() {
	let mut board = Board::new();
	let coords = Coordinates { row: 5, col: 5 };

	assert_eq!(board.get_cell(&coords), Cell::Empty, "Cell should be empty before change.");
	board.update_cell(&coords, Cell::Block);
	assert_eq!(board.get_cell(&coords), Cell::Block, "Cell should had changed.");
}

#[test]
fn should_create_new_game() {
	let game_state = Game::new_game(ALICE, BOB, Some(INITIAL_SEED));
	let computed_from_initial_seed = 46_384;
	assert_eq!(game_state.seed, computed_from_initial_seed);
	assert_eq!(game_state.winner, None, "No player should have won yet");
	assert_eq!(game_state.next_player, ALICE);
	assert_eq!(game_state.bomb_energy.len(), NUM_OF_PLAYERS);
	assert_eq!(game_state.get_bomb_energy_for(&ALICE), Some(BOMB_ENERGY_PER_PLAYER),);
	assert_eq!(game_state.get_bomb_energy_for(&BOB), Some(BOMB_ENERGY_PER_PLAYER),);
	assert!(game_state.is_player_in_game(&ALICE), "Player Alice should be in the game");
	assert!(game_state.is_player_in_game(&BOB), "Player Bob should be in the game");
	assert_eq!(game_state.last_move, None);
}

#[test]
fn should_create_new_game_with_random_blocks() {
	let blocks = |board: Board| -> u8 {
		let mut block_count = 0;
		board.cells.iter().for_each(|row| {
			row.iter().for_each(|cell| {
				if let Cell::Block = cell {
					block_count += 1;
				}
			})
		});
		block_count
	};

	let (mut seed_1, mut seed_2) = (123, 456);
	for _ in 0..20 {
		let game_1 = Game::new_game(ALICE, BOB, Some(seed_1));
		let game_2 = Game::new_game(ALICE, BOB, Some(seed_2));
		assert_ne!(game_1.board, game_2.board);
		assert_eq!(blocks(game_1.board), NUM_OF_BLOCKS);
		assert_eq!(blocks(game_2.board), NUM_OF_BLOCKS);
		assert_ne!(seed_1, game_1.seed, "seed 1 should be updated");
		assert_ne!(seed_2, game_2.seed, "seed 2 should be updated");
		seed_1 = game_1.seed;
		seed_2 = game_2.seed;
	}
}

#[test]
fn should_create_new_game_with_deterministic_blocks_with_fixed_seed() {
	let seed = 7357;
	for _ in 0..20 {
		let game_1 = Game::new_game(ALICE, BOB, Some(seed));
		let game_2 = Game::new_game(ALICE, BOB, Some(seed));
		assert_eq!(game_1.board, game_2.board);
	}
}

#[test]
fn a_player_cannot_drop_a_stone_out_of_turn() {
	let state = Game::new_game(ALICE, BOB, Some(INITIAL_SEED));
	let drop_stone_result = Game::drop_stone(state, BOB, Side::North, 0);
	assert_eq!(drop_stone_result, Err(GameError::NotPlayerTurn));
}

#[test]
fn a_player_cannot_drop_stone_if_game_already_finished() {
	let mut game_state = Game::new_game(ALICE, BOB, Some(INITIAL_SEED));
	game_state.winner = Some(BOB);
	assert_eq!(
		Game::drop_stone(game_state, ALICE, Side::East, 1),
		Err(GameError::GameAlreadyFinished),
	)
}

#[test]
fn a_player_cannot_place_more_bombs_than_the_limit() {
	let mut game_state = Game::new_game(ALICE, BOB, Some(INITIAL_SEED));

	for i in 0..BOMB_AMOUNT_PER_PLAYER {
		game_state = Game::place_bomb(
			game_state,
			ALICE,
			Coordinates { row: 0, col: i as u8 },
			HashSalt::from_low_u64_le(3453),
		)
		.expect("Bomb should be placed");

		game_state.next_player = ALICE;
	}

	assert_eq!(
		Game::place_bomb(
			game_state,
			ALICE,
			Coordinates { row: 0, col: 9 },
			HashSalt::from_low_u64_le(3453),
		),
		Err(GameError::NoMoreBombsAvailable),
	)
}

#[test]
fn a_player_cannot_place_a_bomb_in_the_same_coordinates() {
	let mut game_state = Game::new_game(ALICE, BOB, Some(INITIAL_SEED));

	game_state = Game::place_bomb(
		game_state,
		ALICE,
		Coordinates { row: 0, col: 0 },
		HashSalt::from_low_u64_le(3453),
	)
	.expect("Bomb should be placed");

	game_state.next_player = ALICE;

	assert_eq!(
		Game::place_bomb(
			game_state,
			ALICE,
			Coordinates { row: 0, col: 0 },
			HashSalt::from_low_u64_le(3453),
		),
		Err(GameError::InvalidBombCoordinates),
	)
}

#[test]
fn a_player_cannot_detonate_a_bomb_with_the_wrong_salt() {
	let mut game_state = Game::new_game(ALICE, BOB, Some(INITIAL_SEED));

	game_state = Game::place_bomb(
		game_state,
		ALICE,
		Coordinates { row: 0, col: 0 },
		HashSalt::from_low_u64_le(3453),
	)
	.expect("Bomb should be placed");

	game_state.next_player = ALICE;

	assert_eq!(
		Game::detonate_bomb(
			game_state,
			ALICE,
			Coordinates { row: 0, col: 0 },
			HashSalt::from_low_u64_le(1241),
			PowerLevel::One
		),
		Err(GameError::InvalidBombCoordinates),
	)
}

#[test]
fn a_player_cannot_detonate_a_bomb_with_the_wrong_coordinates() {
	let mut game_state = Game::new_game(ALICE, BOB, Some(INITIAL_SEED));

	game_state = Game::place_bomb(
		game_state,
		ALICE,
		Coordinates { row: 0, col: 0 },
		HashSalt::from_low_u64_le(3453),
	)
	.expect("Bomb should be placed");

	game_state.next_player = ALICE;

	assert_eq!(
		Game::detonate_bomb(
			game_state,
			ALICE,
			Coordinates { row: 0, col: 1 },
			HashSalt::from_low_u64_le(3453),
			PowerLevel::One
		),
		Err(GameError::InvalidBombCoordinates),
	)
}

#[test]
fn player_turn_changes_after_dropping_stone() {
	let mut state = Game::new_game(CHARLIE, BOB, Some(INITIAL_SEED));
	for i in 0..BOARD_WIDTH {
		state.board.update_cell(&Coordinates::new(i, 0), Cell::Empty);
	}

	let drop_stone_result = Game::drop_stone(state, CHARLIE, Side::North, 0);
	assert!(drop_stone_result.is_ok());
	let state = drop_stone_result.unwrap();

	let drop_stone_result = Game::drop_stone(state.clone(), CHARLIE, Side::North, 0);
	assert_eq!(drop_stone_result, Err(GameError::NotPlayerTurn));

	let drop_stone_result = Game::drop_stone(state, BOB, Side::North, 0);
	assert!(drop_stone_result.is_ok());
}

#[test]
fn last_move_changes_after_dropping_stone() {
	let mut state = Game::new_game(BOB, ALICE, Some(INITIAL_SEED));

	assert_eq!(state.last_move, None);

	for (player, side, position) in [
		(BOB, Side::West, 2),
		(ALICE, Side::East, 1),
		(BOB, Side::North, 6),
		(ALICE, Side::South, 8),
	] {
		state = Game::drop_stone(state, player, side, position).unwrap();
		assert_eq!(state.last_move, Some(LastMove::new(player, side, position)));
	}
}

#[test]
fn a_stone_dropped_on_a_stone() {
	let mut state = Game::new_game(ALICE, BOB, Some(INITIAL_SEED));
	let (alice_index, bob_index) = (state.player_index(&ALICE), state.player_index(&BOB));

	let o = Cell::Empty;
	let x = Cell::Stone(bob_index);
	let cells = [
		[o, x, o, o, o, o, o, o, o, o],
		[o, o, o, o, o, o, o, o, o, o],
		[o, o, o, o, o, o, o, o, o, o],
		[o, o, o, o, o, o, o, o, o, o],
		[o, o, o, o, o, o, o, o, o, o],
		[o, o, o, o, o, o, o, o, o, o],
		[o, o, o, o, o, o, o, o, o, o],
		[o, o, o, o, o, o, o, o, o, o],
		[o, o, o, o, o, o, o, o, o, o],
		[o, o, o, o, o, o, o, o, o, o],
	];

	state.board.cells = cells;

	let state = Game::drop_stone(state, ALICE, Side::West, 0).unwrap();
	assert_eq!(state.board.get_cell(&Coordinates { row: 0, col: 0 }), Cell::Stone(alice_index));
	assert_eq!(state.board.get_cell(&Coordinates { row: 0, col: 1 }), Cell::Stone(bob_index));
}

#[test]
fn a_stone_cannot_be_dropped_at_bounds() {
	let state = Game::new_game(ALICE, BOB, Some(INITIAL_SEED));

	let mut state_with_stones_at_bounds = state.clone();
	let o = Cell::Empty;
	let x = Cell::Stone(state_with_stones_at_bounds.player_index(&BOB));
	state_with_stones_at_bounds.board.cells = [
		[x, x, x, x, x, x, x, x, x, x],
		[x, o, o, o, o, o, o, o, o, x],
		[x, o, o, o, o, o, o, o, o, x],
		[x, o, o, o, o, o, o, o, o, x],
		[x, o, o, o, o, o, o, o, o, x],
		[x, o, o, o, o, o, o, o, o, x],
		[x, o, o, o, o, o, o, o, o, x],
		[x, o, o, o, o, o, o, o, o, x],
		[x, o, o, o, o, o, o, o, o, x],
		[x, x, x, x, x, x, x, x, x, x],
	];

	let mut state_with_blocks_at_bounds = state;
	let b = Cell::Block;
	state_with_blocks_at_bounds.board.cells = [
		[b, b, b, b, b, b, b, b, b, b],
		[b, o, o, o, o, o, o, o, o, b],
		[b, o, o, o, o, o, o, o, o, b],
		[b, o, o, o, o, o, o, o, o, b],
		[b, o, o, o, o, o, o, o, o, b],
		[b, o, o, o, o, o, o, o, o, b],
		[b, o, o, o, o, o, o, o, o, b],
		[b, o, o, o, o, o, o, o, o, b],
		[b, o, o, o, o, o, o, o, o, b],
		[b, b, b, b, b, b, b, b, b, x],
	];

	for state in [state_with_stones_at_bounds, state_with_blocks_at_bounds] {
		// left -> right check, dropping stones from top and bottom
		for position in 0..BOARD_WIDTH {
			assert_eq!(
				Game::drop_stone(state.clone(), ALICE, Side::North, position),
				Err(GameError::InvalidStonePosition)
			);
			assert_eq!(
				Game::drop_stone(state.clone(), ALICE, Side::South, position),
				Err(GameError::InvalidStonePosition)
			);
		}

		// top -> bottom check, dropping stones from left and right
		for position in 0..BOARD_HEIGHT {
			assert_eq!(
				Game::drop_stone(state.clone(), ALICE, Side::West, position),
				Err(GameError::InvalidStonePosition)
			);
			assert_eq!(
				Game::drop_stone(state.clone(), ALICE, Side::East, position),
				Err(GameError::InvalidStonePosition)
			);
		}
	}
}

#[test]
fn a_stone_dropped_from_north_side_should_move_until_it_reaches_an_obstacle() {
	let o = Cell::Empty;
	let b = Cell::Block;
	let cells = [
		[o, o, o, b, o, o, o, o, o, o],
		[o, o, b, o, o, o, o, o, o, o],
		[o, o, o, o, o, o, o, o, o, o],
		[o, o, o, o, o, o, o, o, o, o],
		[o, o, o, o, o, o, o, o, o, o],
		[o, o, o, o, o, o, o, o, o, o],
		[o, o, o, o, o, o, o, o, o, o],
		[o, o, o, o, o, o, o, o, o, o],
		[o, o, o, o, o, o, o, o, o, o],
		[o, b, o, o, o, o, o, o, o, o],
	];

	let mut state = Game::new_game(ALICE, BOB, Some(INITIAL_SEED));
	state.board.cells = cells;

	let state = Game::drop_stone(state, ALICE, Side::North, 0).unwrap();
	let (alice_index, bob_index) = (state.player_index(&ALICE), state.player_index(&BOB));
	assert_eq!(state.board.get_cell(&Coordinates { row: 9, col: 0 }), Cell::Stone(alice_index));
	let state = Game::drop_stone(state, BOB, Side::North, 1).unwrap();
	assert_eq!(state.board.get_cell(&Coordinates { row: 8, col: 1 }), Cell::Stone(bob_index));
	let state = Game::drop_stone(state, ALICE, Side::North, 2).unwrap();
	assert_eq!(state.board.get_cell(&Coordinates { row: 0, col: 2 }), Cell::Stone(alice_index));
	assert_eq!(
		Game::drop_stone(state, BOB, Side::North, 3).unwrap_err(),
		GameError::InvalidStonePosition
	);
}

#[test]
fn a_stone_dropped_from_south_side_should_move_until_it_reaches_an_obstacle() {
	let o = Cell::Empty;
	let b = Cell::Block;

	let cells = [
		[o, b, o, o, o, o, o, o, o, o],
		[o, o, o, o, o, o, o, o, o, o],
		[o, o, o, o, o, o, o, o, o, o],
		[o, o, o, o, o, o, o, o, o, o],
		[o, o, o, o, o, o, o, o, o, o],
		[o, o, o, o, o, o, o, o, o, o],
		[o, o, o, o, o, o, o, o, o, o],
		[o, o, o, o, o, o, o, o, o, o],
		[o, o, b, o, o, o, o, o, o, o],
		[o, o, o, b, o, o, o, o, o, o],
	];

	let mut state = Game::new_game(ALICE, BOB, Some(INITIAL_SEED));
	let (alice_index, bob_index) = (state.player_index(&ALICE), state.player_index(&BOB));
	state.board.cells = cells;

	let state = Game::drop_stone(state, ALICE, Side::South, 0).unwrap();
	assert_eq!(state.board.get_cell(&Coordinates { row: 0, col: 0 }), Cell::Stone(alice_index));
	let state = Game::drop_stone(state, BOB, Side::South, 1).unwrap();
	assert_eq!(state.board.get_cell(&Coordinates { row: 1, col: 1 }), Cell::Stone(bob_index));
	let state = Game::drop_stone(state, ALICE, Side::South, 2).unwrap();
	assert_eq!(state.board.get_cell(&Coordinates { row: 9, col: 2 }), Cell::Stone(alice_index));
	assert_eq!(
		Game::drop_stone(state, BOB, Side::South, 3).unwrap_err(),
		GameError::InvalidStonePosition
	);
}

#[test]
fn a_stone_dropped_from_east_side_should_move_until_it_reaches_an_obstacle() {
	let o = Cell::Empty;
	let b = Cell::Block;

	let cells = [
		[o, o, o, o, o, o, o, o, o, o],
		[b, o, o, o, o, o, o, o, o, o],
		[o, o, o, o, o, o, o, o, b, o],
		[o, o, o, o, o, o, o, o, o, b],
		[o, o, o, o, o, o, o, o, o, o],
		[o, o, o, o, o, o, o, o, o, o],
		[o, o, o, o, o, o, o, o, o, o],
		[o, o, o, o, o, o, o, o, o, o],
		[o, o, o, o, o, o, o, o, o, o],
		[o, o, o, o, o, o, o, o, o, o],
	];

	let mut state = Game::new_game(ALICE, BOB, Some(INITIAL_SEED));
	let (alice_index, bob_index) = (state.player_index(&ALICE), state.player_index(&BOB));
	state.board.cells = cells;

	let state = Game::drop_stone(state, ALICE, Side::East, 0).unwrap();
	assert_eq!(state.board.get_cell(&Coordinates { row: 0, col: 0 }), Cell::Stone(alice_index));
	let state = Game::drop_stone(state, BOB, Side::East, 1).unwrap();
	assert_eq!(state.board.get_cell(&Coordinates { row: 1, col: 1 }), Cell::Stone(bob_index));
	let state = Game::drop_stone(state, ALICE, Side::East, 2).unwrap();
	assert_eq!(state.board.get_cell(&Coordinates { row: 2, col: 9 }), Cell::Stone(alice_index));
	assert_eq!(
		Game::drop_stone(state, BOB, Side::East, 3).unwrap_err(),
		GameError::InvalidStonePosition
	);
}

#[test]
fn a_stone_dropped_from_west_side_should_move_until_it_reaches_an_obstacle() {
	let o = Cell::Empty;
	let b = Cell::Block;

	let cells = [
		[o, o, o, o, o, o, o, o, o, o],
		[o, o, o, o, o, o, o, o, o, b],
		[o, b, o, o, o, o, o, o, o, o],
		[b, o, o, o, o, o, o, o, o, o],
		[o, o, o, o, o, o, o, o, o, o],
		[o, o, o, o, o, o, o, o, o, o],
		[o, o, o, o, o, o, o, o, o, o],
		[o, o, o, o, o, o, o, o, o, o],
		[o, o, o, o, o, o, o, o, o, o],
		[o, o, o, o, o, o, o, o, o, o],
	];

	let mut state = Game::new_game(ALICE, BOB, Some(INITIAL_SEED));
	state.board.cells = cells;

	let state = Game::drop_stone(state, ALICE, Side::West, 0).unwrap();
	let (alice_index, bob_index) = (state.player_index(&ALICE), state.player_index(&BOB));
	assert_eq!(state.board.get_cell(&Coordinates { row: 0, col: 9 }), Cell::Stone(alice_index));
	let state = Game::drop_stone(state, BOB, Side::West, 1).unwrap();
	assert_eq!(state.board.get_cell(&Coordinates { row: 1, col: 8 }), Cell::Stone(bob_index));
	let state = Game::drop_stone(state, ALICE, Side::West, 2).unwrap();
	assert_eq!(state.board.get_cell(&Coordinates { row: 2, col: 0 }), Cell::Stone(alice_index));
	assert_eq!(
		Game::drop_stone(state, BOB, Side::West, 3).unwrap_err(),
		GameError::InvalidStonePosition
	);
}

#[test]
fn a_player_wins_when_has_stones_in_three_squares() {
	let mut state = Game::new_game(ALICE, BOB, Some(INITIAL_SEED));
	let alice_index = state.player_index(&ALICE);
	let o = Cell::Empty;
	let s = Cell::Stone(alice_index);
	state.board.cells = [
		[o, o, o, o, o, o, o, o, o, o],
		[o, o, s, s, o, o, o, o, o, o],
		[o, o, s, s, o, o, o, o, o, o],
		[o, o, o, o, o, o, o, o, o, o],
		[o, o, o, o, o, s, s, o, o, o],
		[o, o, o, o, o, s, s, o, o, o],
		[o, o, o, s, s, o, o, o, o, o],
		[o, o, o, s, s, o, o, o, o, o],
		[o, o, o, o, o, o, o, o, o, o],
		[o, o, o, o, o, o, o, o, o, o],
	];

	state = Game::check_winner_player(state);
	assert_eq!(state.winner, Some(ALICE));
}

#[test]
fn a_player_wins_when_has_stones_in_three_squares_with_overlap() {
	let mut state = Game::new_game(CHARLIE, BOB, Some(INITIAL_SEED));
	let winner_index = state.player_index(&BOB);
	let o = Cell::Empty;
	let w = Cell::Stone(winner_index);
	state.board.cells = [
		[o, o, o, o, o, o, o, o, o, o],
		[o, o, o, o, o, o, o, o, o, o],
		[o, o, o, o, o, o, o, o, o, o],
		[o, o, o, o, w, w, o, o, o, o],
		[o, o, o, w, w, w, o, o, o, o],
		[o, o, o, w, w, w, o, o, o, o],
		[o, o, o, o, o, o, o, o, o, o],
		[o, o, o, o, o, o, o, o, o, o],
		[o, o, o, o, o, o, o, o, o, o],
		[o, o, o, o, o, o, o, o, o, o],
	];

	state = Game::check_winner_player(state);
	assert_eq!(state.winner, Some(BOB));
}

#[test]
fn no_player_wins_if_stones_are_not_in_four_squares() {
	let mut state = Game::new_game(ALICE, BOB, Some(INITIAL_SEED));
	let o = Cell::Empty;
	let b = Cell::Block;
	let r = Cell::Stone(state.player_index(&ALICE));
	let m = Cell::Stone(state.player_index(&BOB));
	state.board.cells = [
		[o, r, o, o, o, o, o, o, m, o],
		[m, o, o, o, o, m, o, o, o, o],
		[m, o, r, r, m, m, m, o, o, o],
		[b, o, o, o, o, m, m, o, o, o],
		[m, m, o, o, r, o, o, o, o, o],
		[m, m, b, m, o, r, o, o, o, o],
		[o, o, o, o, b, o, m, o, o, o],
		[o, o, r, o, o, o, o, r, o, o],
		[r, r, r, o, o, o, o, o, o, o],
		[r, r, r, o, o, o, o, o, o, o],
	];

	state = Game::check_winner_player(state);
	assert!(state.winner.is_none(), "No player should have won");
}

#[test]
fn should_play_a_game() {
	let o = Cell::Empty;
	let b = Cell::Block;

	let salt = HashSalt::from_low_u64_be(17);

	let bob_bomb_coordinates = Coordinates { row: 0, col: 0 };
	let alice_bomb_coordinates = Coordinates { row: 8, col: 2 };

	let mut state = Game::new_game(ALICE, BOB, Some(INITIAL_SEED));
	state.board.cells = [
		[o, o, o, o, o, o, o, o, b, o],
		[b, o, o, o, o, o, o, o, o, o],
		[b, o, o, o, b, b, b, o, o, o],
		[b, o, o, o, o, o, o, o, o, o],
		[b, o, o, o, o, o, o, o, o, o],
		[b, o, o, o, o, o, o, o, o, o],
		[o, o, o, o, o, o, b, o, o, o],
		[o, o, o, o, o, o, o, o, o, o],
		[o, o, o, o, o, o, o, o, o, o],
		[o, o, o, o, o, o, o, o, o, o],
	];

	let drop_stone_result = Game::drop_stone(state.clone(), BOB, Side::North, 0);
	assert!(drop_stone_result.is_err());
	assert_eq!(drop_stone_result.unwrap_err(), GameError::NotPlayerTurn);

	let drop_stone_result = Game::drop_stone(state, ALICE, Side::North, 0);
	assert!(drop_stone_result.is_ok());
	state = drop_stone_result.unwrap();

	let place_bomb_result = Game::place_bomb(state, BOB, bob_bomb_coordinates, salt);
	assert!(place_bomb_result.is_ok());
	state = place_bomb_result.unwrap();

	let place_bomb_result = Game::place_bomb(state, ALICE, alice_bomb_coordinates, salt);
	assert!(place_bomb_result.is_ok());
	state = place_bomb_result.unwrap();

	let detonate_result =
		Game::detonate_bomb(state, BOB, bob_bomb_coordinates, salt, PowerLevel::One);
	assert!(detonate_result.is_ok());
	state = detonate_result.unwrap();

	assert_eq!(state.board.get_cell(&Coordinates { row: 0, col: 0 }), Cell::Empty);

	state = Game::drop_stone(state, ALICE, Side::South, 8).unwrap();
	state = Game::drop_stone(state, BOB, Side::North, 2).unwrap();
	state = Game::drop_stone(state, ALICE, Side::South, 8).unwrap();
	state = Game::drop_stone(state, BOB, Side::North, 2).unwrap();

	// alice plays first square of stones
	state = Game::drop_stone(state, ALICE, Side::South, 8).unwrap();
	state = Game::drop_stone(state, BOB, Side::North, 2).unwrap();
	state = Game::drop_stone(state, ALICE, Side::South, 8).unwrap();
	state = Game::drop_stone(state, BOB, Side::North, 2).unwrap();
	state = Game::drop_stone(state, ALICE, Side::East, 1).unwrap();
	state = Game::drop_stone(state, BOB, Side::North, 2).unwrap();

	// alice plays second square of stones
	state = Game::drop_stone(state, ALICE, Side::South, 9).unwrap();
	state = Game::drop_stone(state, BOB, Side::East, 8).unwrap();
	state = Game::drop_stone(state, ALICE, Side::South, 9).unwrap();
	state = Game::drop_stone(state, BOB, Side::East, 8).unwrap();

	// alice plays third square of stones
	state = Game::drop_stone(state, ALICE, Side::North, 5).unwrap();
	state = Game::drop_stone(state, BOB, Side::East, 8).unwrap();
	state = Game::drop_stone(state, ALICE, Side::North, 5).unwrap();
	state = Game::drop_stone(state, BOB, Side::East, 8).unwrap();
	state = Game::drop_stone(state, ALICE, Side::North, 6).unwrap();
	state = Game::drop_stone(state, BOB, Side::East, 8).unwrap();

	// alice detonates bomb in the middle of bob's intersection of stones
	let detonate_result =
		Game::detonate_bomb(state, ALICE, alice_bomb_coordinates, salt, PowerLevel::Two);
	assert!(detonate_result.is_ok());
	state = detonate_result.unwrap();

	// bob drops a stone back
	state = Game::drop_stone(state, BOB, Side::South, 2).unwrap();

	assert!(state.winner.is_none());
	let x = Cell::Stone(state.player_index(&ALICE));
	let y = Cell::Stone(state.player_index(&BOB));
	assert_eq!(
		state.board.cells,
		[
			[o, o, o, o, o, x, o, o, b, o],
			[b, o, o, o, o, x, x, o, x, x],
			[b, o, o, o, b, b, b, o, x, x],
			[b, o, o, o, o, o, o, o, x, x],
			[b, o, o, o, o, o, o, o, x, o],
			[b, o, y, o, o, o, o, o, o, o],
			[o, o, y, o, o, o, b, o, o, o],
			[o, o, y, o, o, o, o, o, o, o],
			[o, o, o, o, y, y, y, y, o, o],
			[o, o, o, o, o, o, o, o, o, o],
		]
	);

	// trigger winning condition and check winner
	state = Game::drop_stone(state, ALICE, Side::North, 6).unwrap();
	assert!(state.winner.is_some());
	assert_eq!(state.winner.unwrap(), ALICE);
}

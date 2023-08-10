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

use codec::{Decode, Encode, MaxEncodedLen};
use core::marker::PhantomData;
use frame_support::{pallet_prelude::ConstU32, BoundedVec};
use scale_info::{prelude::vec::Vec, TypeInfo};
use sp_core::H256;
use sp_io::hashing::blake2_256;

use sp_runtime::traits::{BlakeTwo256, Hash};

#[cfg(test)]
mod tests;
mod traits;

pub(crate) use traits::Bound;

const INITIAL_SEED: Seed = 123_456;
const INCREMENT: Seed = 74;
const MULTIPLIER: Seed = 75;
const MODULUS: Seed = Seed::pow(2, 16);

const BOARD_WIDTH: u8 = 10;
const BOARD_HEIGHT: u8 = 10;
const NUM_OF_PLAYERS: usize = 2;
const BOMB_AMOUNT_PER_PLAYER: usize = 3;
const BOMB_ENERGY_PER_PLAYER: u8 = 5;
const NUM_OF_BLOCKS: u8 = 10;

pub type PlayerIndex = u8;
pub type Position = u8;
pub type Seed = u32;
pub type HashSalt = H256;
pub type HashedCoordinates = H256;

/// Represents a cell of the board.
#[derive(Encode, Decode, TypeInfo, MaxEncodedLen, Clone, Copy, Debug, Eq, PartialEq)]
pub enum Cell {
	Empty,
	Block,
	Stone(PlayerIndex),
}

impl Default for Cell {
	fn default() -> Self {
		Self::Empty
	}
}

impl Cell {
	/// Tells if a cell is suitable for dropping a stone.
	fn is_stone_droppable(&self) -> bool {
		!matches!(self, Cell::Block | Cell::Stone(_))
	}
}

/// Coordinates for a cell in the board.
#[derive(Encode, Decode, TypeInfo, MaxEncodedLen, Clone, Copy, Debug, Eq, PartialEq)]
pub struct Coordinates {
	pub row: u8,
	pub col: u8,
}

impl Coordinates {
	pub const fn new(row: u8, col: u8) -> Self {
		Self { row, col }
	}

	fn random(seed: Seed) -> (Self, Seed) {
		let linear_congruential_generator = |seed: Seed| -> Seed {
			MULTIPLIER.saturating_mul(seed).saturating_add(INCREMENT) % MODULUS
		};

		let random_seed_1 = linear_congruential_generator(seed);
		let random_seed_2 = linear_congruential_generator(random_seed_1);

		(
			Coordinates::new(
				(random_seed_1 % (BOARD_WIDTH as Seed - 1)) as u8,
				(random_seed_2 % (BOARD_HEIGHT as Seed - 1)) as u8,
			),
			random_seed_2,
		)
	}

	/// Tells if a cell is in the opposite of a side.
	fn is_opposite_cell(&self, side: Side) -> bool {
		match side {
			Side::North => self.row == BOARD_HEIGHT - 1,
			Side::East => self.col == 0,
			Side::South => self.row == 0,
			Side::West => self.col == BOARD_WIDTH - 1,
		}
	}
}

/// Sides of the board from which a player can drop a stone.
#[derive(Encode, Decode, TypeInfo, MaxEncodedLen, Copy, Clone, Debug, Eq, PartialEq)]
pub enum Side {
	North,
	East,
	South,
	West,
}

impl Side {
	fn bound_coordinates(&self, position: Position) -> Coordinates {
		match self {
			Side::North => Coordinates::new(0, position),
			Side::South => Coordinates::new(BOARD_HEIGHT - 1, position),
			Side::West => Coordinates::new(position, 0),
			Side::East => Coordinates::new(position, BOARD_WIDTH - 1),
		}
	}
}

/// Bomb power radius levels.
#[derive(Encode, Decode, TypeInfo, MaxEncodedLen, Copy, Clone, Debug, Eq, PartialEq)]
pub enum PowerLevel {
	One,
	Two,
	Three,
}

impl PowerLevel {
	pub fn can_use_level(&self, energy: BombEnergy) -> bool {
		match self {
			PowerLevel::One => energy >= 1,
			PowerLevel::Two => energy >= 2,
			PowerLevel::Three => energy >= 3,
		}
	}

	pub fn explode<Player>(&self, game_state: &mut GameState<Player>, epicenter: &Coordinates) {
		// Level 1 explosion always triggers
		Self::detonate_cell(game_state, epicenter);

		if *self == PowerLevel::Two || *self == PowerLevel::Three {
			// Level 2 explosion
			let center_row = epicenter.row;
			let center_col = epicenter.col;

			Self::detonate_cell(game_state, &Coordinates::new(center_row + 1, center_col));
			Self::detonate_cell(game_state, &Coordinates::new(center_row, center_col + 1));
			Self::detonate_cell(game_state, &Coordinates::new(center_row - 1, center_col));
			Self::detonate_cell(game_state, &Coordinates::new(center_row, center_col - 1));

			if *self == PowerLevel::Three {
				// Level 3 explosion
				Self::detonate_cell(game_state, &Coordinates::new(center_row + 1, center_col + 1));
				Self::detonate_cell(game_state, &Coordinates::new(center_row + 1, center_col - 1));
				Self::detonate_cell(game_state, &Coordinates::new(center_row - 1, center_col + 1));
				Self::detonate_cell(game_state, &Coordinates::new(center_row - 1, center_col - 1));
			}
		}
	}

	pub fn decrease_bomb_energy<Player>(&self, game_state: &mut GameState<Player>, player: &Player)
	where
		Player: PartialEq + Clone,
	{
		let energy_decrease = match self {
			PowerLevel::One => 1,
			PowerLevel::Two => 2,
			PowerLevel::Three => 3,
		};

		game_state.decrease_bomb_energy_for(player, energy_decrease);
	}

	fn detonate_cell<Player>(game_state: &mut GameState<Player>, position: &Coordinates) {
		if matches!(game_state.board.get_cell(position), Cell::Stone(_)) {
			game_state.board.update_cell(position, Cell::Empty);
		}
	}
}

pub type BombEnergy = u8;

#[derive(Encode, Decode, TypeInfo, MaxEncodedLen, Copy, Clone, Eq, Debug, Default, PartialEq)]
pub struct Board {
	cells: [[Cell; BOARD_WIDTH as usize]; BOARD_HEIGHT as usize],
}

impl Board {
	pub fn new() -> Board {
		Board::default()
	}

	fn is_stone_droppable(&self, position: &Coordinates) -> bool {
		position.is_inside_board() && self.get_cell(position).is_stone_droppable()
	}

	fn get_cell(&self, position: &Coordinates) -> Cell {
		let cell = &self.cells[position.row as usize][position.col as usize];
		*cell
	}

	fn update_cell(&mut self, position: &Coordinates, cell: Cell) {
		self.cells[position.row as usize][position.col as usize] = cell;
		assert_eq!(self.cells[position.row as usize][position.col as usize], cell);
	}
}

#[derive(Encode, Decode, TypeInfo, Debug, Eq, PartialEq)]
pub enum GameError {
	/// The player has no more bombs to drop.
	NoMoreBombsAvailable,
	/// The player has not enough bomb energy for the requested power level.
	InsufficientBombEnergy,
	/// Tried to target a blocked cell or an owned stone with a bomb detonation.
	InvalidBombCoordinates,
	/// Tried to drop a stone in an invalid cell. The cell is already taken.
	InvalidStonePosition,
	/// Tried to drop a stone during other player's turn
	NotPlayerTurn,
	/// The cell has no previous position. It is an edge cell.
	NoPreviousPosition,
	/// Tried playing when game has finished.
	GameAlreadyFinished,
}

#[derive(Encode, Decode, TypeInfo, MaxEncodedLen, Copy, Clone, Debug, Eq, PartialEq)]
pub struct LastMove<Player> {
	pub player: Player,
	pub side: Side,
	pub position: Position,
}

impl<Player> LastMove<Player> {
	fn new(player: Player, side: Side, position: Position) -> Self {
		Self { player, side, position }
	}
}

#[derive(Encode, Decode, TypeInfo, MaxEncodedLen, Clone, Debug, Eq, PartialEq)]
pub struct GameState<Player> {
	/// Represents random seed.
	pub seed: Seed,
	/// Represents the game board.
	pub board: Board,
	/// When present,it contains the player that won.
	pub winner: Option<Player>,
	/// Next player turn.
	pub next_player: Player,
	/// Players:
	pub players: [Player; NUM_OF_PLAYERS],
	/// Amount of bomb energy available per player.
	pub bomb_energy: [(Player, BombEnergy); NUM_OF_PLAYERS],
	/// Amount of bomb energy available per player.
	pub bombs_placed: [BoundedVec<HashedCoordinates, ConstU32<{ BOMB_AMOUNT_PER_PLAYER as u32 }>>;
		NUM_OF_PLAYERS],
	/// Represents the last move.
	pub last_move: Option<LastMove<Player>>,
}

impl<Player: PartialEq + Clone> GameState<Player> {
	pub fn is_player_in_game(&self, player: &Player) -> bool {
		self.bomb_energy.iter().any(|(p, _)| *p == *player)
	}

	pub fn get_bomb_energy_for(&self, player: &Player) -> Option<u8> {
		self.bomb_energy
			.iter()
			.find(|(p, _)| *p == *player)
			.map(|(_, available_energy)| *available_energy)
	}

	pub fn decrease_bomb_energy_for(&mut self, player: &Player, amount: BombEnergy) {
		for (p, energy) in self.bomb_energy.iter_mut() {
			if *p == *player {
				*energy -= amount;
			}
		}
	}

	pub fn is_player_turn(&self, player: &Player) -> bool {
		self.next_player == *player
	}

	fn player_index(&self, player: &Player) -> PlayerIndex {
		let player_index = self
			.players
			.iter()
			.position(|this_player| this_player == player)
			.expect("game to always start with 2 players") as u8;
		player_index
	}

	fn next_player(&self) -> &Player {
		let current_player_index = self
			.players
			.iter()
			.position(|player| *player == self.next_player)
			.expect("next player to be a subset of players");
		&self.players[(current_player_index + 1) % NUM_OF_PLAYERS]
	}
}

#[derive(Encode, Decode, TypeInfo)]
pub struct Game<Player>(PhantomData<Player>);

impl<Player: PartialEq + Clone> Game<Player> {
	fn can_place_bomb(game_state: &GameState<Player>, player: &Player) -> Result<(), GameError> {
		if game_state.winner.is_some() {
			return Err(GameError::GameAlreadyFinished)
		}
		if !game_state.is_player_turn(player) {
			return Err(GameError::NotPlayerTurn)
		}
		let player_index = game_state.player_index(player);
		if game_state.bombs_placed[player_index as usize].len() >= BOMB_AMOUNT_PER_PLAYER {
			return Err(GameError::NoMoreBombsAvailable)
		}

		Ok(())
	}
	fn can_detonate_bomb(
		game_state: &GameState<Player>,
		player: &Player,
		power_level: &PowerLevel,
	) -> Result<(), GameError> {
		if game_state.winner.is_some() {
			return Err(GameError::GameAlreadyFinished)
		}
		if !game_state.is_player_turn(player) {
			return Err(GameError::NotPlayerTurn)
		}
		if !power_level.can_use_level(game_state.get_bomb_energy_for(player).unwrap_or_default()) {
			return Err(GameError::InsufficientBombEnergy)
		}

		Ok(())
	}

	fn can_drop_stone(
		game_state: &GameState<Player>,
		side: &Side,
		position: Position,
		player: &Player,
	) -> Result<(), GameError> {
		if game_state.winner.is_some() {
			return Err(GameError::GameAlreadyFinished)
		}
		if !game_state.is_player_turn(player) {
			return Err(GameError::NotPlayerTurn)
		}
		if !game_state.board.is_stone_droppable(&side.bound_coordinates(position)) {
			return Err(GameError::InvalidStonePosition)
		}
		Ok(())
	}
}

impl<Player: PartialEq + Clone> Game<Player> {
	/// Create a new game.
	pub fn new_game(player1: Player, player2: Player, seed: Option<Seed>) -> GameState<Player> {
		let mut board = Board::new();
		let mut blocks = Vec::new();
		let mut remaining_blocks = NUM_OF_BLOCKS;

		let mut seed = seed.unwrap_or(INITIAL_SEED);

		while remaining_blocks > 0 {
			let (block_coordinates, new_seed) = Coordinates::random(seed);
			seed = new_seed;
			if !blocks.contains(&block_coordinates) {
				blocks.push(block_coordinates);
				board.update_cell(&block_coordinates, Cell::Block);
				remaining_blocks -= 1;
			}
		}

		GameState {
			seed,
			board,
			winner: Default::default(),
			next_player: player1.clone(),
			players: [player1.clone(), player2.clone()],
			bomb_energy: [(player1, BOMB_ENERGY_PER_PLAYER), (player2, BOMB_ENERGY_PER_PLAYER)],
			bombs_placed: [BoundedVec::default(), BoundedVec::default()],
			last_move: Default::default(),
		}
	}

	pub fn place_bomb(
		mut game_state: GameState<Player>,
		player: Player,
		coordinates: Coordinates,
		salt: HashSalt,
	) -> Result<GameState<Player>, GameError> {
		Self::can_place_bomb(&game_state, &player)?;

		let player_index = game_state.player_index(&player);
		let coordinate_hash = Self::hash_coordinates(coordinates, salt);

		if game_state.bombs_placed[player_index as usize].contains(&coordinate_hash) {
			return Err(GameError::InvalidBombCoordinates)
		}

		game_state.bombs_placed[player_index as usize]
			.try_push(coordinate_hash)
			.map_err(|_| GameError::NoMoreBombsAvailable)?;

		game_state.next_player = game_state.next_player().clone();

		Ok(game_state)
	}

	pub fn detonate_bomb(
		mut game_state: GameState<Player>,
		player: Player,
		coordinates: Coordinates,
		salt: HashSalt,
		power_level: PowerLevel,
	) -> Result<GameState<Player>, GameError> {
		let player_index = game_state.player_index(&player);
		let coordinate_hash = Self::hash_coordinates(coordinates, salt);

		if !game_state.bombs_placed[player_index as usize].contains(&coordinate_hash) {
			return Err(GameError::InvalidBombCoordinates)
		}

		Self::can_detonate_bomb(&game_state, &player, &power_level)?;

		power_level.explode(&mut game_state, &coordinates);
		power_level.decrease_bomb_energy(&mut game_state, &player);

		game_state.bombs_placed[player_index as usize].retain(|hash| hash != &coordinate_hash);
		game_state.next_player = game_state.next_player().clone();

		Ok(game_state)
	}

	/// Drop stone. Called during play phase.
	pub fn drop_stone(
		mut game_state: GameState<Player>,
		player: Player,
		side: Side,
		position: Position,
	) -> Result<GameState<Player>, GameError> {
		Self::can_drop_stone(&game_state, &side, position, &player)?;
		let player_index = game_state.player_index(&player);
		match side {
			Side::North => {
				let mut row = 0;
				let mut stop = false;
				while row < BOARD_HEIGHT && !stop {
					let position = Coordinates::new(row, position);
					match game_state.board.get_cell(&position) {
						// The stone is placed at the end if it's empty.
						Cell::Empty =>
							if position.is_opposite_cell(side) {
								game_state.board.update_cell(&position, Cell::Stone(player_index));
								stop = true;
							},
						// The stone is placed in the position previous to a block.
						Cell::Block => {
							if row > 0 {
								game_state.board.update_cell(
									&Coordinates::new(position.row.saturating_sub(1), position.col),
									Cell::Stone(player_index),
								);
							} else {
								return Err(GameError::InvalidStonePosition)
							}
							stop = true;
						},
						// The stone is placed in the previous position of a stone.
						Cell::Stone(_) => {
							if row > 0 {
								game_state.board.update_cell(
									&Coordinates::new(position.row.saturating_sub(1), position.col),
									Cell::Stone(player_index),
								);
							} else {
								return Err(GameError::InvalidStonePosition)
							}
							stop = true;
						},
					}
					row += 1;
				}
			},
			Side::East => {
				let mut col = BOARD_WIDTH - 1;

				loop {
					let position = Coordinates::new(position, col);
					match game_state.board.get_cell(&position) {
						// The stone is placed at the end if it's empty.
						Cell::Empty =>
							if position.is_opposite_cell(side) {
								game_state.board.update_cell(&position, Cell::Stone(player_index));
								break
							},
						// The stone is placed in the position previous to a block.
						Cell::Block => {
							if col < BOARD_WIDTH - 1 {
								game_state.board.update_cell(
									&Coordinates::new(position.row, position.col + 1),
									Cell::Stone(player_index),
								);
							} else {
								return Err(GameError::InvalidStonePosition)
							}
							break
						},
						// The stone is placed in the previous position of a stone.
						Cell::Stone(_) => {
							if col < BOARD_WIDTH - 1 {
								game_state.board.update_cell(
									&Coordinates::new(position.row, position.col + 1),
									Cell::Stone(player_index),
								);
							} else {
								return Err(GameError::InvalidStonePosition)
							}
							break
						},
					}
					if col == 0 {
						break
					};
					col -= 1;
				}
			},
			Side::South => {
				let mut row = BOARD_HEIGHT - 1;

				loop {
					let position = Coordinates::new(row, position);
					match game_state.board.get_cell(&position) {
						// The stone is placed at the end if it's empty.
						Cell::Empty =>
							if position.is_opposite_cell(side) {
								game_state.board.update_cell(&position, Cell::Stone(player_index));
								break
							},
						// The stone is placed in the position previous to a block.
						Cell::Block => {
							if row < BOARD_HEIGHT - 1 {
								game_state.board.update_cell(
									&Coordinates::new(position.row + 1, position.col),
									Cell::Stone(player_index),
								);
							} else {
								return Err(GameError::InvalidStonePosition)
							}
							break
						},
						// The stone is placed in the previous position of a stone.
						Cell::Stone(_) => {
							if row < BOARD_HEIGHT - 1 {
								game_state.board.update_cell(
									&Coordinates::new(position.row + 1, position.col),
									Cell::Stone(player_index),
								);
							} else {
								return Err(GameError::InvalidStonePosition)
							}
							break
						},
					}

					if row == 0 {
						break
					}
					row -= 1;
				}
			},
			Side::West => {
				let mut col = 0;
				let mut stop = false;
				while col < BOARD_WIDTH && !stop {
					let position = Coordinates::new(position, col);
					match game_state.board.get_cell(&position) {
						// The stone is placed at the end if it's empty.
						Cell::Empty =>
							if position.is_opposite_cell(side) {
								game_state.board.update_cell(&position, Cell::Stone(player_index));
								stop = true;
							},
						// The stone is placed in the position previous to a block.
						Cell::Block => {
							if col > 0 {
								game_state.board.update_cell(
									&Coordinates::new(position.row, position.col.saturating_sub(1)),
									Cell::Stone(player_index),
								);
							} else {
								return Err(GameError::InvalidStonePosition)
							}
							stop = true;
						},
						// The stone is placed in the previous position of a stone.
						Cell::Stone(_) => {
							if col < BOARD_WIDTH - 1 {
								game_state.board.update_cell(
									&Coordinates::new(position.row, position.col.saturating_sub(1)),
									Cell::Stone(player_index),
								);
							} else {
								return Err(GameError::InvalidStonePosition)
							}
							stop = true;
						},
					}
					col += 1;
				}
			},
		}

		game_state.last_move = Some(LastMove::new(player, side, position));
		game_state.next_player = game_state.next_player().clone();
		game_state = Self::check_winner_player(game_state);

		Ok(game_state)
	}

	fn check_winner_player(mut game_state: GameState<Player>) -> GameState<Player> {
		if game_state.winner.is_some() {
			return game_state
		}

		let board = &game_state.board;
		let mut squares = [0; NUM_OF_PLAYERS];

		for row in 0..BOARD_HEIGHT - 1 {
			for col in 0..BOARD_WIDTH - 1 {
				let cell = board.get_cell(&Coordinates::new(row, col));
				if let Cell::Stone(player_index) = cell {
					if cell == board.get_cell(&Coordinates::new(row, col + 1)) &&
						cell == board.get_cell(&Coordinates::new(row + 1, col)) &&
						cell == board.get_cell(&Coordinates::new(row + 1, col + 1))
					{
						squares[player_index as usize] += 1;
						if squares[player_index as usize] >= 3 {
							let winner = game_state.players[player_index as usize].clone();
							game_state.winner = Some(winner);
							break
						}
					}
				}
			}
		}

		game_state
	}

	fn hash_coordinates(coordinates: Coordinates, salt: HashSalt) -> HashedCoordinates {
		let mut hashed_coordinates = salt;
		hashed_coordinates.0[30] = coordinates.row;
		hashed_coordinates.0[31] = coordinates.col;
		let choice_hashed = blake2_256(hashed_coordinates.as_bytes());
		choice_hashed.using_encoded(BlakeTwo256::hash)
	}
}

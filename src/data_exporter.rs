use crate::actions::Action;
use crate::simulation_event_handler::SimulationEventHandler;
use crate::state::{GameOutcome, State};
use log::warn;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

/// Struct to hold the exported data point (state, action pair)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedDataPoint {
    pub game_id: String,
    pub ply: u32,
    pub actor: usize,
    pub state: State,
    pub playable_actions: Vec<Action>,
    pub chosen_action: Action,
}

/// The outcome of one exported game, written to `<dir>/<game_id>/result.json`.
///
/// This is the label every exported ply of that game is trained against: the per-ply files hold
/// the position and the action, and this file holds who eventually won it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameResult {
    /// Index of the winning player (0 or 1), or `None` for a tie and for a game that hit the turn
    /// cap without a winner.
    pub winner: Option<usize>,
    /// Final point totals, `[player 0, player 1]`.
    pub points: [u8; 2],
    /// `State::turn_count` of the final state.
    pub turns: u8,
    /// The player who took the first turn (the coin-flip winner), read from `current_player` of
    /// the first exported ply. `None` if the game exported no plys.
    pub starting_player: Option<usize>,
    /// Deck file path given for player 0, when the exporter was told about it.
    pub deck_a: Option<String>,
    /// Deck file path given for player 1, when the exporter was told about it.
    pub deck_b: Option<String>,
}

/// Event handler that exports (state, action) pairs to JSON files
pub struct DataExporter {
    output_folder: PathBuf,
    ply_counter: u32,
    current_game_id: Option<Uuid>,
    deck_a: Option<String>,
    deck_b: Option<String>,
    starting_player: Option<usize>,
}

impl DataExporter {
    pub fn new(output_folder: PathBuf) -> Self {
        Self {
            output_folder,
            ply_counter: 0,
            current_game_id: None,
            deck_a: None,
            deck_b: None,
            starting_player: None,
        }
    }

    /// Record the deck files the two players are using, so `result.json` says which decks the
    /// game was between. Without this the two fields are exported as null.
    pub fn with_decks(mut self, deck_a: impl Into<String>, deck_b: impl Into<String>) -> Self {
        self.deck_a = Some(deck_a.into());
        self.deck_b = Some(deck_b.into());
        self
    }
}

impl SimulationEventHandler for DataExporter {
    fn on_game_start(&mut self, game_id: Uuid) {
        self.current_game_id = Some(game_id);
        self.ply_counter = 0;
        self.starting_player = None;

        // Create folder for this game
        let game_folder = self.output_folder.join(game_id.to_string());
        if let Err(e) = fs::create_dir_all(&game_folder) {
            warn!("Failed to create game folder {:?}: {}", game_folder, e);
        }
    }

    fn on_action(
        &mut self,
        game_id: Uuid,
        state_before_action: &State,
        actor: usize,
        playable_actions: &[Action],
        action: &Action,
    ) {
        let game_folder = self.output_folder.join(game_id.to_string());

        // The first decision of a game is the initial setup placement, which
        // `move_generation::generate_possible_actions` hands to `state.current_player`, i.e. the
        // player the coin flip in `State::initialize` chose to go first.
        if self.ply_counter == 0 {
            self.starting_player = Some(state_before_action.current_player);
        }

        // Create data point
        let data_point = ExportedDataPoint {
            game_id: game_id.to_string(),
            ply: self.ply_counter,
            actor,
            state: state_before_action.clone(),
            playable_actions: playable_actions.to_vec(),
            chosen_action: action.clone(),
        };

        // Write to file
        let file_path = game_folder.join(format!("ply_{:04}.json", self.ply_counter));
        match serde_json::to_string_pretty(&data_point) {
            Ok(json) => {
                if let Err(e) = fs::write(&file_path, json) {
                    warn!("Failed to write ply file {:?}: {}", file_path, e);
                }
            }
            Err(e) => {
                warn!(
                    "Failed to serialize data point for ply {}: {}",
                    self.ply_counter, e
                );
            }
        }

        self.ply_counter += 1;
    }

    fn on_game_end(&mut self, game_id: Uuid, state: State, result: Option<GameOutcome>) {
        let winner = match result {
            Some(GameOutcome::Win(player)) => Some(player),
            Some(GameOutcome::Tie) | None => None,
        };
        let game_result = GameResult {
            winner,
            points: state.points,
            turns: state.turn_count,
            starting_player: self.starting_player,
            deck_a: self.deck_a.clone(),
            deck_b: self.deck_b.clone(),
        };

        let game_folder = self.output_folder.join(game_id.to_string());
        let file_path = game_folder.join("result.json");
        match serde_json::to_string_pretty(&game_result) {
            Ok(json) => {
                if let Err(e) = fs::write(&file_path, json) {
                    warn!("Failed to write result file {:?}: {}", file_path, e);
                }
            }
            Err(e) => {
                warn!("Failed to serialize result for game {}: {}", game_id, e);
            }
        }

        // Reset for next game
        self.ply_counter = 0;
        self.current_game_id = None;
        self.starting_player = None;
    }

    fn on_simulation_end(&mut self) {
        warn!(
            "Data export complete. Data written to: {:?}",
            self.output_folder
        );
    }

    fn merge(&mut self, _other: &dyn SimulationEventHandler) {
        // DataExporter doesn't need to merge data since each thread
        // writes to separate game folders. No aggregation needed.
    }
}

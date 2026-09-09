// Collection of value functions for ExpectiMiniMaxPlayer
//
// Each value function evaluates a game state from a player's perspective
// and returns a score (higher is better for that player)

use log::trace;

use crate::card_logic::get_highest_evolutions;
use crate::hooks::energy_missing;
use crate::models::{Card, EnergyType, PlayedCard};
use crate::state::GameOutcome;
use crate::State;

/// Coefficients for the parametric value function
#[derive(Debug, Clone, Copy)]
pub struct ValueFunctionParams {
    pub points: f64,
    pub pokemon_value: f64,
    pub hand_size: f64,
    pub deck_size: f64,
    pub active_retreat_cost: f64,
    pub active_pokemon_online_score: f64,
    pub active_safety: f64,
    pub active_has_tool: f64,
    pub is_winner: f64,
    pub turns_until_opponent_wins: f64,
    pub online_pokemon_count: f64,
    pub energy_distance_to_online: f64,
    pub opponent_discard_size: f64,
}

impl ValueFunctionParams {
    /// Baseline parameters (same as original baseline function)
    pub const fn baseline() -> Self {
        Self {
            points: 10_000.0,
            pokemon_value: 1.0,
            hand_size: 1.0,
            deck_size: 1.0,
            active_retreat_cost: 1.0,
            active_pokemon_online_score: 500.0,
            active_safety: 1.0,
            active_has_tool: 10.0,
            is_winner: 100_000.0,
            turns_until_opponent_wins: 100.0,
            online_pokemon_count: 0.0,
            energy_distance_to_online: 0.0,
            opponent_discard_size: 0.1,
        }
    }

    /// Variant parameters
    pub const fn variant() -> Self {
        Self {
            points: 10_000.0,
            pokemon_value: 1.0,
            hand_size: 1.0,
            deck_size: 1.0,
            active_retreat_cost: 1.0,
            active_pokemon_online_score: 500.0,
            active_safety: 1.0,
            active_has_tool: 10.0,
            is_winner: 100_000.0,
            turns_until_opponent_wins: 100.0,
            online_pokemon_count: 0.0,
            energy_distance_to_online: 0.0,
            opponent_discard_size: 0.1,
        }
    }
}

pub fn baseline_value_function(state: &State, myself: usize) -> f64 {
    parametric_value_function(state, myself, &ValueFunctionParams::baseline())
}

/// A variant of the baseline value function
pub fn variant_value_function(state: &State, myself: usize) -> f64 {
    parametric_value_function(state, myself, &ValueFunctionParams::variant())
}

/// Parametric value function that uses the provided coefficients
pub fn parametric_value_function(
    state: &State,
    myself: usize,
    params: &ValueFunctionParams,
) -> f64 {
    let opponent = (myself + 1) % 2;
    let (my, opp) = (
        extract_features(state, myself, 1.0),
        extract_features(state, opponent, 1.0),
    );
    let score = (my.points - opp.points) * params.points
        + (my.pokemon_value - opp.pokemon_value) * params.pokemon_value
        + (my.hand_size - opp.hand_size) * params.hand_size
        + (opp.deck_size - my.deck_size) * params.deck_size
        + (-my.active_retreat_cost) * params.active_retreat_cost
        + (my.active_pokemon_online_score - opp.active_pokemon_online_score)
            * params.active_pokemon_online_score
        + (my.active_safety - opp.active_safety) * params.active_safety
        + (my.active_has_tool - opp.active_has_tool) * params.active_has_tool
        + (my.is_winner - opp.is_winner) * params.is_winner
        + (my.turns_until_opponent_wins - opp.turns_until_opponent_wins)
            * params.turns_until_opponent_wins
        + (my.online_pokemon_count - opp.online_pokemon_count) * params.online_pokemon_count
        + (my.energy_distance_to_online - opp.energy_distance_to_online)
            * params.energy_distance_to_online
        + opp.discard_size * params.opponent_discard_size;
    trace!("parametric_value_function: {score} (params: {params:?}, my: {my:?}, opp: {opp:?})");
    score
}

/// Features extracted from a player's game state
#[derive(Debug)]
struct Features {
    points: f64,
    pokemon_value: f64,
    hand_size: f64,
    deck_size: f64,
    active_retreat_cost: f64,
    online_pokemon_count: f64,
    energy_distance_to_online: f64,
    active_pokemon_online_score: f64,
    active_safety: f64,
    active_has_tool: f64,
    is_winner: f64,
    turns_until_opponent_wins: f64,
    discard_size: f64,
}

/// Extract features for a single player
fn extract_features(state: &State, player: usize, active_factor: f64) -> Features {
    let points = state.points[player] as f64;
    let pokemon_value = calculate_pokemon_value(state, player, active_factor);
    let hand_size = state.hands[player].len() as f64;
    let deck_size = state.decks[player].cards.len() as f64;
    let active_retreat_cost = get_active_retreat_cost(state, player) as f64;
    let (online_pokemon_count, energy_distance_to_online) =
        calculate_online_metrics(state, player, active_factor);
    let active_pokemon_online_score = calculate_active_pokemon_online_score(state, player);
    let active_safety = calculate_active_safety(state, player);
    let active_has_tool = get_active_has_tool(state, player);
    let is_winner = check_is_winner(state, player);
    let turns_until_opponent_wins = calculate_turns_until_opponent_wins(state, player);
    let discard_size = state.discard_piles[player].len() as f64;

    Features {
        points,
        pokemon_value,
        hand_size,
        deck_size,
        active_retreat_cost,
        online_pokemon_count,
        energy_distance_to_online,
        active_pokemon_online_score,
        active_safety,
        active_has_tool,
        is_winner,
        turns_until_opponent_wins,
        discard_size,
    }
}

fn get_active_retreat_cost(state: &State, player: usize) -> usize {
    state
        .maybe_get_active(player)
        .map(|card| card.card.get_retreat_cost().map(|rc| rc.len()).unwrap_or(5))
        .unwrap_or(0)
}

/// Check if active pokemon has a tool attached
fn get_active_has_tool(state: &State, player: usize) -> f64 {
    state
        .maybe_get_active(player)
        .map(|card| if card.has_tool_attached() { 1.0 } else { 0.0 })
        .unwrap_or(0.0)
}

/// Check if the player has won the game
fn check_is_winner(state: &State, player: usize) -> f64 {
    match state.winner {
        Some(GameOutcome::Win(winner)) if winner == player => 1.0,
        _ => 0.0,
    }
}

/// Calculate expected turns until opponent wins
/// Uses opponent's active damage and simulates KOs until opponent reaches 3 points
fn calculate_turns_until_opponent_wins(state: &State, player: usize) -> f64 {
    let opponent = (player + 1) % 2;

    // Find the closest pokemon to being able to deal damage (by energy requirements)
    let best_threat = state
        .enumerate_in_play_pokemon(opponent)
        .filter_map(|(_, pokemon)| {
            let best_attack = pokemon
                .card
                .get_attacks()
                .iter()
                .filter(|atk| atk.fixed_damage > 0) // Only consider attacks that deal damage
                .map(|atk| {
                    let missing = energy_missing(pokemon, &atk.energy_required, state, opponent);
                    (atk.fixed_damage, missing.len())
                })
                .min_by_key(|(damage, missing)| (*missing, u32::MAX - damage)); // Prioritize by missing energy, then by damage

            best_attack
        })
        .min_by_key(|(damage, missing)| (*missing, u32::MAX - damage)); // Find pokemon with least missing energy
    let (max_damage, missing_energy) = match best_threat {
        Some((damage, missing)) => (damage as f64, missing),
        None => return 30.0, // No pokemon can deal damage
    };

    let mut total_turns = 0.0;
    let mut opp_points = state.points[opponent];

    // If the best threat still can't attack (missing energy), factor that into the calculation
    total_turns += missing_energy as f64;

    // Calculate turns to KO my active pokemon
    if let Some(my_active) = state.maybe_get_active(player) {
        let turns_to_ko = (my_active.get_remaining_hp() as f64 / max_damage).ceil();
        total_turns += turns_to_ko;
        opp_points += my_active.card.get_knockout_points();
    }

    // Simulate KOing bench pokemon until opponent has 3+ points
    while opp_points < 3 {
        // Find the safest bench pokemon (highest hp / ko_points)
        let safest_bench = state
            .enumerate_bench_pokemon(player)
            .max_by_key(|(_, card)| {
                // if missing 1 point, just do by HP. if missing more than 1 point,
                // do by point yield hp / ko_points
                if opp_points == 2 {
                    card.get_remaining_hp()
                } else {
                    let ko_points = card.card.get_knockout_points().max(1) as u32;
                    card.get_remaining_hp() * 1000 / ko_points
                }
            });

        let Some((_, safest_pokemon)) = safest_bench else {
            break; // No more bench pokemon
        };

        let turns_to_ko = (safest_pokemon.get_remaining_hp() as f64 / max_damage).ceil();
        total_turns += turns_to_ko;
        opp_points += safest_pokemon.card.get_knockout_points();
    }

    total_turns
}

/// Calculate online pokemon metrics: (count of online pokemon, total energy distance to online)
/// A pokemon is "online" if it can use at least one attack
/// Applies active_factor bonus to the active pokemon (position 0)
fn calculate_online_metrics(state: &State, player: usize, active_factor: f64) -> (f64, f64) {
    let (online_count, total_distance) = state
        .enumerate_in_play_pokemon(player)
        .map(|(pos, card)| {
            let min_distance = card
                .card
                .get_attacks()
                .iter()
                .map(|atk| {
                    let missing = energy_missing(card, &atk.energy_required, state, player);
                    missing.len()
                })
                .min()
                .unwrap_or(0);

            let position_factor = if pos == 0 { active_factor } else { 1.0 };

            if min_distance == 0 {
                (position_factor, 0.0) // Online pokemon with position bonus
            } else {
                (0.0, min_distance as f64 * position_factor) // Offline pokemon with weighted distance
            }
        })
        .fold((0.0, 0.0), |(count, dist), (c, d)| (count + c, dist + d));

    (online_count, total_distance)
}

/// Calculate total pokemon value (HP * Energy) for a player
fn calculate_pokemon_value(state: &State, player: usize, active_factor: f64) -> f64 {
    state
        .enumerate_in_play_pokemon(player)
        .map(|(pos, card)| {
            let relevant_energy = get_relevant_energy(state, player, card);
            let hp_energy_product = card.get_remaining_hp() as f64 * (relevant_energy + 1.0);
            if pos == 0 {
                hp_energy_product * active_factor
            } else {
                hp_energy_product
            }
        })
        .sum()
}

/// Helper function to calculate relevant energy for a Pokemon
fn get_relevant_energy(state: &State, player: usize, card: &PlayedCard) -> f64 {
    let most_expensive_attack_cost: Vec<EnergyType> = card
        .card
        .get_attacks()
        .iter()
        .map(|atk| atk.energy_required.clone())
        .max()
        .unwrap_or_default();

    let missing = energy_missing(card, &most_expensive_attack_cost, state, player);

    let total = most_expensive_attack_cost.len() as f64;
    total - missing.len() as f64
}

/// Calculate active safety
/// Defined as remaining HP divided by knockout points
fn calculate_active_safety(state: &State, player: usize) -> f64 {
    let Some(active_pokemon) = state.maybe_get_active(player) else {
        return 0.0; // No safety if no active pokemon
    };

    let ko_points = active_pokemon.card.get_knockout_points() as f64;
    let hp = active_pokemon.get_remaining_hp() as f64;

    hp / ko_points.max(1.0)
}

/// Calculate online score for active pokemon (0.0 to 1.0)
/// Returns 1.0 if the active pokemon has enough energy to use the highest attack
/// of its highest evolution available in deck+hand
fn calculate_active_pokemon_online_score(state: &State, player: usize) -> f64 {
    let Some(active_pokemon) = state.maybe_get_active(player) else {
        return 0.0;
    };

    // Get all cards available in deck + hand. For the opponent this reads their hand and deck
    // only as a UNION, which is public under the known-decklist assumption (deck list minus
    // cards in play and discard); the hand/deck split is never used.
    let mut available_cards: Vec<Card> = state.decks[player].cards.to_vec();
    available_cards.extend(state.hands[player].iter().cloned());

    // Find the highest evolution available
    let highest_evolutions = get_highest_evolutions(&active_pokemon.card, &available_cards);

    // If no evolutions found, use the current card
    let target_card = if highest_evolutions.is_empty() {
        &active_pokemon.card
    } else {
        // Use the first highest evolution (they should all be same stage)
        &highest_evolutions[0]
    };

    // Get the highest attack energy cost from the target card
    let most_expensive_attack_cost: Vec<EnergyType> = target_card
        .get_attacks()
        .iter()
        .map(|atk| atk.energy_required.clone())
        .max()
        .unwrap_or_default();

    if most_expensive_attack_cost.is_empty() {
        return 1.0; // No attack requirements, fully online
    }

    // Calculate how much energy we have vs need
    let missing = energy_missing(active_pokemon, &most_expensive_attack_cost, state, player);
    let total_needed = most_expensive_attack_cost.len() as f64;
    let have = total_needed - missing.len() as f64;

    // Return ratio (0.0 to 1.0)
    (have / total_needed).clamp(0.0, 1.0)
}

// ===========================================================================================
// The learned value function
// ===========================================================================================
//
// A logistic regression over 41 features, trained on labelled positions exported by
// `deckgym simulate --data-output`. The single specification for the feature vector is
// `docs/value-features.md` in the Pocket Lab repo; `sim/learn/features.py` there is the second
// implementation, and `sim/learn/features_crosscheck.py` feeds identical exported states to both
// and requires identical vectors. Nothing below may be reordered: `learned_value_weights.json`
// stores the feature names alongside the weights and a reordering invalidates every trained file.
//
// Two rules make the two implementations agree, and both look wrong in isolation:
//
// * HP is always `PlayedCard::get_board_remaining_hp()` (base + stadium + ability), never
//   `get_effective_total_hp()`, so Tool HP capes are excluded on both sides. Features 34 and 35
//   count the Tools instead. See `docs/value-features.md` section 3.3.
// * "Can knock out this turn" is printed `fixed_damage` plus a flat +20 Weakness bonus, with the
//   attack's effect text ignored entirely and the three engine Weakness refinements (granted
//   types, `NoWeakness`, Bounded Field doubling) deliberately not modelled.

/// The 41 feature names, in the fixed order the weights file and the Python extractor use.
pub const LEARNED_FEATURE_NAMES: [&str; 41] = [
    "point_diff",
    "my_active_hp_remaining",
    "my_active_hp_max",
    "opp_active_hp_remaining",
    "opp_active_hp_max",
    "my_bench_hp_remaining",
    "opp_bench_hp_remaining",
    "my_active_energy",
    "my_bench_energy",
    "opp_active_energy",
    "opp_bench_energy",
    "my_hand_size",
    "opp_hand_size",
    "my_deck_size",
    "opp_deck_size",
    "my_bench_count",
    "opp_bench_count",
    "my_active_can_attack",
    "opp_active_can_attack",
    "my_active_can_ko",
    "opp_active_can_ko",
    "my_active_asleep",
    "my_active_paralyzed",
    "my_active_confused",
    "my_active_poisoned",
    "my_active_burned",
    "opp_active_asleep",
    "opp_active_paralyzed",
    "opp_active_confused",
    "opp_active_poisoned",
    "opp_active_burned",
    "turn_count",
    "is_my_turn",
    "stadium_in_play",
    "my_tools_in_play",
    "opp_tools_in_play",
    "my_active_stage",
    "opp_active_stage",
    "is_winner_diff",
    "active_online_score_diff",
    "turns_until_opponent_wins_diff",
];

/// Number of features in the learned vector.
pub const LEARNED_FEATURE_COUNT: usize = LEARNED_FEATURE_NAMES.len();

/// Index of `is_winner_diff`, the one feature the model cannot learn a weight for.
const IS_WINNER_DIFF_INDEX: usize = 38;

/// Scale factor applied to the win probability by [`LearnedValueFunction::score`].
///
/// The search compares and probability-weights values, so any positive constant gives the same
/// play. 100,000 is chosen to put the learned score on the same scale as
/// `ValueFunctionParams::baseline`'s dominant `is_winner` term, which makes the two functions
/// readable side by side in a debug tree.
pub const LEARNED_VALUE_SCALE: f64 = 100_000.0;

/// The trained coefficients, as stored in `src/players/learned_value_weights.json`.
///
/// The `training` block of that file is metadata and is ignored here.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct LearnedValueWeights {
    /// Feature names, which must equal [`LEARNED_FEATURE_NAMES`] in order.
    pub features: Vec<String>,
    /// Per-feature training mean, subtracted before scaling.
    pub means: Vec<f64>,
    /// Per-feature training standard deviation, all strictly positive.
    pub stds: Vec<f64>,
    /// Per-feature logistic-regression coefficient.
    pub weights: Vec<f64>,
    /// Logistic-regression intercept.
    pub bias: f64,
}

impl LearnedValueWeights {
    /// Checks the shape and the name order. Returns the first problem found.
    pub fn validate(&self) -> Result<(), String> {
        for (label, len) in [
            ("features", self.features.len()),
            ("means", self.means.len()),
            ("stds", self.stds.len()),
            ("weights", self.weights.len()),
        ] {
            if len != LEARNED_FEATURE_COUNT {
                return Err(format!(
                    "{label} has {len} entries, expected {LEARNED_FEATURE_COUNT}"
                ));
            }
        }
        for (index, (found, expected)) in self
            .features
            .iter()
            .zip(LEARNED_FEATURE_NAMES.iter())
            .enumerate()
        {
            if found != expected {
                return Err(format!(
                    "feature {index} is {found:?}, expected {expected:?}; the weights file was \
                     trained against a different feature order"
                ));
            }
        }
        for (index, std) in self.stds.iter().enumerate() {
            if !std.is_finite() || *std <= 0.0 {
                return Err(format!(
                    "std {index} ({}) is not a positive finite number",
                    LEARNED_FEATURE_NAMES[index]
                ));
            }
        }
        if !self.bias.is_finite() {
            return Err("bias is not finite".to_string());
        }
        Ok(())
    }

    /// `sigmoid(dot((x - means) / stds, weights) + bias)` as a logit.
    pub fn logit(&self, features: &[f64]) -> f64 {
        features
            .iter()
            .zip(self.means.iter())
            .zip(self.stds.iter())
            .zip(self.weights.iter())
            .map(|(((x, mean), std), weight)| ((x - mean) / std) * weight)
            .sum::<f64>()
            + self.bias
    }
}

const LEARNED_VALUE_WEIGHTS_JSON: &str = include_str!("learned_value_weights.json");

static LEARNED_VALUE_WEIGHTS: std::sync::LazyLock<LearnedValueWeights> =
    std::sync::LazyLock::new(|| {
        let parsed: LearnedValueWeights = serde_json::from_str(LEARNED_VALUE_WEIGHTS_JSON)
            .expect("src/players/learned_value_weights.json is not valid weights JSON");
        if let Err(problem) = parsed.validate() {
            panic!("src/players/learned_value_weights.json is unusable: {problem}");
        }
        parsed
    });

/// The logistic value function trained in step 1 of the bot plan.
///
/// Scores a position from `myself`'s point of view as a win probability rather than as an
/// unbounded hand-tuned sum, which is what makes it correct to average over the chance nodes the
/// expectiminimax search expands: the expectation of a win probability is a win probability.
#[derive(Debug, Clone, Copy)]
pub struct LearnedValueFunction {
    weights: &'static LearnedValueWeights,
}

impl Default for LearnedValueFunction {
    fn default() -> Self {
        Self::new()
    }
}

impl LearnedValueFunction {
    /// Binds to the weights embedded at compile time. Panics on the first call if that file is
    /// malformed or was trained against a different feature order.
    pub fn new() -> Self {
        Self {
            weights: &LEARNED_VALUE_WEIGHTS,
        }
    }

    /// The trained coefficients this instance reads.
    pub fn weights(&self) -> &'static LearnedValueWeights {
        self.weights
    }

    /// The 41 features, in [`LEARNED_FEATURE_NAMES`] order.
    pub fn features(&self, state: &State, myself: usize) -> Vec<f64> {
        learned_value_features(state, myself)
    }

    /// The model's logit for a position: positive means `myself` is favoured.
    pub fn logit(&self, state: &State, myself: usize) -> f64 {
        self.weights.logit(&self.features(state, myself))
    }

    /// The model's probability that `myself` eventually wins, in `(0, 1)`.
    pub fn win_probability(&self, state: &State, myself: usize) -> f64 {
        sigmoid(self.logit(state, myself))
    }

    /// The score the search uses: `LEARNED_VALUE_SCALE * (win probability + is_winner_diff)`.
    ///
    /// The `is_winner_diff` term is not a fudge factor, it is the one feature the model cannot
    /// learn. Training positions are by construction positions where a decision was still to be
    /// made, so feature 38 is 0 in every row and L2 leaves its weight at exactly 0. The search,
    /// unlike the training set, does reach terminal states and scores them with this same
    /// function, so a decided game has to be separated from an undecided one here. Adding
    /// `is_winner_diff` (which is -1, 0 or +1) puts every won position above every undecided one
    /// and every lost position below, exactly as the baseline's `is_winner` x 100,000 term does
    /// against its `points` x 10,000 term. A Tie scores 0 for both sides, again as the baseline
    /// does.
    pub fn score(&self, state: &State, myself: usize) -> f64 {
        let features = self.features(state, myself);
        let probability = sigmoid(self.weights.logit(&features));
        LEARNED_VALUE_SCALE * (probability + features[IS_WINNER_DIFF_INDEX])
    }
}

/// The logistic link, written so that a large negative logit cannot overflow `exp`.
pub fn sigmoid(z: f64) -> f64 {
    if z >= 0.0 {
        1.0 / (1.0 + (-z).exp())
    } else {
        let e = z.exp();
        e / (1.0 + e)
    }
}

/// The learned value function as a plain `fn`, for `ExpectiMiniMaxPlayer::value_function`.
pub fn learned_value_function(state: &State, myself: usize) -> f64 {
    LearnedValueFunction::new().score(state, myself)
}

// -------------------------------------------------------------------------------------------
// Feature extraction
// -------------------------------------------------------------------------------------------

/// The 41 features of `docs/value-features.md`, from `myself`'s point of view.
///
/// Public because `src/bin/value_features.rs` prints these for an exported ply file so the Python
/// extractor can be cross-checked against them.
pub fn learned_value_features(state: &State, myself: usize) -> Vec<f64> {
    let opponent = (myself + 1) % 2;

    let my_active = state.maybe_get_active(myself);
    let opp_active = state.maybe_get_active(opponent);

    let my_best = best_printed_damage(my_active, opp_active);
    let opp_best = best_printed_damage(opp_active, my_active);

    let is_winner_diff = match state.winner {
        Some(GameOutcome::Win(winner)) if winner == myself => 1.0,
        Some(GameOutcome::Win(winner)) if winner == opponent => -1.0,
        _ => 0.0,
    };

    let mut features = Vec::with_capacity(LEARNED_FEATURE_COUNT);
    features.push(state.points[myself] as f64 - state.points[opponent] as f64); // 0
    features.push(board_remaining_hp(my_active)); // 1
    features.push(board_total_hp(my_active)); // 2
    features.push(board_remaining_hp(opp_active)); // 3
    features.push(board_total_hp(opp_active)); // 4
    features.push(
        bench(state, myself)
            .map(|c| board_remaining_hp(Some(c)))
            .sum(),
    ); // 5
    features.push(
        bench(state, opponent)
            .map(|c| board_remaining_hp(Some(c)))
            .sum(),
    ); // 6
    features.push(my_active.map_or(0, |c| effective_energy(c).len()) as f64); // 7
    features.push(
        bench(state, myself)
            .map(|c| effective_energy(c).len())
            .sum::<usize>() as f64,
    ); // 8
    features.push(opp_active.map_or(0, |c| effective_energy(c).len()) as f64); // 9
    features.push(
        bench(state, opponent)
            .map(|c| effective_energy(c).len())
            .sum::<usize>() as f64,
    ); // 10
    features.push(state.hands[myself].len() as f64); // 11
    features.push(state.hands[opponent].len() as f64); // 12
    features.push(state.decks[myself].cards.len() as f64); // 13
    features.push(state.decks[opponent].cards.len() as f64); // 14
    features.push(bench(state, myself).count() as f64); // 15
    features.push(bench(state, opponent).count() as f64); // 16
    features.push(f64::from(my_best.is_some())); // 17
    features.push(f64::from(opp_best.is_some())); // 18
    features.push(f64::from(can_knock_out(my_best, opp_active))); // 19
    features.push(f64::from(can_knock_out(opp_best, my_active))); // 20
    features.extend(status_conditions(my_active)); // 21 to 25
    features.extend(status_conditions(opp_active)); // 26 to 30
    features.push(state.turn_count as f64); // 31
    features.push(f64::from(state.current_player == myself)); // 32
    features.push(f64::from(state.active_stadium.is_some())); // 33
    features.push(tools_in_play(state, myself)); // 34
    features.push(tools_in_play(state, opponent)); // 35
    features.push(active_stage(my_active)); // 36
    features.push(active_stage(opp_active)); // 37
    features.push(is_winner_diff); // 38
    features.push(active_online_score(state, myself) - active_online_score(state, opponent)); // 39
    features.push(
        turns_until_opponent_wins_board_hp(state, myself)
            - turns_until_opponent_wins_board_hp(state, opponent),
    ); // 40

    debug_assert_eq!(features.len(), LEARNED_FEATURE_COUNT);
    features
}

/// The three Bench slots that hold a Pokemon, in slot order.
fn bench(state: &State, player: usize) -> impl Iterator<Item = &PlayedCard> {
    state.in_play_pokemon[player][1..].iter().flatten()
}

/// Every in-play Pokemon of a player, Active first.
fn in_play(state: &State, player: usize) -> impl Iterator<Item = &PlayedCard> {
    state.in_play_pokemon[player].iter().flatten()
}

fn board_total_hp(card: Option<&PlayedCard>) -> f64 {
    card.map_or(0, |c| c.get_board_total_hp()) as f64
}

fn board_remaining_hp(card: Option<&PlayedCard>) -> f64 {
    card.map_or(0, |c| c.get_board_remaining_hp()) as f64
}

/// `attached_energy` with every Grass entry duplicated when the card's cached
/// `double_grass_active` flag is set. Reads the flag rather than recomputing it from the board, so
/// that this matches what the Python extractor sees in an exported state.
fn effective_energy(card: &PlayedCard) -> Vec<EnergyType> {
    if !card.has_double_grass_active() {
        return card.attached_energy.clone();
    }
    let mut doubled = Vec::with_capacity(card.attached_energy.len() * 2);
    for energy in &card.attached_energy {
        doubled.push(*energy);
        if *energy == EnergyType::Grass {
            doubled.push(EnergyType::Grass);
        }
    }
    doubled
}

/// The feature vector's own copy of `hooks::core::energy_missing`, taking the effective attached
/// energy as a plain list instead of reading it back off the live board.
fn feature_energy_missing(attached: &[EnergyType], cost: &[EnergyType]) -> Vec<EnergyType> {
    let mut pool: Vec<EnergyType> = attached.to_vec();
    let mut missing = vec![];
    for energy in cost.iter().filter(|e| **e != EnergyType::Colorless) {
        match pool.iter().position(|held| held == energy) {
            Some(index) => {
                pool.remove(index);
            }
            None => missing.push(*energy),
        }
    }
    let colorless_needed = cost.iter().filter(|e| **e == EnergyType::Colorless).count();
    missing.extend(vec![
        EnergyType::Colorless;
        colorless_needed.saturating_sub(pool.len())
    ]);
    missing
}

fn cost_met(card: &PlayedCard, cost: &[EnergyType]) -> bool {
    feature_energy_missing(&effective_energy(card), cost).is_empty()
}

/// A flat +20 when the defender's printed Weakness equals the attacker's printed Energy type.
fn weakness_bonus(attacker: &PlayedCard, defender: &PlayedCard) -> f64 {
    let (Card::Pokemon(attacking), Card::Pokemon(defending)) = (&attacker.card, &defender.card)
    else {
        return 0.0;
    };
    match defending.weakness {
        Some(weakness) if weakness == attacking.energy_type => 20.0,
        _ => 0.0,
    }
}

/// The most damage the attacker's cost-met attacks print, plus the Weakness bonus. `None` when
/// there is no attacker or it cannot pay for any attack.
fn best_printed_damage(
    attacker: Option<&PlayedCard>,
    defender: Option<&PlayedCard>,
) -> Option<f64> {
    let attacker = attacker?;
    let bonus = defender.map_or(0.0, |d| weakness_bonus(attacker, d));
    attacker
        .card
        .get_attacks()
        .iter()
        .filter(|attack| cost_met(attacker, &attack.energy_required))
        .map(|attack| f64::from(attack.fixed_damage) + bonus)
        .fold(None, |best: Option<f64>, damage| {
            Some(best.map_or(damage, |b| b.max(damage)))
        })
}

fn can_knock_out(best_damage: Option<f64>, defender: Option<&PlayedCard>) -> bool {
    match (best_damage, defender) {
        (Some(damage), Some(defender)) => damage >= board_remaining_hp(Some(defender)),
        _ => false,
    }
}

/// Asleep, Paralyzed, Confused, Poisoned, Burned, in that order.
fn status_conditions(card: Option<&PlayedCard>) -> [f64; 5] {
    match card {
        None => [0.0; 5],
        Some(card) => [
            f64::from(card.is_asleep()),
            f64::from(card.is_paralyzed()),
            f64::from(card.is_confused()),
            f64::from(card.is_poisoned()),
            f64::from(card.is_burned()),
        ],
    }
}

fn tools_in_play(state: &State, player: usize) -> f64 {
    in_play(state, player)
        .map(|card| card.attached_tools.len())
        .sum::<usize>() as f64
}

fn active_stage(card: Option<&PlayedCard>) -> f64 {
    match card.map(|c| &c.card) {
        Some(Card::Pokemon(pokemon)) => f64::from(pokemon.stage),
        _ => 0.0,
    }
}

/// The most expensive attack cost of a card, comparing `Vec<EnergyType>` the way Rust's `max()`
/// does: element by element in the enum's declaration order, longer beats its own prefix.
fn most_expensive_attack_cost(card: &Card) -> Vec<EnergyType> {
    card.get_attacks()
        .iter()
        .map(|attack| attack.energy_required.clone())
        .max()
        .unwrap_or_default()
}

/// Feature 39's half: a port of [`calculate_active_pokemon_online_score`] that reads energy
/// through [`effective_energy`] instead of the live board, so the Python extractor can reproduce
/// it from an exported state.
fn active_online_score(state: &State, player: usize) -> f64 {
    let Some(active) = state.maybe_get_active(player) else {
        return 0.0;
    };
    // Deck plus hand as a union only; public under the known-decklist assumption (see
    // calculate_active_pokemon_online_score).
    let mut available: Vec<Card> = state.decks[player].cards.to_vec();
    available.extend(state.hands[player].iter().cloned());

    let highest_evolutions = get_highest_evolutions(&active.card, &available);
    let target = highest_evolutions.first().unwrap_or(&active.card);

    let cost = most_expensive_attack_cost(target);
    if cost.is_empty() {
        return 1.0;
    }
    let missing = feature_energy_missing(&effective_energy(active), &cost);
    let have = cost.len() as f64 - missing.len() as f64;
    (have / cost.len() as f64).clamp(0.0, 1.0)
}

/// Feature 40's half: a port of [`calculate_turns_until_opponent_wins`] on board HP.
///
/// Two engine quirks are reproduced rather than fixed, because this feature is defined as "what
/// the engine's existing term computes": the Bench scan never removes the Pokemon it just counted,
/// so one Bench Pokemon can be counted twice, and `max_by_key` keeps the last of several equal
/// maxima while `min_by_key` keeps the first.
fn turns_until_opponent_wins_board_hp(state: &State, player: usize) -> f64 {
    let opponent = (player + 1) % 2;

    let best_threat = in_play(state, opponent)
        .filter_map(|pokemon| {
            let attached = effective_energy(pokemon);
            pokemon
                .card
                .get_attacks()
                .iter()
                .filter(|attack| attack.fixed_damage > 0)
                .map(|attack| {
                    let missing = feature_energy_missing(&attached, &attack.energy_required);
                    (attack.fixed_damage, missing.len())
                })
                .min_by_key(|(damage, missing)| (*missing, u32::MAX - damage))
        })
        .min_by_key(|(damage, missing)| (*missing, u32::MAX - damage));

    let (max_damage, missing_energy) = match best_threat {
        Some((damage, missing)) => (f64::from(damage), missing),
        None => return 30.0, // No pokemon can deal damage.
    };

    let mut total_turns = missing_energy as f64;
    let mut opp_points = state.points[opponent];

    if let Some(my_active) = state.maybe_get_active(player) {
        total_turns += (board_remaining_hp(Some(my_active)) / max_damage).ceil();
        opp_points += my_active.card.get_knockout_points();
    }

    while opp_points < 3 {
        let safest = bench(state, player).max_by_key(|card| {
            let hp = card.get_board_remaining_hp();
            if opp_points == 2 {
                hp
            } else {
                hp * 1000 / u32::from(card.card.get_knockout_points().max(1))
            }
        });
        let Some(safest) = safest else {
            break; // No more bench pokemon.
        };
        total_turns += (board_remaining_hp(Some(safest)) / max_damage).ceil();
        opp_points += safest.card.get_knockout_points();
    }

    total_turns
}

// -------------------------------------------------------------------------------------------
// Selecting a value function
// -------------------------------------------------------------------------------------------

/// Which scorer a search player uses.
///
/// The factory below hands out a boxed `ValueFunction` for each, so a player that owns a
/// `ValueFunction` (`ExpectiMiniMaxPlayer` today, the opponent-aware and MCTS players of the next
/// steps of the bot plan) can be built with either scorer without knowing how it is put together.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ValueFunctionKind {
    /// [`baseline_value_function`], the hand-tuned linear scorer. The default.
    #[default]
    Baseline,
    /// [`variant_value_function`].
    Variant,
    /// [`learned_value_function`], the logistic model of step 1 of the bot plan.
    Learned,
}

impl ValueFunctionKind {
    /// The short name accepted by [`parse_value_function_kind`].
    pub fn as_str(&self) -> &'static str {
        match self {
            ValueFunctionKind::Baseline => "baseline",
            ValueFunctionKind::Variant => "variant",
            ValueFunctionKind::Learned => "learned",
        }
    }
}

/// Case-insensitive name to kind, for command-line flags.
pub fn parse_value_function_kind(name: &str) -> Result<ValueFunctionKind, String> {
    match name.to_ascii_lowercase().as_str() {
        "baseline" | "b" => Ok(ValueFunctionKind::Baseline),
        "variant" | "v" => Ok(ValueFunctionKind::Variant),
        "learned" | "l" => Ok(ValueFunctionKind::Learned),
        other => Err(format!(
            "Invalid value function: {other}. Use baseline, variant or learned"
        )),
    }
}

/// A boxed scorer for the given kind.
pub fn build_value_function(kind: ValueFunctionKind) -> super::ValueFunction {
    match kind {
        ValueFunctionKind::Baseline => Box::new(baseline_value_function),
        ValueFunctionKind::Variant => Box::new(variant_value_function),
        ValueFunctionKind::Learned => Box::new(learned_value_function),
    }
}

#[cfg(test)]
mod learned_value_tests {
    use super::*;
    use crate::card_ids::CardId;
    use crate::database::get_card_by_enum;
    use crate::models::StatusCondition;
    use crate::state::PlayedCard;

    /// A board with nothing on it, so every feature reads its "not there" value.
    fn empty_state() -> State {
        State::default()
    }

    /// Riolu (A2 091) against Jigglypuff (A1 193), both sides benched and energised.
    ///
    /// Every card fact below is read from `database.json`:
    /// Riolu A2 091, Basic, 60 HP, [F] type, Weakness [P], one attack Jab [F] for 20.
    /// Jigglypuff A1 193, Basic, 60 HP, [C] type, Weakness [F], one attack Pound [C][C] for 30.
    /// Rattata A1 189, Basic, 40 HP, [C] type, Weakness [F], one attack Gnaw [C] for 20.
    fn riolu_vs_jigglypuff() -> State {
        let mut state = State::default();
        state.points = [1, 2];
        state.turn_count = 7;
        state.current_player = 0;

        state.in_play_pokemon[0][0] = Some(
            PlayedCard::from_id(CardId::A2091Riolu)
                .with_energy(vec![EnergyType::Fighting])
                .with_damage(30)
                // A Giant Cape is worth +20 effective HP in the engine and 0 HP here, on purpose:
                // features 1 and 2 are board HP, and feature 34 counts the Tool instead.
                .with_tool(get_card_by_enum(CardId::A2147GiantCape)),
        );
        state.in_play_pokemon[0][1] = Some(PlayedCard::from_id(CardId::A1189Rattata));

        state.in_play_pokemon[1][0] = Some(
            PlayedCard::from_id(CardId::A1193Jigglypuff)
                .with_energy(vec![EnergyType::Psychic])
                .with_damage(30),
        );
        state.in_play_pokemon[1][2] = Some(
            PlayedCard::from_id(CardId::A1189Rattata)
                .with_energy(vec![EnergyType::Psychic, EnergyType::Psychic]),
        );
        state
    }

    /// Player 0 has already won: 3 points, a healthy Active with two Special Conditions, against
    /// a Rattata on 10 HP. Player 1's view of the same board is the losing side.
    fn decided_state() -> State {
        let mut state = State::default();
        state.points = [3, 0];
        state.turn_count = 12;
        state.current_player = 1;
        state.winner = Some(GameOutcome::Win(0));
        state.active_stadium = Some(get_card_by_enum(CardId::B3155BoundedField));

        let mut riolu =
            PlayedCard::from_id(CardId::A2091Riolu).with_energy(vec![EnergyType::Fighting]);
        riolu.set_status_raw(StatusCondition::Asleep);
        riolu.set_status_raw(StatusCondition::Poisoned);
        state.in_play_pokemon[0][0] = Some(riolu);

        state.in_play_pokemon[1][0] =
            Some(PlayedCard::from_id(CardId::A1189Rattata).with_damage(30));
        state
    }

    #[test]
    fn weights_file_matches_the_feature_contract() {
        let weights = LearnedValueFunction::new().weights();
        weights.validate().expect("embedded weights are valid");
        assert_eq!(weights.features.len(), LEARNED_FEATURE_COUNT);
        assert_eq!(weights.features, LEARNED_FEATURE_NAMES.to_vec());
        assert_eq!(
            LEARNED_FEATURE_NAMES[IS_WINNER_DIFF_INDEX],
            "is_winner_diff"
        );
    }

    #[test]
    fn rejects_a_weights_file_with_the_wrong_feature_order() {
        let mut weights = LearnedValueFunction::new().weights().clone();
        weights.features.swap(0, 1);
        let problem = weights.validate().expect_err("a swapped order is rejected");
        assert!(
            problem.contains("feature 0"),
            "unexpected message: {problem}"
        );
    }

    #[test]
    fn features_on_an_empty_board() {
        let features = learned_value_features(&empty_state(), 0);
        let mut expected = vec![0.0; LEARNED_FEATURE_COUNT];
        // Only `is_my_turn` is true: State::default() leaves current_player at 0. Feature 40 is
        // 0 because neither side has a Pokemon that can deal damage, so both halves are 30.
        expected[32] = 1.0;
        assert_eq!(features, expected);
    }

    #[test]
    fn features_on_a_two_sided_board() {
        let features = learned_value_features(&riolu_vs_jigglypuff(), 0);
        let expected = vec![
            -1.0, //  0 point_diff: 1 - 2
            30.0, //  1 my_active_hp_remaining: 60 base - 30 damage, the Giant Cape not counted
            60.0, //  2 my_active_hp_max: Riolu's printed 60, the Giant Cape not counted
            30.0, //  3 opp_active_hp_remaining: Jigglypuff 60 - 30
            60.0, //  4 opp_active_hp_max
            40.0, //  5 my_bench_hp_remaining: one undamaged Rattata
            40.0, //  6 opp_bench_hp_remaining: one undamaged Rattata
            1.0,  //  7 my_active_energy: one [F]
            0.0,  //  8 my_bench_energy
            1.0,  //  9 opp_active_energy: one [P]
            2.0,  // 10 opp_bench_energy: two [P] on the benched Rattata
            0.0,  // 11 my_hand_size
            0.0,  // 12 opp_hand_size
            0.0,  // 13 my_deck_size
            0.0,  // 14 opp_deck_size
            1.0,  // 15 my_bench_count
            1.0,  // 16 opp_bench_count: slot 2, so a null in slot 1 is skipped
            1.0,  // 17 my_active_can_attack: Jab costs [F] and one [F] is attached
            0.0,  // 18 opp_active_can_attack: Pound costs [C][C] and only one energy is attached
            1.0,  // 19 my_active_can_ko: 20 printed + 20 Weakness ([F] vs Jigglypuff's) >= 30
            0.0,  // 20 opp_active_can_ko: no cost-met attack at all
            0.0,  // 21 my_active_asleep
            0.0,  // 22 my_active_paralyzed
            0.0,  // 23 my_active_confused
            0.0,  // 24 my_active_poisoned
            0.0,  // 25 my_active_burned
            0.0,  // 26 opp_active_asleep
            0.0,  // 27 opp_active_paralyzed
            0.0,  // 28 opp_active_confused
            0.0,  // 29 opp_active_poisoned
            0.0,  // 30 opp_active_burned
            7.0,  // 31 turn_count
            1.0,  // 32 is_my_turn
            0.0,  // 33 stadium_in_play
            1.0,  // 34 my_tools_in_play: the Giant Cape
            0.0,  // 35 opp_tools_in_play
            0.0,  // 36 my_active_stage: Riolu is Basic
            0.0,  // 37 opp_active_stage: Jigglypuff is Basic
            0.0,  // 38 is_winner_diff
            0.5,  // 39 online score: Riolu pays its whole [F] cost (1.0), Jigglypuff pays one
            //         of Pound's two (0.5), and the deck and hand are empty so neither has an
            //         evolution to aim at
            -2.0, // 40 turns_until_opponent_wins: the benched Rattata is the fastest threat on
                  //    both sides at 20 a turn. Against me that is ceil(30/20) = 2 turns for the
                  //    opponent's third point. Against the opponent it is ceil(30/20) = 2 for
                  //    Jigglypuff plus ceil(40/20) = 2 for the Bench, so 4. 2 - 4 = -2.
        ];
        assert_eq!(features, expected);
    }

    #[test]
    fn features_on_a_decided_board() {
        let features = learned_value_features(&decided_state(), 0);
        let expected = vec![
            3.0,  //  0 point_diff: 3 - 0
            60.0, //  1 my_active_hp_remaining: an undamaged Riolu
            60.0, //  2 my_active_hp_max
            10.0, //  3 opp_active_hp_remaining: Rattata 40 - 30
            40.0, //  4 opp_active_hp_max
            0.0,  //  5 my_bench_hp_remaining: no Bench
            0.0,  //  6 opp_bench_hp_remaining
            1.0,  //  7 my_active_energy
            0.0,  //  8 my_bench_energy
            0.0,  //  9 opp_active_energy
            0.0,  // 10 opp_bench_energy
            0.0,  // 11 my_hand_size
            0.0,  // 12 opp_hand_size
            0.0,  // 13 my_deck_size
            0.0,  // 14 opp_deck_size
            0.0,  // 15 my_bench_count
            0.0,  // 16 opp_bench_count
            1.0,  // 17 my_active_can_attack
            0.0,  // 18 opp_active_can_attack: Gnaw costs [C] and nothing is attached
            1.0,  // 19 my_active_can_ko: 20 + 20 Weakness ([F] vs Rattata's) >= 10
            0.0,  // 20 opp_active_can_ko
            1.0,  // 21 my_active_asleep
            0.0,  // 22 my_active_paralyzed
            0.0,  // 23 my_active_confused
            1.0,  // 24 my_active_poisoned
            0.0,  // 25 my_active_burned
            0.0,  // 26 opp_active_asleep
            0.0,  // 27 opp_active_paralyzed
            0.0,  // 28 opp_active_confused
            0.0,  // 29 opp_active_poisoned
            0.0,  // 30 opp_active_burned
            12.0, // 31 turn_count
            0.0,  // 32 is_my_turn: current_player is 1
            1.0,  // 33 stadium_in_play: Bounded Field. Its Weakness doubling is deliberately not
            //         modelled, so feature 19 stays a flat +20 (docs/value-features.md 1.1)
            0.0, // 34 my_tools_in_play
            0.0, // 35 opp_tools_in_play
            0.0, // 36 my_active_stage
            0.0, // 37 opp_active_stage
            1.0, // 38 is_winner_diff: player 0 has won
            1.0, // 39 online score: Riolu 1.0, Rattata 0.0 (Gnaw's [C] unpaid)
            3.0, // 40 turns_until_opponent_wins: Rattata needs 1 turn of energy plus
                 //    ceil(60/20) = 3 to take my Riolu, so 4; my Riolu needs ceil(10/20) = 1 and
                 //    the opponent is already at 3 points so no Bench pass runs, so 1. 4 - 1 = 3.
        ];
        assert_eq!(features, expected);
    }

    #[test]
    fn double_grass_doubles_the_attached_grass_energy() {
        // Bulbasaur A1 001: Basic, 70 HP, [G], one attack Vine Whip [G][C] for 40. One [G] on its
        // own does not pay for it; Serperior's Jungle Totem doubling it does.
        let mut state = State::default();
        state.in_play_pokemon[0][0] =
            Some(PlayedCard::from_id(CardId::A1001Bulbasaur).with_energy(vec![EnergyType::Grass]));

        let plain = learned_value_features(&state, 0);
        assert_eq!(plain[7], 1.0, "my_active_energy without the doubling");
        assert_eq!(plain[17], 0.0, "one [G] does not pay for [G][C]");

        state.in_play_pokemon[0][0]
            .as_mut()
            .expect("active is set")
            .refresh_double_grass_active(true);

        let doubled = learned_value_features(&state, 0);
        assert_eq!(doubled[7], 2.0, "my_active_energy with the doubling");
        assert_eq!(doubled[17], 1.0, "two effective [G] pay for [G][C]");
    }

    #[test]
    fn the_learned_function_ranks_a_won_position_above_a_lost_one() {
        let state = decided_state();
        let learned = LearnedValueFunction::new();

        let winner = learned.score(&state, 0);
        let loser = learned.score(&state, 1);
        assert!(
            winner > loser,
            "the won side scored {winner} and the lost side {loser}"
        );
        // A decided game has to sit outside the range an undecided one can reach, or the search
        // would trade a win for a comfortable position.
        assert!(winner > LEARNED_VALUE_SCALE, "won side scored {winner}");
        assert!(loser < 0.0, "lost side scored {loser}");
    }

    #[test]
    fn the_learned_function_ranks_a_strong_undecided_position_above_a_weak_one() {
        let mut ahead = riolu_vs_jigglypuff();
        ahead.points = [2, 0];
        let mut behind = ahead.clone();
        behind.points = [0, 2];

        let learned = LearnedValueFunction::new();
        let strong = learned.score(&ahead, 0);
        let weak = learned.score(&behind, 0);
        assert!(strong > weak, "ahead scored {strong}, behind scored {weak}");
        // Both are undecided, so both stay inside the probability band.
        for score in [strong, weak] {
            assert!((0.0..=LEARNED_VALUE_SCALE).contains(&score), "{score}");
        }
    }

    #[test]
    fn win_probability_and_logit_agree() {
        let state = riolu_vs_jigglypuff();
        let learned = LearnedValueFunction::new();
        let logit = learned.logit(&state, 0);
        let probability = learned.win_probability(&state, 0);
        assert!((probability - sigmoid(logit)).abs() < 1e-12);
        assert!((0.0..=1.0).contains(&probability));
        // An undecided position scores exactly the scaled probability.
        assert!((learned.score(&state, 0) - LEARNED_VALUE_SCALE * probability).abs() < 1e-6);
    }

    #[test]
    fn sigmoid_does_not_overflow_at_the_extremes() {
        assert_eq!(sigmoid(0.0), 0.5);
        assert!(sigmoid(-800.0) >= 0.0);
        assert!(sigmoid(800.0) <= 1.0);
        assert!(sigmoid(-800.0) < sigmoid(800.0));
    }

    #[test]
    fn the_value_function_factory_hands_out_each_kind() {
        let state = riolu_vs_jigglypuff();
        for (kind, name) in [
            (ValueFunctionKind::Baseline, "baseline"),
            (ValueFunctionKind::Variant, "variant"),
            (ValueFunctionKind::Learned, "learned"),
        ] {
            assert_eq!(kind.as_str(), name);
            assert_eq!(parse_value_function_kind(name).unwrap(), kind);
            assert_eq!(
                parse_value_function_kind(&name.to_uppercase()).unwrap(),
                kind
            );
            let scorer = build_value_function(kind);
            assert!(scorer(&state, 0).is_finite());
        }
        assert_eq!(
            parse_value_function_kind("b").unwrap(),
            ValueFunctionKind::Baseline
        );
        assert_eq!(
            parse_value_function_kind("l").unwrap(),
            ValueFunctionKind::Learned
        );
        assert!(parse_value_function_kind("nope").is_err());
        assert_eq!(ValueFunctionKind::default(), ValueFunctionKind::Baseline);
    }
}

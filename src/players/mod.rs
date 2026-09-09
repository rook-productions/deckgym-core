mod attach_attack_player;
mod end_turn_player;
mod evolution_rusher_player;
mod expectiminimax_player;
mod human_player;
mod mcts_player;
mod opponent_aware_player;
mod random_player;
mod value_function_player;
pub mod value_functions;
mod weighted_random_player;

pub use attach_attack_player::AttachAttackPlayer;
pub use end_turn_player::EndTurnPlayer;
pub use evolution_rusher_player::EvolutionRusherPlayer;
pub use expectiminimax_player::{ExpectiMiniMaxPlayer, ValueFunction};
pub use human_player::HumanPlayer;
pub use mcts_player::MctsPlayer;
#[cfg(feature = "test-utils")]
pub use opponent_aware_player::determinise_opponent_hand_for_test;
pub use opponent_aware_player::{OpponentAwarePlayer, ReplyPolicy};
pub use random_player::RandomPlayer;
pub use value_function_player::ValueFunctionPlayer;
pub use value_functions::*;
pub use weighted_random_player::WeightedRandomPlayer;

use crate::{actions::Action, Deck, State};
use rand::rngs::StdRng;
use std::fmt::Debug;

pub trait Player: Debug {
    fn get_deck(&self) -> Deck;
    fn decision_fn(
        &mut self,
        rng: &mut StdRng,
        state: &State,
        possible_actions: &[Action],
    ) -> Action;
}

/// Enum for allowed player strategies
#[derive(Debug, Clone, PartialEq)]
pub enum PlayerCode {
    AA,
    ET,
    R,
    H,
    W,
    M,
    V,
    E {
        max_depth: usize,
    },
    ER, // Evolution Rusher
    /// ExpectiMiniMax with the learned value function (step 1 of the bot plan).
    L {
        max_depth: usize,
    },
    /// Opponent-aware search: own turn searched to `max_depth` as `E` does, then each end-of-turn
    /// position is scored by averaging over `samples` determinisations of the opponent's hidden
    /// hand, each played out for one full reply turn.
    O {
        max_depth: usize,
        samples: usize,
    },
}

/// Player codes accepted by `--players`, and what they mean.
///
/// | code | player |
/// |---|---|
/// | `r` | uniformly random legal action |
/// | `w` | weighted random |
/// | `aa` | attach energy, then attack |
/// | `et` | end the turn immediately |
/// | `er` | evolution rusher |
/// | `v` | one-ply greedy on a small hand-set value function |
/// | `m` | MCTS, 100 iterations |
/// | `h` | human (TUI only) |
/// | `e`, `e<depth>` | expectiminimax over its own turn, default depth 3 |
/// | `l`, `l<depth>` | the same search scored by the learned value function |
/// | `o`, `o<depth>`, `ok<K>`, `o<depth>k<K>` | opponent-aware search, default depth 3 and K 3 |
///
/// The `o` family is the only one that looks past the end of its own turn. `o` is `o3k3`; `o2` is
/// the cheap depth, `ok5` raises the number of sampled opponent hands, and `o2k5` sets both.
/// Custom parser function enforcing case-insensitivity
pub fn parse_player_code(s: &str) -> Result<PlayerCode, String> {
    let lower = s.to_ascii_lowercase();

    // The opponent-aware family: "o", "o<depth>", "ok<K>", "o<depth>k<K>".
    if let Some(rest) = lower.strip_prefix('o') {
        let (depth_part, samples_part) = match rest.split_once('k') {
            Some((depth, samples)) => (depth, Some(samples)),
            None => (rest, None),
        };
        let max_depth = parse_positive(depth_part, opponent_aware_player::DEFAULT_DEPTH)
            .ok_or_else(|| invalid_o_code(s))?;
        let samples = match samples_part {
            Some(samples) => parse_positive(samples, 0).filter(|k| *k > 0),
            None => Some(opponent_aware_player::DEFAULT_SAMPLES),
        }
        .ok_or_else(|| invalid_o_code(s))?;
        return Ok(PlayerCode::O { max_depth, samples });
    }

    // Check if it starts with 'e' followed by digits (e.g., e2, e4)
    if lower.starts_with('e') && lower.len() > 1 {
        let rest = &lower[1..];
        if let Ok(max_depth) = rest.parse::<usize>() {
            return Ok(PlayerCode::E { max_depth });
        }
        // If it starts with 'e' but not followed by valid number, check if it's 'er'
        if lower == "er" {
            return Ok(PlayerCode::ER);
        }
        return Err(format!("Invalid player code: {s}. Use 'e<number>' for ExpectiMiniMax with depth, e.g., 'e2', 'e5'"));
    }

    // Same shape for the learned value function: 'l' at depth 3, 'l<number>' for another depth.
    if lower.starts_with('l') && lower.len() > 1 {
        let rest = &lower[1..];
        if let Ok(max_depth) = rest.parse::<usize>() {
            return Ok(PlayerCode::L { max_depth });
        }
        return Err(format!("Invalid player code: {s}. Use 'l<number>' for ExpectiMiniMax with the learned value function and a depth, e.g., 'l2', 'l5'"));
    }

    match lower.as_str() {
        "aa" => Ok(PlayerCode::AA),
        "et" => Ok(PlayerCode::ET),
        "r" => Ok(PlayerCode::R),
        "h" => Ok(PlayerCode::H),
        "w" => Ok(PlayerCode::W),
        "m" => Ok(PlayerCode::M),
        "v" => Ok(PlayerCode::V),
        "e" => Ok(PlayerCode::E { max_depth: 3 }), // Default depth
        "er" => Ok(PlayerCode::ER),
        "l" => Ok(PlayerCode::L { max_depth: 3 }), // Default depth
        _ => Err(format!("Invalid player code: {s}")),
    }
}

/// Parses `text` as a positive integer, or returns `default` when it is empty. `None` means the
/// text was present but not a usable count.
fn parse_positive(text: &str, default: usize) -> Option<usize> {
    if text.is_empty() {
        return Some(default);
    }
    text.parse::<usize>().ok().filter(|value| *value > 0)
}

fn invalid_o_code(s: &str) -> String {
    format!(
        "Invalid player code: {s}. Use 'o' for the opponent-aware search at depth {} with K={}, \
         or 'o<depth>', 'ok<K>', 'o<depth>k<K>', e.g. 'o2', 'ok5', 'o2k5'",
        opponent_aware_player::DEFAULT_DEPTH,
        opponent_aware_player::DEFAULT_SAMPLES
    )
}

pub fn parse_player_code_generic(s: String) -> Result<PlayerCode, String> {
    parse_player_code(s.as_ref())
}

pub fn fill_code_array(maybe_players: Option<Vec<PlayerCode>>) -> Vec<PlayerCode> {
    match maybe_players {
        Some(mut player_codes) => {
            if player_codes.is_empty() || player_codes.len() > 2 {
                panic!("Invalid number of players");
            } else if player_codes.len() == 1 {
                player_codes.push(PlayerCode::R);
            }
            player_codes
        }
        None => vec![PlayerCode::R, PlayerCode::R],
    }
}

pub fn create_players(
    deck_a: Deck,
    deck_b: Deck,
    players: Vec<PlayerCode>,
) -> Vec<Box<dyn Player>> {
    let player_a: Box<dyn Player> = get_player(deck_a.clone(), &players[0]);
    let player_b: Box<dyn Player> = get_player(deck_b.clone(), &players[1]);
    vec![player_a, player_b]
}

fn get_player(deck: Deck, player: &PlayerCode) -> Box<dyn Player> {
    match player {
        PlayerCode::AA => Box::new(AttachAttackPlayer { deck }),
        PlayerCode::ET => Box::new(EndTurnPlayer { deck }),
        PlayerCode::R => Box::new(RandomPlayer { deck }),
        PlayerCode::H => Box::new(HumanPlayer { deck }),
        PlayerCode::W => Box::new(WeightedRandomPlayer { deck }),
        PlayerCode::M => Box::new(MctsPlayer::new(deck, 100)),
        PlayerCode::V => Box::new(ValueFunctionPlayer { deck }),
        PlayerCode::E { max_depth } => Box::new(ExpectiMiniMaxPlayer {
            deck,
            max_depth: *max_depth,
            write_debug_trees: false,
            value_function: value_functions::build_value_function(
                value_functions::ValueFunctionKind::Baseline,
            ),
        }),
        PlayerCode::ER => Box::new(EvolutionRusherPlayer { deck }),
        PlayerCode::L { max_depth } => Box::new(ExpectiMiniMaxPlayer {
            deck,
            max_depth: *max_depth,
            write_debug_trees: false,
            value_function: value_functions::build_value_function(
                value_functions::ValueFunctionKind::Learned,
            ),
        }),
        PlayerCode::O { max_depth, samples } => {
            Box::new(OpponentAwarePlayer::new(deck, *max_depth, *samples))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opponent_aware_player::{DEFAULT_DEPTH, DEFAULT_SAMPLES};

    #[test]
    fn test_parse_opponent_aware_codes() {
        let defaults = PlayerCode::O {
            max_depth: DEFAULT_DEPTH,
            samples: DEFAULT_SAMPLES,
        };
        assert_eq!(parse_player_code("o"), Ok(defaults.clone()));
        assert_eq!(parse_player_code("O"), Ok(defaults));
        assert_eq!(
            parse_player_code("o2"),
            Ok(PlayerCode::O {
                max_depth: 2,
                samples: DEFAULT_SAMPLES
            })
        );
        assert_eq!(
            parse_player_code("ok5"),
            Ok(PlayerCode::O {
                max_depth: DEFAULT_DEPTH,
                samples: 5
            })
        );
        assert_eq!(
            parse_player_code("o2k5"),
            Ok(PlayerCode::O {
                max_depth: 2,
                samples: 5
            })
        );
    }

    #[test]
    fn test_parse_rejects_malformed_opponent_aware_codes() {
        for code in ["o0", "ok0", "o2k0", "ox", "o2k", "okx", "o-1"] {
            assert!(
                parse_player_code(code).is_err(),
                "{code} should not parse as a player code"
            );
        }
    }

    #[test]
    fn test_parse_leaves_the_other_codes_alone() {
        assert_eq!(parse_player_code("e"), Ok(PlayerCode::E { max_depth: 3 }));
        assert_eq!(parse_player_code("e2"), Ok(PlayerCode::E { max_depth: 2 }));
        assert_eq!(parse_player_code("er"), Ok(PlayerCode::ER));
        assert_eq!(parse_player_code("v"), Ok(PlayerCode::V));
        assert_eq!(parse_player_code("m"), Ok(PlayerCode::M));
        assert_eq!(parse_player_code("r"), Ok(PlayerCode::R));
        assert_eq!(parse_player_code("l"), Ok(PlayerCode::L { max_depth: 3 }));
        assert_eq!(parse_player_code("l2"), Ok(PlayerCode::L { max_depth: 2 }));
    }
}

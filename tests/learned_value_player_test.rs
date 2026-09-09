//! The `L` player code: the expectiminimax search scored by the learned value function.
//!
//! The feature vector itself is unit-tested in `src/players/value_functions.rs`, and its
//! agreement with the Python extractor is checked by `sim/learn/features_crosscheck.py` in the
//! Pocket Lab repo. What is left to check here is that `L` is wired up: that the code parses,
//! that it builds a player, and that a real game played with it finishes.

use deckgym::players::{
    parse_player_code, LearnedValueFunction, PlayerCode, LEARNED_FEATURE_COUNT,
};
use deckgym::simulate::Simulation;
use deckgym::test_support::load_test_decks;
use deckgym::State;

#[test]
fn player_code_l_parses_at_the_default_depth_and_at_an_explicit_one() {
    assert_eq!(
        parse_player_code("l").unwrap(),
        PlayerCode::L { max_depth: 3 }
    );
    assert_eq!(
        parse_player_code("L").unwrap(),
        PlayerCode::L { max_depth: 3 }
    );
    assert_eq!(
        parse_player_code("l2").unwrap(),
        PlayerCode::L { max_depth: 2 }
    );
    assert_eq!(
        parse_player_code("L5").unwrap(),
        PlayerCode::L { max_depth: 5 }
    );
    // Still distinct from the baseline search, and still rejecting nonsense.
    assert_eq!(
        parse_player_code("e").unwrap(),
        PlayerCode::E { max_depth: 3 }
    );
    assert!(parse_player_code("lx").is_err());
}

#[test]
fn the_learned_function_scores_a_fresh_state() {
    let features = LearnedValueFunction::new().features(&State::default(), 0);
    assert_eq!(features.len(), LEARNED_FEATURE_COUNT);
    assert!(LearnedValueFunction::new()
        .score(&State::default(), 0)
        .is_finite());
}

#[test]
fn a_short_simulation_with_l_finishes() {
    let (deck_a, deck_b) = load_test_decks();
    // Four games at depth 3 is a smoke test, not a measurement: the ladder is the measure.
    let mut simulation = Simulation::new_with_decks(
        deck_a,
        deck_b,
        vec![
            PlayerCode::L { max_depth: 3 },
            PlayerCode::E { max_depth: 2 },
        ],
        4,
        Some(20_260_909),
        false,
        None,
    )
    .expect("decks load");

    let outcomes = simulation.run();
    assert_eq!(outcomes.len(), 4);
}

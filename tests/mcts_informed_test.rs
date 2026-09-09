//! Tests for the value-guided information-set MCTS player (`m`).
//!
//! Card facts used to build the positions below come from `database.json`:
//! Bulbasaur (A1 001) 70 HP, Vine Whip [G][C] for 40. Venusaur (A1 003) 160 HP, Mega Drain
//! [G][G][C][C] for 80. Koffing (A1 176) 70 HP, Suffocating Gas [D] for 20. Rocky Helmet (A2 148),
//! Tool: "If the Pokemon this card is attached to is in the Active Spot and is damaged by an attack
//! from your opponent's Pokemon, do 20 damage to the Attacking Pokemon."

use std::time::{Duration, Instant};

use deckgym::{
    actions::{Action, SimpleAction},
    card_ids::CardId,
    database::get_card_by_enum,
    models::{EnergyType, PlayedCard},
    players::{ExpectiMiniMaxPlayer, MctsInformedPlayer, Player, RandomPlayer},
    state::GameOutcome,
    test_support::{get_initialized_game, load_test_decks},
    State,
};
use rand::{rngs::StdRng, SeedableRng};
use rayon::prelude::*;

/// Iterations used by the 100-game ladder test. Deliberately far below the `m` default of 200:
/// the point is that the search beats a random player decisively, not that it is at full strength,
/// and the suite has to stay cheap on a shared machine (100 games at 6 iterations is about 45 s
/// of an unoptimised suite run, spread over LADDER_THREADS threads).
const LADDER_ITERATIONS: u64 = 6;

/// A decision at the `m` default of 200 iterations must land inside this budget.
const DECISION_BUDGET: Duration = Duration::from_secs(2);

/// Threads the 100-game test is allowed to use. Capped rather than left to rayon's default of one
/// per core so that it does not starve the timing test running beside it, and so the suite stays
/// polite on a shared machine.
const LADDER_THREADS: usize = 4;

/// Hardware yardstick. An ExpectiMiniMax depth-3 decision on the same position is a known quantity
/// (the published matrix is run with it), so if that alone takes longer than this the machine is
/// slower than the one the budget was measured on, or this is an unoptimised build, and the timing
/// assertion is skipped rather than reported as a failure.
/// Warmed up, this machine (M4 Max) runs the yardstick in about 0.4 ms. A budget seven times that
/// means the assertion is skipped once the machine is loaded enough (by other agents sharing the
/// CPU, say) to push a decision that would otherwise take half a second past the two-second budget.
const YARDSTICK_BUDGET: Duration = Duration::from_millis(3);

/// The position the puzzle tests use.
///
/// Player 0 to move, mid-game. Their Active Bulbasaur is down to 20 HP but is fully powered, and
/// the opponent's Active Venusaur is untouched, has no Energy at all (so it cannot answer next
/// turn: Mega Drain costs four) and wears a Rocky Helmet. Vine Whip would deal 40 to a 160 HP
/// Pokemon and take 20 back, which knocks Bulbasaur out and hands the opponent a point for nothing.
/// Every other action keeps the board.
fn rocky_helmet_position() -> (State, Vec<Action>) {
    let mut game = get_initialized_game(4);
    game.play_until_stable();

    let mut state = game.get_state_clone();
    state.set_board(
        vec![
            PlayedCard::from_id(CardId::A1001Bulbasaur)
                .with_energy(vec![EnergyType::Grass, EnergyType::Grass])
                .with_remaining_hp(20),
            PlayedCard::from_id(CardId::A1003Venusaur)
                .with_energy(vec![EnergyType::Grass, EnergyType::Grass]),
        ],
        vec![
            PlayedCard::from_id(CardId::A1003Venusaur)
                .with_tool(get_card_by_enum(CardId::A2148RockyHelmet)),
            PlayedCard::from_id(CardId::A1176Koffing),
        ],
    );
    state.current_player = 0;
    state.turn_count = 9;
    // An empty hand keeps the decision about the attack rather than about card play.
    state.hands[0] = vec![];

    let (actor, actions) = state.generate_possible_actions();
    assert_eq!(actor, 0, "player 0 should be the one to move");
    (state, actions)
}

fn is_attack(action: &Action) -> bool {
    matches!(action.action, SimpleAction::Attack(_))
}

#[test]
fn test_informed_mcts_beats_random_decisively() {
    let games = 100;
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(LADDER_THREADS)
        .build()
        .expect("rayon pool");
    let wins: usize = pool.install(|| {
        (0..games)
            .into_par_iter()
            .map(|game_index| {
                let (deck_a, deck_b) = load_test_decks();
                // Alternate seats so the coin-toss advantage of going first cancels out.
                let mcts_seat = game_index % 2;
                let players: Vec<Box<dyn Player>> = if mcts_seat == 0 {
                    vec![
                        Box::new(MctsInformedPlayer::new(deck_a, LADDER_ITERATIONS)),
                        Box::new(RandomPlayer { deck: deck_b }),
                    ]
                } else {
                    vec![
                        Box::new(RandomPlayer { deck: deck_a }),
                        Box::new(MctsInformedPlayer::new(deck_b, LADDER_ITERATIONS)),
                    ]
                };
                let mut game = deckgym::Game::new(players, 1_000 + game_index as u64);
                match game.play() {
                    Some(GameOutcome::Win(winner)) if winner == mcts_seat => 1,
                    _ => 0,
                }
            })
            .sum()
    });

    eprintln!("informed MCTS won {wins}/{games} games against a random player");

    // A coin flip over 100 games sits inside 40 to 60 wins with very high probability, so 75 is a
    // decisive margin and still leaves room for the variance of a low-iteration search.
    assert!(
        wins >= 75,
        "informed MCTS won only {wins}/100 games against a random player"
    );
}

#[test]
fn test_declines_suicidal_attack_into_rocky_helmet() {
    let (state, actions) = rocky_helmet_position();
    assert!(
        actions.iter().any(is_attack),
        "the position should offer the attack that the player has to decline"
    );

    let (deck_a, _) = load_test_decks();
    let mut player = MctsInformedPlayer::new(deck_a, 200);
    let mut rng = StdRng::seed_from_u64(11);
    let chosen = player.decision_fn(&mut rng, &state, &actions);

    assert!(
        !is_attack(&chosen),
        "attacking into Rocky Helmet knocks our own 20 HP Active out for a point; chose {:?}",
        chosen.action
    );
}

#[test]
fn test_decision_is_deterministic_for_a_given_seed() {
    let (state, actions) = rocky_helmet_position();
    let (deck_a, _) = load_test_decks();

    let mut first = MctsInformedPlayer::new(deck_a.clone(), 60);
    let mut rng = StdRng::seed_from_u64(2024);
    let action_a = first.decision_fn(&mut rng, &state, &actions);

    // A fresh player and a fresh game RNG on the same seed must reproduce the decision exactly:
    // the search RNG is seeded from one draw off the game RNG and nothing is carried between
    // decisions.
    let mut second = MctsInformedPlayer::new(deck_a.clone(), 60);
    let mut rng = StdRng::seed_from_u64(2024);
    let action_b = second.decision_fn(&mut rng, &state, &actions);
    assert_eq!(action_a, action_b);

    // And the same player asked again off an identically seeded RNG.
    let mut rng = StdRng::seed_from_u64(2024);
    let action_c = second.decision_fn(&mut rng, &state, &actions);
    assert_eq!(action_a, action_c);
}

#[test]
fn test_single_action_decisions_do_not_search() {
    let (state, actions) = rocky_helmet_position();
    let (deck_a, _) = load_test_decks();
    let only = vec![actions[0].clone()];

    let mut player = MctsInformedPlayer::new(deck_a, 100_000);
    let mut rng = StdRng::seed_from_u64(5);
    let started = Instant::now();
    let chosen = player.decision_fn(&mut rng, &state, &only);

    assert_eq!(chosen, only[0]);
    assert!(
        started.elapsed() < Duration::from_millis(50),
        "a forced decision should return without searching"
    );
}

#[test]
fn test_decision_at_default_iterations_fits_the_budget() {
    if cfg!(debug_assertions) {
        eprintln!(
            "skipping the MCTS timing assertion: this is an unoptimised build, and the 2 s budget \
             is a claim about the release binary. Run `cargo test --release --features \
             \"tui test-utils\" --test mcts_informed_test` to check it."
        );
        return;
    }

    let (state, actions) = rocky_helmet_position();
    let (deck_a, _) = load_test_decks();

    // One discarded run first: the very first decision in a process pays for the card database
    // and for the CPU still ramping up, which is not what the yardstick is trying to measure.
    time_yardstick(&state, &actions, &deck_a);
    let before = time_yardstick(&state, &actions, &deck_a);
    if before > YARDSTICK_BUDGET {
        skip(before);
        return;
    }

    let mut player = MctsInformedPlayer::new(deck_a.clone(), 200);
    let mut rng = StdRng::seed_from_u64(3);
    let started = Instant::now();
    player.decision_fn(&mut rng, &state, &actions);
    let elapsed = started.elapsed();

    // The yardstick is measured on both sides of the decision, because the thing most likely to
    // blow the budget on this machine is another agent's build landing halfway through.
    let after = time_yardstick(&state, &actions, &deck_a);
    if after > YARDSTICK_BUDGET {
        skip(after);
        return;
    }

    eprintln!("informed MCTS decision at 200 iterations took {elapsed:?}");
    assert!(
        elapsed < DECISION_BUDGET,
        "a 200-iteration decision took {elapsed:?}, over the {DECISION_BUDGET:?} budget"
    );
}

fn time_yardstick(state: &State, actions: &[Action], deck: &deckgym::Deck) -> Duration {
    let mut yardstick = ExpectiMiniMaxPlayer {
        deck: deck.clone(),
        max_depth: 3,
        write_debug_trees: false,
        value_function: Box::new(deckgym::players::baseline_value_function),
    };
    let mut rng = StdRng::seed_from_u64(3);
    let started = Instant::now();
    yardstick.decision_fn(&mut rng, state, actions);
    let elapsed = started.elapsed();
    eprintln!("ExpectiMiniMax depth-3 yardstick took {elapsed:?}");
    elapsed
}

fn skip(yardstick_elapsed: Duration) {
    eprintln!(
        "skipping the MCTS timing assertion: the ExpectiMiniMax depth-3 yardstick took \
         {yardstick_elapsed:?} (budget {YARDSTICK_BUDGET:?}), so this is either an unoptimised \
         build, slower hardware than the budget was measured on, or a machine busy with other work"
    );
}

#[test]
fn test_player_codes_parse() {
    use deckgym::players::{parse_player_code, PlayerCode, ValueFunctionKind};

    // The value function is part of the code since the bot branches were merged: a bare `m` keeps
    // the hand-tuned baseline and `mL` asks for the learned one.
    assert_eq!(
        parse_player_code("m"),
        Ok(PlayerCode::M {
            iterations: 200,
            value_function: ValueFunctionKind::Baseline
        })
    );
    assert_eq!(
        parse_player_code("m500"),
        Ok(PlayerCode::M {
            iterations: 500,
            value_function: ValueFunctionKind::Baseline
        })
    );
    assert_eq!(
        parse_player_code("M500"),
        Ok(PlayerCode::M {
            iterations: 500,
            value_function: ValueFunctionKind::Baseline
        })
    );
    assert_eq!(
        parse_player_code("mL"),
        Ok(PlayerCode::M {
            iterations: 200,
            value_function: ValueFunctionKind::Learned
        })
    );
    // The old random-rollout MCTS keeps its previous default of 100 iterations under `mr`.
    assert_eq!(
        parse_player_code("mr"),
        Ok(PlayerCode::MR { iterations: 100 })
    );
    assert_eq!(
        parse_player_code("mr50"),
        Ok(PlayerCode::MR { iterations: 50 })
    );
    assert!(parse_player_code("mx").is_err());
    assert!(parse_player_code("mr7x").is_err());
    // Unrelated codes still parse the way they did.
    assert_eq!(parse_player_code("e"), Ok(PlayerCode::E { max_depth: 3 }));
    assert_eq!(parse_player_code("r"), Ok(PlayerCode::R));
}

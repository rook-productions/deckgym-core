//! Puzzles for the opponent-aware player (`o`). Each position is built from card text in
//! `database.json`, quoted in the test that uses it, and has one move a decent player would make.

use std::sync::{Mutex, Once, OnceLock};

use deckgym::actions::{Action, SimpleAction};
use deckgym::card_ids::CardId;
use deckgym::database::get_card_by_enum;
use deckgym::models::{Card, EnergyType, PlayedCard};
use deckgym::players::{ExpectiMiniMaxPlayer, OpponentAwarePlayer, Player, ReplyPolicy};
use deckgym::state::State;
use deckgym::test_support::{get_initialized_game, DECK_A};
use deckgym::Game;
use log::{LevelFilter, Log, Metadata, Record};
use rand::rngs::StdRng;
use rand::SeedableRng;

/// Builds a decision point: a real initialised game, its board replaced, and player 0 to move.
/// The opponent's Trainer cards are removed from their deck so that a sampled reply turn is about
/// the board and not about which Supporter the determinisation happened to deal them.
fn position(player_0: Vec<PlayedCard>, player_1: Vec<PlayedCard>) -> State {
    let mut game: Game<'static> = get_initialized_game(0);
    game.play_until_stable();
    let mut state = game.get_state_clone();
    state.set_board(player_0, player_1);
    state.current_player = 0;
    state.turn_count = 7;
    state.hands[0].clear();
    state.hands[1].clear();
    state.decks[1]
        .cards
        .retain(|c| matches!(c, Card::Pokemon(_)));
    state.energy_zone[0].current = Some(EnergyType::Grass);
    state
}

fn opponent_aware(depth: usize, samples: usize) -> OpponentAwarePlayer {
    OpponentAwarePlayer::new(DECK_A.clone(), depth, samples)
}

fn expectiminimax(max_depth: usize) -> ExpectiMiniMaxPlayer {
    ExpectiMiniMaxPlayer {
        deck: DECK_A.clone(),
        max_depth,
        write_debug_trees: false,
        value_function: Box::new(deckgym::players::baseline_value_function),
    }
}

fn attack_named(actions: &[Action], title: &str) -> Action {
    actions
        .iter()
        .find(|action| match &action.action {
            SimpleAction::Attack(attack) => attack.title == title,
            _ => false,
        })
        .unwrap_or_else(|| panic!("{title} should be a legal attack in this position"))
        .clone()
}

/// Rocky Helmet (A2 148): "If the Pokémon this card is attached to is in the Active Spot and is
/// damaged by an attack from your opponent's Pokémon, do 20 damage to the Attacking Pokémon."
///
/// Blacephalon ex (B2 023) has 20 HP left, so Pop-Punk (140 damage) leaves Venusaur ex (190 HP)
/// alive, takes the 20 back and hands the opponent two points. Singe deals no damage, so the
/// helmet never fires. The bot must take the safe attack.
#[test]
fn test_o_avoids_attacking_into_a_lethal_rocky_helmet_counter() {
    let state = position(
        vec![
            PlayedCard::from_id(CardId::B2023BlacephalonEx)
                .with_energy(vec![EnergyType::Fire, EnergyType::Fire, EnergyType::Fire])
                .with_remaining_hp(20),
            PlayedCard::from_id(CardId::A1021Exeggcute),
        ],
        vec![
            PlayedCard::from_id(CardId::A1004VenusaurEx)
                .with_tool(get_card_by_enum(CardId::A2148RockyHelmet)),
            PlayedCard::from_id(CardId::A1001Bulbasaur),
        ],
    );

    let (actor, all_actions) = state.generate_possible_actions();
    assert_eq!(actor, 0);
    let safe = attack_named(&all_actions, "Singe");
    let lethal = attack_named(&all_actions, "Pop-Punk");

    let mut rng = StdRng::seed_from_u64(11);
    let chosen =
        opponent_aware(3, 3).decision_fn(&mut rng, &state, &[lethal.clone(), safe.clone()]);

    assert_eq!(
        chosen, safe,
        "o attacked into the helmet counter that knocks its own ex out"
    );
}

/// Arbok (A1 165) has Darkness and Colorless attached, so Corner (60 damage) is online. Bulbasaur
/// (A1 001) is down to 20 HP and its one Fire energy pays the single Colorless retreat cost
/// without paying for anything else, since Vine Whip needs Grass. Ending the turn hands the
/// opponent a point; retreating to the untouched Venusaur ex (190 HP) does not.
///
/// This is the case plain `e` cannot see: nothing inside player 0's own turn distinguishes the
/// two lines.
#[test]
fn test_o_retreats_a_one_shottable_active_to_a_healthy_bench() {
    let state = position(
        vec![
            PlayedCard::from_id(CardId::A1001Bulbasaur)
                .with_energy(vec![EnergyType::Fire])
                .with_remaining_hp(20),
            PlayedCard::from_id(CardId::A1004VenusaurEx),
        ],
        vec![PlayedCard::from_id(CardId::A1165Arbok)
            .with_energy(vec![EnergyType::Darkness, EnergyType::Colorless])],
    );

    let (actor, all_actions) = state.generate_possible_actions();
    assert_eq!(actor, 0);
    let candidates: Vec<Action> = all_actions
        .iter()
        .filter(|action| {
            matches!(
                action.action,
                SimpleAction::Retreat(_) | SimpleAction::EndTurn
            )
        })
        .cloned()
        .collect();
    assert!(
        candidates
            .iter()
            .any(|a| matches!(a.action, SimpleAction::Retreat(1))),
        "retreating to the benched Venusaur ex should be legal"
    );

    let mut rng = StdRng::seed_from_u64(23);
    let chosen = opponent_aware(3, 3).decision_fn(&mut rng, &state, &candidates);

    assert!(
        matches!(chosen.action, SimpleAction::Retreat(1)),
        "o left a Pokemon the opponent's visible attack knocks out in the Active Spot: {:?}",
        chosen.action
    );
}

/// The one puzzle in this file that `e` gets wrong, and the reason this player exists.
///
/// Masquerain (A3 004) is Grass with a Fire Weakness, 90 HP and no retreat cost; Drifblim
/// (A2b 034) is Psychic with a Darkness Weakness, also 90 HP and no retreat cost. Both are down
/// to 60 HP, so the two boards are worth the same to the baseline value function: identical HP,
/// identical retreat cost, identical safety, and its `turns_until_opponent_wins` term reads
/// Magmar's Magma Punch (A1 044, "Magma Punch", 50 damage) as the threat and takes two turns to
/// knock either of them out.
///
/// Weakness is what the value function leaves out. Against Masquerain the same attack does
/// 50 + 20 = 70 and takes the knockout now; against Drifblim it does 50 and leaves 10 HP. So `e`
/// stays and attacks with Bug Buzz, and `o`, which plays the reply out, retreats for free instead.
#[test]
fn test_o_retreats_from_a_weakness_knockout_that_e_cannot_see() {
    let state = position(
        vec![
            PlayedCard::from_id(CardId::A3004Masquerain)
                .with_energy(vec![EnergyType::Grass])
                .with_remaining_hp(60),
            PlayedCard::from_id(CardId::A2b034Drifblim).with_remaining_hp(60),
        ],
        vec![PlayedCard::from_id(CardId::A1044Magmar)
            .with_energy(vec![EnergyType::Fire, EnergyType::Fire])],
    );

    let (actor, actions) = state.generate_possible_actions();
    assert_eq!(actor, 0);
    assert!(
        actions
            .iter()
            .any(|a| matches!(a.action, SimpleAction::Retreat(1))),
        "the free retreat to Drifblim should be legal"
    );

    let mut rng_o = StdRng::seed_from_u64(23);
    let chosen_o = opponent_aware(3, 3).decision_fn(&mut rng_o, &state, &actions);
    assert!(
        matches!(chosen_o.action, SimpleAction::Retreat(1)),
        "o left an Active the opponent's Weakness knockout takes next turn: {:?}",
        chosen_o.action
    );

    // The control: this is the discriminating case, so `e` is expected to get it wrong. If this
    // ever starts passing, the value function learned about Weakness and the puzzle is stale.
    let mut rng_e = StdRng::seed_from_u64(23);
    let chosen_e = expectiminimax(3).decision_fn(&mut rng_e, &state, &actions);
    assert!(
        !matches!(chosen_e.action, SimpleAction::Retreat(1)),
        "e now solves this puzzle too, so it no longer shows what o adds"
    );
}

/// `ReplyPolicy::Greedy` is the cheap reply: one ply instead of the `e2` search. It is a public
/// knob on the player, so it gets exercised here rather than left to rot.
#[test]
fn test_o_solves_the_weakness_puzzle_with_the_greedy_reply_policy() {
    let state = position(
        vec![
            PlayedCard::from_id(CardId::A3004Masquerain)
                .with_energy(vec![EnergyType::Grass])
                .with_remaining_hp(60),
            PlayedCard::from_id(CardId::A2b034Drifblim).with_remaining_hp(60),
        ],
        vec![PlayedCard::from_id(CardId::A1044Magmar)
            .with_energy(vec![EnergyType::Fire, EnergyType::Fire])],
    );
    let (_, actions) = state.generate_possible_actions();

    let mut bot = opponent_aware(3, 3);
    bot.reply_policy = ReplyPolicy::Greedy;
    let mut rng = StdRng::seed_from_u64(23);
    let chosen = bot.decision_fn(&mut rng, &state, &actions);

    assert!(
        matches!(chosen.action, SimpleAction::Retreat(1)),
        "the greedy reply policy should still see the Weakness knockout: {:?}",
        chosen.action
    );
    assert_eq!(bot.fallback_count(), 0);
}

/// With two points already banked and Exeggcute's Seed Bomb (20 damage) exactly lethal on a
/// Bulbasaur down to 20 HP, the knockout wins the game outright, so no reply exists to search and
/// `o` must agree with `e`. The Energy Zone is emptied so that attaching is not also a legal
/// action: an attach followed by the same lethal attack wins just as well, and both bots would
/// then be picking between two lines that score identically.
#[test]
fn test_o_matches_e_when_the_opponent_never_gets_to_reply() {
    let mut state = position(
        vec![
            PlayedCard::from_id(CardId::A1021Exeggcute).with_energy(vec![EnergyType::Grass]),
            PlayedCard::from_id(CardId::A1001Bulbasaur),
        ],
        vec![
            PlayedCard::from_id(CardId::A1001Bulbasaur).with_remaining_hp(20),
            PlayedCard::from_id(CardId::A1164Ekans),
        ],
    );
    state.points[0] = 2;
    state.energy_zone[0].current = None;

    let (actor, actions) = state.generate_possible_actions();
    assert_eq!(actor, 0);

    let mut rng_o = StdRng::seed_from_u64(5);
    let chosen_o = opponent_aware(3, 3).decision_fn(&mut rng_o, &state, &actions);
    let mut rng_e = StdRng::seed_from_u64(5);
    let chosen_e = expectiminimax(3).decision_fn(&mut rng_e, &state, &actions);

    assert_eq!(
        chosen_o, chosen_e,
        "o and e disagreed on a position with no opponent reply to search"
    );
    assert_eq!(chosen_o, attack_named(&actions, "Seed Bomb"));
}

/// The node budget is the last cost control: once reply rollouts have spent it, the remaining
/// leaves are scored with the plain value function, which is exactly `e`. Setting the budget to
/// zero forces that on the first leaf, and the player both counts the decision and logs it.
#[test]
fn test_o_node_budget_exhaustion_falls_back_to_expectiminimax_and_logs() {
    install_capturing_logger();
    clear_captured_logs();

    let state = position(
        vec![
            PlayedCard::from_id(CardId::A1021Exeggcute).with_energy(vec![EnergyType::Grass]),
            PlayedCard::from_id(CardId::A1001Bulbasaur),
        ],
        vec![PlayedCard::from_id(CardId::A1165Arbok)
            .with_energy(vec![EnergyType::Darkness, EnergyType::Colorless])],
    );
    let (actor, actions) = state.generate_possible_actions();
    assert_eq!(actor, 0);

    let mut bot = opponent_aware(3, 3);
    bot.node_budget = 0;
    let mut rng = StdRng::seed_from_u64(31);
    let chosen = bot.decision_fn(&mut rng, &state, &actions);

    assert!(actions.contains(&chosen));
    assert_eq!(bot.decision_count(), 1);
    assert_eq!(
        bot.fallback_count(),
        1,
        "a zero node budget should force the fallback on the first end-of-turn leaf"
    );
    let logs = captured_logs();
    assert!(
        logs.iter().any(|line| line.contains("node budget")),
        "the fallback should be logged, got: {logs:?}"
    );
}

/// A budget large enough for the position never fires the fallback.
#[test]
fn test_o_does_not_fall_back_when_the_budget_is_ample() {
    let state = position(
        vec![
            PlayedCard::from_id(CardId::A1021Exeggcute).with_energy(vec![EnergyType::Grass]),
            PlayedCard::from_id(CardId::A1001Bulbasaur),
        ],
        vec![PlayedCard::from_id(CardId::A1165Arbok)
            .with_energy(vec![EnergyType::Darkness, EnergyType::Colorless])],
    );
    let (_, actions) = state.generate_possible_actions();

    let mut bot = opponent_aware(3, 3);
    let mut rng = StdRng::seed_from_u64(31);
    bot.decision_fn(&mut rng, &state, &actions);

    assert_eq!(bot.fallback_count(), 0);
}

/// The determinisation may not read the opponent's actual hand: it must reshuffle everything they
/// have not revealed. Sampling the same position repeatedly should therefore deal different hands,
/// while the unseen multiset (hand plus remaining deck), the split sizes and everything public
/// (board, discard, points) stay exactly as they were.
#[test]
fn test_o_resamples_the_opponent_hand_rather_than_reading_it() {
    let mut state = position(
        vec![
            PlayedCard::from_id(CardId::A1021Exeggcute).with_energy(vec![EnergyType::Grass]),
            PlayedCard::from_id(CardId::A1001Bulbasaur),
        ],
        vec![PlayedCard::from_id(CardId::A1165Arbok)
            .with_energy(vec![EnergyType::Darkness, EnergyType::Colorless])],
    );

    // Give the opponent a known hand and a known remaining deck, so the test is about the
    // resampling and not about whatever the initial deal happened to leave behind.
    let hand_ids = [
        CardId::A1164Ekans,
        CardId::A1176Koffing,
        CardId::PA001Potion,
    ];
    let deck_ids = [
        CardId::A1177Weezing,
        CardId::A1225Sabrina,
        CardId::A1223Giovanni,
        CardId::PA002XSpeed,
        CardId::A1165Arbok,
    ];
    state.hands[1] = hand_ids.iter().map(|id| get_card_by_enum(*id)).collect();
    state.decks[1].cards = deck_ids.iter().map(|id| get_card_by_enum(*id)).collect();

    let mut expected_unseen: Vec<String> = state.hands[1]
        .iter()
        .chain(state.decks[1].cards.iter())
        .map(|c| c.get_id())
        .collect();
    expected_unseen.sort();

    let mut hands_seen = std::collections::HashSet::new();
    for seed in 0..8u64 {
        let mut rng = StdRng::seed_from_u64(seed);
        let sampled = deckgym::players::determinise_opponent_hand_for_test(&state, 0, &mut rng);

        assert_eq!(sampled.hands[1].len(), state.hands[1].len());
        assert_eq!(
            sampled.decks[1].cards.len(),
            state.decks[1].cards.len(),
            "the deck/hand split must keep its sizes"
        );
        assert_eq!(sampled.in_play_pokemon, state.in_play_pokemon);
        assert_eq!(sampled.discard_piles, state.discard_piles);
        assert_eq!(sampled.points, state.points);
        assert_eq!(
            sampled.hands[0], state.hands[0],
            "our own hand is not the one being resampled"
        );

        let mut after: Vec<String> = sampled.hands[1]
            .iter()
            .chain(sampled.decks[1].cards.iter())
            .map(|c| c.get_id())
            .collect();
        after.sort();
        assert_eq!(
            after, expected_unseen,
            "the unseen multiset must be preserved"
        );

        hands_seen.insert(
            sampled.hands[1]
                .iter()
                .map(|c| c.get_id())
                .collect::<Vec<_>>()
                .join(","),
        );
    }
    assert!(
        hands_seen.len() > 1,
        "every determinisation dealt the same hand, so nothing was resampled"
    );
}

// ---------------------------------------------------------------------------------------------
// A logger that keeps what it is given, so the fallback test can assert on the warning itself
// rather than only on the counter.

static CAPTURED: OnceLock<Mutex<Vec<String>>> = OnceLock::new();
static INSTALL: Once = Once::new();

struct CapturingLogger;

impl Log for CapturingLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= log::Level::Warn
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            if let Ok(mut lines) = CAPTURED.get_or_init(Default::default).lock() {
                lines.push(record.args().to_string());
            }
        }
    }

    fn flush(&self) {}
}

fn install_capturing_logger() {
    INSTALL.call_once(|| {
        let _ = log::set_boxed_logger(Box::new(CapturingLogger));
        log::set_max_level(LevelFilter::Warn);
    });
}

fn clear_captured_logs() {
    if let Ok(mut lines) = CAPTURED.get_or_init(Default::default).lock() {
        lines.clear();
    }
}

fn captured_logs() -> Vec<String> {
    CAPTURED
        .get_or_init(Default::default)
        .lock()
        .map(|lines| lines.clone())
        .unwrap_or_default()
}

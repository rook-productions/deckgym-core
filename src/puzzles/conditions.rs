//! Puzzles about Special Conditions. An Asleep or Paralyzed Pokemon can neither attack nor
//! retreat, so those positions are about what else the turn can buy; a Confused Pokemon can still
//! retreat, and getting off the coin flip is usually the point.

use super::support::{attach_to, evolve_at, position, puzzle_deck, retreat_to, spent_zone, zone};
use super::Puzzle;
use crate::card_ids::CardId;
use crate::database::get_card_by_enum;
use crate::models::{EnergyType, PlayedCard, StatusCondition};

pub(super) fn puzzles() -> Vec<Puzzle> {
    vec![
        retreat_the_confused_active_to_the_loaded_bench(),
        asleep_active_attaches_to_the_bench(),
        evolve_out_of_paralysis_to_attack(),
    ]
}

/// Confusion only gates attacking (a coin flip), not retreating. Gnaw's 20 plus 20 for
/// Magikarp's [L] Weakness would Knock Out its 30 HP, but only on heads. Retreating costs one of
/// Pikachu's two [L] and leaves it holding one, and the Bench Hitmonchan is already paid up: its
/// Jab does 30 into the same 30 HP for the third point in this same turn, with no coin at all.
fn retreat_the_confused_active_to_the_loaded_bench() -> Puzzle {
    let mut game = position(
        puzzle_deck(
            &[CardId::A1094Pikachu, CardId::A1155Hitmonchan],
            &[EnergyType::Lightning],
        ),
        puzzle_deck(
            &[CardId::A1077Magikarp, CardId::A1059Poliwag],
            &[EnergyType::Water],
        ),
    );
    game.set_board(
        vec![
            PlayedCard::from_id(CardId::A1094Pikachu)
                .with_energy(vec![EnergyType::Lightning, EnergyType::Lightning]),
            PlayedCard::from_id(CardId::A1155Hitmonchan).with_energy(vec![EnergyType::Fighting]),
        ],
        vec![
            PlayedCard::from_id(CardId::A1077Magikarp),
            PlayedCard::from_id(CardId::A1059Poliwag),
        ],
    );
    game.apply_status_condition(0, 0, StatusCondition::Confused);
    game.points = [2, 0];
    game.energy_zone[0] = spent_zone(EnergyType::Lightning);
    Puzzle {
        name: "retreat_the_confused_active_to_the_loaded_bench",
        why: "Attacking through Confusion wins on heads only; retreating into Jab's 30 wins outright.",
        game,
        acceptable: vec![retreat_to(1)],
    }
}

/// An Asleep Pokemon cannot attack or retreat, so the only thing this turn can buy is Energy on
/// the Bench. Magikarp's Splash does 10 and it is stuck in the Active Spot; Hitmonchan's Jab
/// does 30 and covers the opponent's 30 remaining HP the moment it has one [F].
fn asleep_active_attaches_to_the_bench() -> Puzzle {
    let mut game = position(
        puzzle_deck(
            &[CardId::A1077Magikarp, CardId::A1155Hitmonchan],
            &[EnergyType::Fighting],
        ),
        puzzle_deck(
            &[CardId::A1189Rattata, CardId::A1059Poliwag],
            &[EnergyType::Water],
        ),
    );
    game.set_board(
        vec![
            PlayedCard::from_id(CardId::A1077Magikarp),
            PlayedCard::from_id(CardId::A1155Hitmonchan),
        ],
        vec![
            PlayedCard::from_id(CardId::A1189Rattata).with_remaining_hp(30),
            PlayedCard::from_id(CardId::A1059Poliwag),
        ],
    );
    game.apply_status_condition(0, 0, StatusCondition::Asleep);
    game.energy_zone[0] = zone(EnergyType::Fighting);
    Puzzle {
        name: "asleep_active_attaches_to_the_bench",
        why: "The Asleep Magikarp can do nothing with Energy; one [F] puts Jab's 30 online behind it.",
        game,
        acceptable: vec![attach_to(1)],
    }
}

/// Paralysis blocks attacking and retreating, but evolving replaces the card in play and clears
/// Special Conditions with it. Wartortle's Wave Splash costs [W][C], which the two [W] already on
/// Squirtle cover, and its 40 is exactly the Rattata's 40 HP for the third point.
fn evolve_out_of_paralysis_to_attack() -> Puzzle {
    let mut game = position(
        puzzle_deck(
            &[
                CardId::A1053Squirtle,
                CardId::A1054Wartortle,
                CardId::A1189Rattata,
            ],
            &[EnergyType::Water],
        ),
        puzzle_deck(
            &[CardId::A1189Rattata, CardId::A1059Poliwag],
            &[EnergyType::Water],
        ),
    );
    game.set_board(
        vec![
            PlayedCard::from_id(CardId::A1053Squirtle)
                .with_energy(vec![EnergyType::Water, EnergyType::Water]),
            PlayedCard::from_id(CardId::A1189Rattata),
        ],
        vec![
            PlayedCard::from_id(CardId::A1189Rattata),
            PlayedCard::from_id(CardId::A1059Poliwag),
        ],
    );
    game.apply_status_condition(0, 0, StatusCondition::Paralyzed);
    game.hands[0] = vec![get_card_by_enum(CardId::A1054Wartortle)];
    game.points = [2, 0];
    game.energy_zone[0] = spent_zone(EnergyType::Water);
    Puzzle {
        name: "evolve_out_of_paralysis_to_attack",
        why: "Evolving clears the Paralysis, and Wave Splash's 40 is the Rattata's 40 HP for the win.",
        game,
        acceptable: vec![evolve_at(0, CardId::A1054Wartortle)],
    }
}

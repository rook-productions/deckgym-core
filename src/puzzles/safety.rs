//! Puzzles about not walking the Active Pokemon into a Knock Out, and about knowing when the
//! danger does not matter.

use super::support::{attack, play, position, puzzle_deck, retreat_to, spent_zone};
use super::Puzzle;
use crate::card_ids::CardId;
use crate::database::get_card_by_enum;
use crate::models::{EnergyType, PlayedCard};

pub(super) fn puzzles() -> Vec<Puzzle> {
    vec![
        do_not_attack_into_the_lethal_rocky_helmet(),
        attack_through_the_helmet_when_it_cannot_kill(),
        potion_survives_the_visible_lethal(),
        retreat_the_ex_out_of_the_game_losing_knockout(),
    ]
}

/// Rocky Helmet does 20 to the Attacking Pokemon whenever the holder is damaged in the Active
/// Spot. Pikachu is on 20 remaining HP, so Gnaw kills Pikachu and does 20 to a 150 HP Snorlax.
/// Retreating instead brings up a full-health Chansey that is already holding its Energy.
fn do_not_attack_into_the_lethal_rocky_helmet() -> Puzzle {
    let mut game = position(
        puzzle_deck(
            &[CardId::A1094Pikachu, CardId::A1202Chansey],
            &[EnergyType::Lightning],
        ),
        puzzle_deck(
            &[CardId::A1211Snorlax, CardId::A1189Rattata],
            &[EnergyType::Fighting],
        ),
    );
    game.set_board(
        vec![
            PlayedCard::from_id(CardId::A1094Pikachu)
                .with_energy(vec![EnergyType::Lightning])
                .with_remaining_hp(20),
            PlayedCard::from_id(CardId::A1202Chansey).with_energy(vec![
                EnergyType::Lightning,
                EnergyType::Lightning,
                EnergyType::Lightning,
            ]),
        ],
        vec![
            PlayedCard::from_id(CardId::A1211Snorlax)
                .with_tool(get_card_by_enum(CardId::A2148RockyHelmet)),
            PlayedCard::from_id(CardId::A1189Rattata),
        ],
    );
    game.energy_zone[0] = spent_zone(EnergyType::Lightning);
    Puzzle {
        name: "do_not_attack_into_the_lethal_rocky_helmet",
        why: "Gnaw's 20 cannot dent 150 HP and the Helmet's 20 back Knocks Out the 20 HP Pikachu.",
        game,
        acceptable: vec![retreat_to(1)],
    }
}

/// The same Helmet, but Pikachu is at full 60 HP and the attack takes the game-winning point.
/// The 20 coming back is irrelevant.
fn attack_through_the_helmet_when_it_cannot_kill() -> Puzzle {
    let mut game = position(
        puzzle_deck(
            &[CardId::A1094Pikachu, CardId::A1202Chansey],
            &[EnergyType::Lightning],
        ),
        puzzle_deck(
            &[CardId::A1189Rattata, CardId::A1059Poliwag],
            &[EnergyType::Water],
        ),
    );
    game.set_board(
        vec![
            PlayedCard::from_id(CardId::A1094Pikachu).with_energy(vec![EnergyType::Lightning]),
            PlayedCard::from_id(CardId::A1202Chansey),
        ],
        vec![
            PlayedCard::from_id(CardId::A1189Rattata)
                .with_remaining_hp(20)
                .with_tool(get_card_by_enum(CardId::A2148RockyHelmet)),
            PlayedCard::from_id(CardId::A1059Poliwag),
        ],
    );
    game.points = [2, 0];
    game.energy_zone[0] = spent_zone(EnergyType::Lightning);
    Puzzle {
        name: "attack_through_the_helmet_when_it_cannot_kill",
        why: "Gnaw's 20 takes the third point; the Helmet's 20 back leaves Pikachu on 40 HP.",
        game,
        acceptable: vec![attack("Gnaw")],
    }
}

/// Chansey is on 20 remaining HP and the opponent's loaded Pikachu does exactly 20 with Gnaw
/// (Chansey is Weak to [F], not [L], so there is no Weakness bonus). The opponent is on 2 points,
/// so that Knock Out ends the game. Potion's 20 puts Chansey out of range.
fn potion_survives_the_visible_lethal() -> Puzzle {
    let mut game = position(
        puzzle_deck(
            &[
                CardId::A1202Chansey,
                CardId::A1189Rattata,
                CardId::PA001Potion,
            ],
            &[EnergyType::Fighting],
        ),
        puzzle_deck(
            &[CardId::A1094Pikachu, CardId::A1059Poliwag],
            &[EnergyType::Lightning],
        ),
    );
    game.set_board(
        vec![
            PlayedCard::from_id(CardId::A1202Chansey).with_remaining_hp(20),
            PlayedCard::from_id(CardId::A1189Rattata),
        ],
        vec![
            PlayedCard::from_id(CardId::A1094Pikachu).with_energy(vec![EnergyType::Lightning]),
            PlayedCard::from_id(CardId::A1059Poliwag),
        ],
    );
    game.hands[0] = vec![get_card_by_enum(CardId::PA001Potion)];
    game.points = [0, 2];
    game.energy_zone[0] = spent_zone(EnergyType::Fighting);
    Puzzle {
        name: "potion_survives_the_visible_lethal",
        why: "Gnaw does exactly Chansey's 20 remaining HP for the opponent's third point; Potion denies it.",
        game,
        acceptable: vec![play("Potion")],
    }
}

/// Starmie ex is on 20 remaining HP with the opponent on 2 points, so losing it hands over the
/// 2 points that end the game. Its Retreat Cost is zero, and the Chansey behind it survives
/// Rollout's 70 comfortably. Hydro Splash's 90 does not reach Snorlax's 150 HP, so attacking
/// only delays the loss by nothing at all.
fn retreat_the_ex_out_of_the_game_losing_knockout() -> Puzzle {
    let mut game = position(
        puzzle_deck(
            &[
                CardId::A1074Staryu,
                CardId::A1076StarmieEx,
                CardId::A1202Chansey,
            ],
            &[EnergyType::Water],
        ),
        puzzle_deck(
            &[CardId::A1211Snorlax, CardId::A1059Poliwag],
            &[EnergyType::Lightning],
        ),
    );
    game.set_board(
        vec![
            PlayedCard::from_id(CardId::A1076StarmieEx)
                .with_energy(vec![EnergyType::Water, EnergyType::Water])
                .with_remaining_hp(20),
            PlayedCard::from_id(CardId::A1202Chansey),
        ],
        vec![
            PlayedCard::from_id(CardId::A1211Snorlax).with_energy(vec![
                EnergyType::Lightning,
                EnergyType::Lightning,
                EnergyType::Lightning,
                EnergyType::Lightning,
            ]),
            PlayedCard::from_id(CardId::A1059Poliwag),
        ],
    );
    game.points = [0, 2];
    game.energy_zone[0] = spent_zone(EnergyType::Water);
    Puzzle {
        name: "retreat_the_ex_out_of_the_game_losing_knockout",
        why: "Starmie ex retreats for free; leaving it Active gives Rollout the 2 points that win the game.",
        game,
        acceptable: vec![retreat_to(1)],
    }
}

//! Puzzles that only have a right answer once you look at what the opponent does next turn:
//! where the turn's Energy survives, which of your Pokemon the opponent can reach, and which of
//! them is worth two points.

use super::support::{activate, attach_to, attack, play, position, puzzle_deck, spent_zone, zone};
use super::Puzzle;
use crate::actions::SimpleAction;
use crate::card_ids::CardId;
use crate::database::get_card_by_enum;
use crate::models::{EnergyType, PlayedCard};

pub(super) fn puzzles() -> Vec<Puzzle> {
    vec![
        attach_to_the_bench_attacker_not_the_dead_active(),
        do_not_overextend_energy_onto_the_doomed_active(),
        heal_the_bench_attacker_out_of_cyrus_range(),
        keep_the_damaged_ex_off_the_active_spot(),
        promote_the_attacker_after_the_knockout(),
    ]
}

/// Chansey is on 20 remaining HP with no Energy and needs three for Gentle Slap, so it will never
/// attack: the opponent's loaded Pikachu Knocks it Out next turn with Gnaw's 20. One [F] on the
/// Bench Hitmonchan is one [F] that is still there afterwards, and it puts Jab's 30 online.
fn attach_to_the_bench_attacker_not_the_dead_active() -> Puzzle {
    let mut game = position(
        puzzle_deck(
            &[CardId::A1202Chansey, CardId::A1155Hitmonchan],
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
            PlayedCard::from_id(CardId::A1155Hitmonchan),
        ],
        vec![
            PlayedCard::from_id(CardId::A1094Pikachu).with_energy(vec![EnergyType::Lightning]),
            PlayedCard::from_id(CardId::A1059Poliwag),
        ],
    );
    game.energy_zone[0] = zone(EnergyType::Fighting);
    Puzzle {
        name: "attach_to_the_bench_attacker_not_the_dead_active",
        why: "Chansey is one attack from dying and three Energy from attacking; the Bench keeps the Energy.",
        game,
        acceptable: vec![attach_to(1)],
    }
}

/// Machop already has the one [F] Knuckle Punch costs and has no second attack, so a second [F]
/// on it buys nothing, and Rollout's 70 clears its 20 remaining HP next turn and discards that
/// Energy with it. Rhyhorn on the Bench is one Energy short of Tackle's 60.
///
/// Machop's Retreat Cost is two and it is holding one Energy, so retreating is not on the table:
/// the position is purely about where the turn's Energy goes.
fn do_not_overextend_energy_onto_the_doomed_active() -> Puzzle {
    let mut game = position(
        puzzle_deck(
            &[CardId::A1143Machop, CardId::A1156Rhyhorn],
            &[EnergyType::Fighting],
        ),
        puzzle_deck(
            &[CardId::A1211Snorlax, CardId::A1059Poliwag],
            &[EnergyType::Lightning],
        ),
    );
    game.set_board(
        vec![
            PlayedCard::from_id(CardId::A1143Machop)
                .with_energy(vec![EnergyType::Fighting])
                .with_remaining_hp(20),
            PlayedCard::from_id(CardId::A1156Rhyhorn)
                .with_energy(vec![EnergyType::Fighting, EnergyType::Fighting]),
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
    game.energy_zone[0] = zone(EnergyType::Fighting);
    Puzzle {
        name: "do_not_overextend_energy_onto_the_doomed_active",
        why: "Knuckle Punch is already paid for and Machop dies to Rollout; the Energy belongs on Rhyhorn.",
        game,
        acceptable: vec![attach_to(1)],
    }
}

/// Cyrus switches in one of the opponent's Benched Pokemon "that has damage on it". The Bench
/// Mewtwo ex carries exactly 20 damage, and the opponent is holding Cyrus with a loaded Mewtwo ex
/// of their own: Psydrive's 150 covers its 130 remaining HP for the 2 points that end the game.
/// Potion's 20 takes the damage off entirely, which takes it off Cyrus's list.
fn heal_the_bench_attacker_out_of_cyrus_range() -> Puzzle {
    let mut game = position(
        puzzle_deck(
            &[
                CardId::A1202Chansey,
                CardId::A1129MewtwoEx,
                CardId::PA001Potion,
            ],
            &[EnergyType::Psychic],
        ),
        puzzle_deck(
            &[CardId::A1129MewtwoEx, CardId::A2150Cyrus],
            &[EnergyType::Psychic],
        ),
    );
    game.set_board(
        vec![
            PlayedCard::from_id(CardId::A1202Chansey),
            PlayedCard::from_id(CardId::A1129MewtwoEx)
                .with_energy(vec![
                    EnergyType::Psychic,
                    EnergyType::Psychic,
                    EnergyType::Psychic,
                    EnergyType::Psychic,
                ])
                .with_damage(20),
        ],
        vec![
            PlayedCard::from_id(CardId::A1129MewtwoEx).with_energy(vec![
                EnergyType::Psychic,
                EnergyType::Psychic,
                EnergyType::Psychic,
                EnergyType::Psychic,
            ]),
            PlayedCard::from_id(CardId::A1059Poliwag),
        ],
    );
    game.hands[0] = vec![get_card_by_enum(CardId::PA001Potion)];
    game.hands[1] = vec![get_card_by_enum(CardId::A2150Cyrus)];
    game.points = [0, 1];
    game.energy_zone[0] = spent_zone(EnergyType::Psychic);
    Puzzle {
        name: "heal_the_bench_attacker_out_of_cyrus_range",
        why: "Healing the last 20 damage off the Bench ex makes it an illegal Cyrus target.",
        game,
        acceptable: vec![play("Potion")],
    }
}

/// Retreating to the loaded Starmie ex would Knock Out the Electrode, but it puts a 20 HP ex in
/// the Active Spot with a loaded Pikachu on the opponent's Bench: Gnaw's 20 plus 20 for the [L]
/// Weakness collects the 2 points that end the game. Chansey stays and hits for 60 instead, and
/// survives Electro Ball's 70 with 50 HP to spare.
fn keep_the_damaged_ex_off_the_active_spot() -> Puzzle {
    let mut game = position(
        puzzle_deck(
            &[
                CardId::A1202Chansey,
                CardId::A1074Staryu,
                CardId::A1076StarmieEx,
            ],
            &[EnergyType::Lightning],
        ),
        puzzle_deck(
            &[CardId::A1100Electrode, CardId::A1094Pikachu],
            &[EnergyType::Lightning],
        ),
    );
    game.set_board(
        vec![
            PlayedCard::from_id(CardId::A1202Chansey).with_energy(vec![
                EnergyType::Lightning,
                EnergyType::Lightning,
                EnergyType::Lightning,
            ]),
            PlayedCard::from_id(CardId::A1076StarmieEx)
                .with_energy(vec![EnergyType::Water, EnergyType::Water])
                .with_remaining_hp(20),
        ],
        vec![
            PlayedCard::from_id(CardId::A1100Electrode)
                .with_energy(vec![EnergyType::Lightning, EnergyType::Lightning]),
            PlayedCard::from_id(CardId::A1094Pikachu).with_energy(vec![EnergyType::Lightning]),
        ],
    );
    game.points = [0, 2];
    game.energy_zone[0] = spent_zone(EnergyType::Lightning);
    Puzzle {
        name: "keep_the_damaged_ex_off_the_active_spot",
        why:
            "Promoting the 20 HP ex hands over the 2 points that win; Chansey attacks and survives.",
        game,
        acceptable: vec![attack("Gentle Slap")],
    }
}

/// The Active Spot is empty and the game is waiting for a promotion. Magikarp and Poliwag have no
/// Energy and 10 damage attacks; Hitmonchan is holding its [F] and Jab's 30 covers the opponent's
/// 30 remaining HP for the third point next turn.
fn promote_the_attacker_after_the_knockout() -> Puzzle {
    let mut game = position(
        puzzle_deck(
            &[
                CardId::A1077Magikarp,
                CardId::A1155Hitmonchan,
                CardId::A1059Poliwag,
            ],
            &[EnergyType::Fighting],
        ),
        puzzle_deck(
            &[CardId::A1189Rattata, CardId::A1059Poliwag],
            &[EnergyType::Water],
        ),
    );
    game.set_board(
        vec![],
        vec![
            PlayedCard::from_id(CardId::A1189Rattata).with_remaining_hp(30),
            PlayedCard::from_id(CardId::A1059Poliwag),
        ],
    );
    // The Active Spot is empty, which `set_board` cannot express: it fills from index 0 upwards.
    game.in_play_pokemon[0] = [
        None,
        Some(PlayedCard::from_id(CardId::A1077Magikarp)),
        Some(PlayedCard::from_id(CardId::A1155Hitmonchan).with_energy(vec![EnergyType::Fighting])),
        Some(PlayedCard::from_id(CardId::A1059Poliwag)),
    ];
    game.move_generation_stack.push((
        0,
        (1..=3)
            .map(|in_play_idx| SimpleAction::Activate {
                player: 0,
                in_play_idx,
            })
            .collect(),
    ));
    game.points = [2, 0];
    game.energy_zone[0] = spent_zone(EnergyType::Fighting);
    Puzzle {
        name: "promote_the_attacker_after_the_knockout",
        why: "Only Hitmonchan is paid up, and its Jab covers the opponent's 30 remaining HP.",
        game,
        acceptable: vec![activate(2)],
    }
}

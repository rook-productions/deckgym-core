//! Puzzles about seeing a Knock Out and taking the biggest one on offer.

use super::support::{attack, play, position, puzzle_deck, spent_zone};
use super::Puzzle;
use crate::card_ids::CardId;
use crate::database::get_card_by_enum;
use crate::models::{EnergyType, PlayedCard};

pub(super) fn puzzles() -> Vec<Puzzle> {
    vec![
        take_the_available_knockout(),
        weakness_makes_the_knockout(),
        bigger_attack_takes_the_knockout(),
        winning_attack_beats_the_safe_retreat(),
        cyrus_pulls_the_damaged_ex_for_two_points(),
        sabrina_forces_up_the_only_bench_target(),
        giovanni_completes_the_knockout(),
    ]
}

/// Hitmonchan's Jab costs [F] and does 30. Magikarp has 30 HP. Nothing else on the board matters.
fn take_the_available_knockout() -> Puzzle {
    let mut game = position(
        puzzle_deck(
            &[CardId::A1155Hitmonchan, CardId::A1143Machop],
            &[EnergyType::Fighting],
        ),
        puzzle_deck(
            &[CardId::A1077Magikarp, CardId::A1059Poliwag],
            &[EnergyType::Water],
        ),
    );
    game.set_board(
        vec![
            PlayedCard::from_id(CardId::A1155Hitmonchan).with_energy(vec![EnergyType::Fighting]),
            PlayedCard::from_id(CardId::A1143Machop),
        ],
        vec![
            PlayedCard::from_id(CardId::A1077Magikarp),
            PlayedCard::from_id(CardId::A1059Poliwag),
        ],
    );
    game.energy_zone[0] = spent_zone(EnergyType::Fighting);
    Puzzle {
        name: "take_the_available_knockout",
        why: "Jab does 30 and Magikarp has 30 HP, so the attack is a free point.",
        game,
        acceptable: vec![attack("Jab")],
    }
}

/// Pikachu's Gnaw does 20. Squirtle is Weak to [L], and Weakness is a flat +20 on the Active
/// Spot, so Gnaw does 40 to a Squirtle sitting on 40 remaining HP. Without the Weakness bonus
/// there is no Knock Out here at all.
fn weakness_makes_the_knockout() -> Puzzle {
    let mut game = position(
        puzzle_deck(
            &[CardId::A1094Pikachu, CardId::A1189Rattata],
            &[EnergyType::Lightning],
        ),
        puzzle_deck(
            &[CardId::A1053Squirtle, CardId::A1059Poliwag],
            &[EnergyType::Water],
        ),
    );
    game.set_board(
        vec![
            PlayedCard::from_id(CardId::A1094Pikachu).with_energy(vec![EnergyType::Lightning]),
            PlayedCard::from_id(CardId::A1189Rattata),
        ],
        vec![
            PlayedCard::from_id(CardId::A1053Squirtle).with_remaining_hp(40),
            PlayedCard::from_id(CardId::A1059Poliwag),
        ],
    );
    game.energy_zone[0] = spent_zone(EnergyType::Lightning);
    Puzzle {
        name: "weakness_makes_the_knockout",
        why: "Gnaw's 20 plus 20 for Squirtle's [L] Weakness is exactly its 40 remaining HP.",
        game,
        acceptable: vec![attack("Gnaw")],
    }
}

/// Mewtwo ex holds four [P]. Psychic Sphere does 50 and leaves Chansey alive on 120 HP;
/// Psydrive does 150 and Knocks it Out, at the cost of discarding two [P] afterwards.
fn bigger_attack_takes_the_knockout() -> Puzzle {
    let mut game = position(
        puzzle_deck(
            &[CardId::A1129MewtwoEx, CardId::A1189Rattata],
            &[EnergyType::Psychic],
        ),
        puzzle_deck(
            &[CardId::A1202Chansey, CardId::A1189Rattata],
            &[EnergyType::Fighting],
        ),
    );
    game.set_board(
        vec![
            PlayedCard::from_id(CardId::A1129MewtwoEx).with_energy(vec![
                EnergyType::Psychic,
                EnergyType::Psychic,
                EnergyType::Psychic,
                EnergyType::Psychic,
            ]),
            PlayedCard::from_id(CardId::A1189Rattata),
        ],
        vec![
            PlayedCard::from_id(CardId::A1202Chansey),
            PlayedCard::from_id(CardId::A1189Rattata),
        ],
    );
    game.energy_zone[0] = spent_zone(EnergyType::Psychic);
    Puzzle {
        name: "bigger_attack_takes_the_knockout",
        why: "Only Psydrive's 150 reaches Chansey's 120 HP; the Energy discard is worth a point.",
        game,
        acceptable: vec![attack("Psydrive")],
    }
}

/// The Active is on 20 HP and there is a full-health Chansey to hide behind, but Jab takes the
/// third point and the game ends before the opponent ever attacks again.
fn winning_attack_beats_the_safe_retreat() -> Puzzle {
    let mut game = position(
        puzzle_deck(
            &[CardId::A1155Hitmonchan, CardId::A1202Chansey],
            &[EnergyType::Fighting],
        ),
        puzzle_deck(
            &[CardId::A1077Magikarp, CardId::A1059Poliwag],
            &[EnergyType::Water],
        ),
    );
    game.set_board(
        vec![
            PlayedCard::from_id(CardId::A1155Hitmonchan)
                .with_energy(vec![EnergyType::Fighting])
                .with_remaining_hp(20),
            PlayedCard::from_id(CardId::A1202Chansey).with_energy(vec![
                EnergyType::Fighting,
                EnergyType::Fighting,
                EnergyType::Fighting,
            ]),
        ],
        vec![
            PlayedCard::from_id(CardId::A1077Magikarp),
            PlayedCard::from_id(CardId::A1059Poliwag),
        ],
    );
    game.points = [2, 0];
    game.energy_zone[0] = spent_zone(EnergyType::Fighting);
    Puzzle {
        name: "winning_attack_beats_the_safe_retreat",
        why: "Jab's 30 Knocks Out the 30 HP Magikarp for the third point, which ends the game.",
        game,
        acceptable: vec![attack("Jab")],
    }
}

/// Jab can Knock Out either the Active Magikarp for one point or, after Cyrus drags it up, the
/// damaged Starmie ex for two. Only the two-point line reaches three points.
fn cyrus_pulls_the_damaged_ex_for_two_points() -> Puzzle {
    let mut game = position(
        puzzle_deck(
            &[
                CardId::A1155Hitmonchan,
                CardId::A1143Machop,
                CardId::A2150Cyrus,
            ],
            &[EnergyType::Fighting],
        ),
        puzzle_deck(
            &[
                CardId::A1077Magikarp,
                CardId::A1074Staryu,
                CardId::A1076StarmieEx,
            ],
            &[EnergyType::Water],
        ),
    );
    game.set_board(
        vec![
            PlayedCard::from_id(CardId::A1155Hitmonchan).with_energy(vec![EnergyType::Fighting]),
            PlayedCard::from_id(CardId::A1143Machop),
        ],
        vec![
            PlayedCard::from_id(CardId::A1077Magikarp),
            PlayedCard::from_id(CardId::A1076StarmieEx).with_remaining_hp(20),
        ],
    );
    game.hands[0] = vec![get_card_by_enum(CardId::A2150Cyrus)];
    game.points = [1, 0];
    game.energy_zone[0] = spent_zone(EnergyType::Fighting);
    Puzzle {
        name: "cyrus_pulls_the_damaged_ex_for_two_points",
        why:
            "Cyrus switches the damaged Starmie ex in, and Knocking it Out is 2 points for the win.",
        game,
        acceptable: vec![play("Cyrus")],
    }
}

/// Snorlax has 150 HP and cannot be Knocked Out, but Sabrina switches it out and the opponent's
/// only Benched Pokemon is a Rattata on 20 remaining HP.
fn sabrina_forces_up_the_only_bench_target() -> Puzzle {
    let mut game = position(
        puzzle_deck(
            &[
                CardId::A1155Hitmonchan,
                CardId::A1143Machop,
                CardId::A1225Sabrina,
            ],
            &[EnergyType::Fighting],
        ),
        puzzle_deck(
            &[CardId::A1211Snorlax, CardId::A1189Rattata],
            &[EnergyType::Fighting],
        ),
    );
    game.set_board(
        vec![
            PlayedCard::from_id(CardId::A1155Hitmonchan).with_energy(vec![EnergyType::Fighting]),
            PlayedCard::from_id(CardId::A1143Machop),
        ],
        vec![
            PlayedCard::from_id(CardId::A1211Snorlax),
            PlayedCard::from_id(CardId::A1189Rattata).with_remaining_hp(20),
        ],
    );
    game.hands[0] = vec![get_card_by_enum(CardId::A1225Sabrina)];
    game.points = [2, 0];
    game.energy_zone[0] = spent_zone(EnergyType::Fighting);
    Puzzle {
        name: "sabrina_forces_up_the_only_bench_target",
        why:
            "Sabrina leaves the opponent only the 20 HP Rattata to promote, and Jab wins the game.",
        game,
        acceptable: vec![play("Sabrina")],
    }
}

/// Gnaw does 20 into a Rattata on 30 remaining HP. Giovanni's "+10 damage to your opponent's
/// Active Pokemon" turns that into exactly lethal.
fn giovanni_completes_the_knockout() -> Puzzle {
    let mut game = position(
        puzzle_deck(
            &[
                CardId::A1094Pikachu,
                CardId::A1189Rattata,
                CardId::A1223Giovanni,
            ],
            &[EnergyType::Lightning],
        ),
        puzzle_deck(
            &[CardId::A1189Rattata, CardId::A1059Poliwag],
            &[EnergyType::Fighting],
        ),
    );
    game.set_board(
        vec![
            PlayedCard::from_id(CardId::A1094Pikachu).with_energy(vec![EnergyType::Lightning]),
            PlayedCard::from_id(CardId::A1189Rattata),
        ],
        vec![
            PlayedCard::from_id(CardId::A1189Rattata).with_remaining_hp(30),
            PlayedCard::from_id(CardId::A1059Poliwag),
        ],
    );
    game.hands[0] = vec![get_card_by_enum(CardId::A1223Giovanni)];
    game.points = [2, 0];
    game.energy_zone[0] = spent_zone(EnergyType::Lightning);
    Puzzle {
        name: "giovanni_completes_the_knockout",
        why: "Gnaw's 20 plus Giovanni's +10 is the Rattata's 30 remaining HP and the third point.",
        game,
        acceptable: vec![play("Giovanni")],
    }
}

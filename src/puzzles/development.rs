//! Puzzles about spending the turn on the board: evolving, Rare Candy, Supporters, the Stadium,
//! and putting the turn's Energy where it does work.

use super::support::{
    anything_but_evolving_into, attach_to, attack, evolve_at, place_on_bench, play, position,
    puzzle_deck, spent_zone, zone,
};
use super::Puzzle;
use crate::card_ids::CardId;
use crate::database::get_card_by_enum;
use crate::models::{EnergyType, PlayedCard};

pub(super) fn puzzles() -> Vec<Puzzle> {
    vec![
        evolve_before_attacking(),
        do_not_evolve_out_of_the_winning_attack(),
        rare_candy_enables_the_knockout(),
        do_not_evolve_into_the_two_point_target(),
        professors_research_when_the_board_is_empty_of_basics(),
        bench_a_second_attacker_before_the_active_dies(),
        stadium_that_swings_the_damage(),
        attach_to_the_active_that_can_attack_now(),
    ]
}

/// Squirtle's Water Gun does 20 and leaves the Rattata alive on 40 HP. Evolving first costs the
/// same Energy and Wave Splash's 40 takes the third point in the same turn.
fn evolve_before_attacking() -> Puzzle {
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
    game.hands[0] = vec![get_card_by_enum(CardId::A1054Wartortle)];
    game.points = [2, 0];
    game.energy_zone[0] = spent_zone(EnergyType::Water);
    Puzzle {
        name: "evolve_before_attacking",
        why: "Attacking as Squirtle does 20 into 40 HP; evolving first makes the same turn lethal.",
        game,
        acceptable: vec![evolve_at(0, CardId::A1054Wartortle)],
    }
}

/// The control for the evolution puzzle. Water Gun costs one [W] and its 20 clears the Rattata's
/// 20 remaining HP for the third point. Wartortle is in hand and evolving is free, but Wave
/// Splash costs [W][C] and Squirtle is holding a single [W], so evolving trades the win for a
/// Pokemon that cannot attack at all this turn.
fn do_not_evolve_out_of_the_winning_attack() -> Puzzle {
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
            PlayedCard::from_id(CardId::A1053Squirtle).with_energy(vec![EnergyType::Water]),
            PlayedCard::from_id(CardId::A1189Rattata),
        ],
        vec![
            PlayedCard::from_id(CardId::A1189Rattata).with_remaining_hp(20),
            PlayedCard::from_id(CardId::A1059Poliwag),
        ],
    );
    game.hands[0] = vec![get_card_by_enum(CardId::A1054Wartortle)];
    game.points = [2, 0];
    game.energy_zone[0] = spent_zone(EnergyType::Water);
    Puzzle {
        name: "do_not_evolve_out_of_the_winning_attack",
        why: "Water Gun's 20 wins now; Wartortle's Wave Splash needs a second Energy Squirtle has not got.",
        game,
        acceptable: vec![attack("Water Gun")],
    }
}

/// Machop holds three [F] and Machamp is in hand with no Machoke anywhere, so Rare Candy is the
/// only way to reach Seismic Toss. Knuckle Punch does 20 plus 20 for Chansey's [F] Weakness and
/// leaves it on 60; Seismic Toss does 100 plus the same 20 and clears its 100 remaining HP.
fn rare_candy_enables_the_knockout() -> Puzzle {
    let mut game = position(
        puzzle_deck(
            &[
                CardId::A1143Machop,
                CardId::A1145Machamp,
                CardId::A3144RareCandy,
            ],
            &[EnergyType::Fighting],
        ),
        puzzle_deck(
            &[CardId::A1202Chansey, CardId::A1059Poliwag],
            &[EnergyType::Water],
        ),
    );
    game.set_board(
        vec![
            PlayedCard::from_id(CardId::A1143Machop).with_energy(vec![
                EnergyType::Fighting,
                EnergyType::Fighting,
                EnergyType::Fighting,
            ]),
            PlayedCard::from_id(CardId::A1189Rattata),
        ],
        vec![
            PlayedCard::from_id(CardId::A1202Chansey).with_remaining_hp(100),
            PlayedCard::from_id(CardId::A1059Poliwag),
        ],
    );
    game.hands[0] = vec![
        get_card_by_enum(CardId::A1145Machamp),
        get_card_by_enum(CardId::A3144RareCandy),
    ];
    game.points = [2, 0];
    game.energy_zone[0] = spent_zone(EnergyType::Fighting);
    Puzzle {
        name: "rare_candy_enables_the_knockout",
        why: "Rare Candy skips Machoke, and Seismic Toss plus Weakness clears Chansey for the win.",
        game,
        acceptable: vec![play("Rare Candy")],
    }
}

/// Staryu is on 10 remaining HP and the damage carries onto whatever it evolves into. Electro
/// Ball does 70 plus 20 for the [L] Weakness, which is 90: exactly the 130 HP Starmie ex minus
/// the 40 damage already on Staryu. Evolving therefore converts a 1-point loss into the 2-point
/// loss that puts the opponent on three. Staryu retreats for one [W] instead, and Hitmonchan has
/// no [L] Weakness, so the same Electro Ball does only 70 into its 80 HP.
fn do_not_evolve_into_the_two_point_target() -> Puzzle {
    let mut game = position(
        puzzle_deck(
            &[
                CardId::A1074Staryu,
                CardId::A1076StarmieEx,
                CardId::A1155Hitmonchan,
            ],
            &[EnergyType::Water],
        ),
        puzzle_deck(
            &[CardId::A1099Voltorb, CardId::A1100Electrode],
            &[EnergyType::Lightning],
        ),
    );
    game.set_board(
        vec![
            PlayedCard::from_id(CardId::A1074Staryu)
                .with_energy(vec![EnergyType::Water])
                .with_remaining_hp(10),
            PlayedCard::from_id(CardId::A1155Hitmonchan).with_energy(vec![EnergyType::Fighting]),
        ],
        vec![
            PlayedCard::from_id(CardId::A1100Electrode)
                .with_energy(vec![EnergyType::Lightning, EnergyType::Lightning]),
            PlayedCard::from_id(CardId::A1059Poliwag),
        ],
    );
    game.hands[0] = vec![get_card_by_enum(CardId::A1076StarmieEx)];
    game.points = [1, 1];
    game.energy_zone[0] = spent_zone(EnergyType::Water);
    Puzzle {
        name: "do_not_evolve_into_the_two_point_target",
        why: "Electro Ball plus Weakness is exactly lethal on the evolved Starmie ex, and it pays 2 points.",
        game,
        acceptable: vec![anything_but_evolving_into(CardId::A1076StarmieEx)],
    }
}

/// Hitmonchan has no Energy, so there is no attack and no retreat, and the Bench is empty.
/// Professor's Research draws two, which is the only way this turn improves anything.
fn professors_research_when_the_board_is_empty_of_basics() -> Puzzle {
    let mut game = position(
        puzzle_deck(
            &[
                CardId::A1155Hitmonchan,
                CardId::A1143Machop,
                CardId::PA007ProfessorsResearch,
            ],
            &[EnergyType::Fighting],
        ),
        puzzle_deck(
            &[CardId::A1211Snorlax, CardId::A1059Poliwag],
            &[EnergyType::Fighting],
        ),
    );
    game.set_board(
        vec![PlayedCard::from_id(CardId::A1155Hitmonchan)],
        vec![
            PlayedCard::from_id(CardId::A1211Snorlax),
            PlayedCard::from_id(CardId::A1059Poliwag),
        ],
    );
    game.hands[0] = vec![get_card_by_enum(CardId::PA007ProfessorsResearch)];
    game.energy_zone[0] = spent_zone(EnergyType::Fighting);
    Puzzle {
        name: "professors_research_when_the_board_is_empty_of_basics",
        why: "Drawing 2 is free and is the only route to a Basic for the empty Bench; passing is not.",
        game,
        acceptable: vec![play("Professor's Research")],
    }
}

/// Magikarp has 30 HP and the opponent's Pikachu is holding its [L]: Gnaw's 20 plus 20 for
/// Magikarp's [L] Weakness is 40. With an empty Bench that Knock Out ends the game on the spot,
/// so the Basic in hand has to go down now.
fn bench_a_second_attacker_before_the_active_dies() -> Puzzle {
    let mut game = position(
        puzzle_deck(
            &[CardId::A1077Magikarp, CardId::A1155Hitmonchan],
            &[EnergyType::Water],
        ),
        puzzle_deck(
            &[CardId::A1094Pikachu, CardId::A1059Poliwag],
            &[EnergyType::Lightning],
        ),
    );
    game.set_board(
        vec![PlayedCard::from_id(CardId::A1077Magikarp)],
        vec![
            PlayedCard::from_id(CardId::A1094Pikachu).with_energy(vec![EnergyType::Lightning]),
            PlayedCard::from_id(CardId::A1059Poliwag),
        ],
    );
    game.hands[0] = vec![get_card_by_enum(CardId::A1155Hitmonchan)];
    game.energy_zone[0] = spent_zone(EnergyType::Water);
    Puzzle {
        name: "bench_a_second_attacker_before_the_active_dies",
        why: "Gnaw plus Weakness clears Magikarp's 30 HP, and an empty Bench loses the game outright.",
        game,
        acceptable: vec![place_on_bench(CardId::A1155Hitmonchan)],
    }
}

/// Training Area gives Stage 1 attackers +10 against the Active Pokemon. Starmie's Wave Splash
/// does 40 into a Chansey on 50 remaining HP; with the Stadium down it does 50. The opponent has
/// only Basics in play, so the Stadium gives them nothing back.
fn stadium_that_swings_the_damage() -> Puzzle {
    let mut game = position(
        puzzle_deck(
            &[
                CardId::A1074Staryu,
                CardId::A1075Starmie,
                CardId::B2153TrainingArea,
            ],
            &[EnergyType::Water],
        ),
        puzzle_deck(
            &[CardId::A1202Chansey, CardId::A1059Poliwag],
            &[EnergyType::Water],
        ),
    );
    game.set_board(
        vec![
            PlayedCard::from_id(CardId::A1075Starmie).with_energy(vec![EnergyType::Water]),
            PlayedCard::from_id(CardId::A1189Rattata),
        ],
        vec![
            PlayedCard::from_id(CardId::A1202Chansey).with_remaining_hp(50),
            PlayedCard::from_id(CardId::A1059Poliwag),
        ],
    );
    game.hands[0] = vec![get_card_by_enum(CardId::B2153TrainingArea)];
    game.points = [2, 0];
    game.energy_zone[0] = spent_zone(EnergyType::Water);
    Puzzle {
        name: "stadium_that_swings_the_damage",
        why: "Training Area's +10 for a Stage 1 attacker turns Wave Splash's 40 into the exact 50 needed.",
        game,
        acceptable: vec![play("Training Area")],
    }
}

/// The control for the two "put it on the Bench" puzzles: one [F] on the Active Hitmonchan makes
/// Jab live this turn and Knocks Out a 30 HP Magikarp for the third point. The same [F] on the
/// Bench Chansey does nothing until next turn, by which time the game is already over.
fn attach_to_the_active_that_can_attack_now() -> Puzzle {
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
            PlayedCard::from_id(CardId::A1155Hitmonchan),
            PlayedCard::from_id(CardId::A1202Chansey)
                .with_energy(vec![EnergyType::Fighting, EnergyType::Fighting]),
        ],
        vec![
            PlayedCard::from_id(CardId::A1077Magikarp),
            PlayedCard::from_id(CardId::A1059Poliwag),
        ],
    );
    game.points = [2, 0];
    game.energy_zone[0] = zone(EnergyType::Fighting);
    Puzzle {
        name: "attach_to_the_active_that_can_attack_now",
        why: "One [F] on Hitmonchan makes Jab's 30 lethal this turn; on the Bench it is a turn too late.",
        game,
        acceptable: vec![attach_to(0)],
    }
}

//! Runs the tactical puzzles in `deckgym::puzzles` against one or more bots.
//!
//! ```text
//! cargo run --release --bin bot_puzzles -- --players e
//! cargo run --release --bin bot_puzzles -- --players r,v,e2,e --verbose
//! ```
//!
//! Each puzzle asks the bot for exactly one action from a fixed position and compares it with the
//! puzzle's accepted set. Bots that consume randomness do not answer the same way twice, so every
//! puzzle is asked `--trials` times (default 5) and the majority verdict is what counts.

use clap::Parser;
use deckgym::players::{parse_player_code, PlayerCode};
use deckgym::puzzles::{acceptance, all_puzzles, chance_score, pass_count, run_puzzle};

#[derive(Parser, Debug)]
#[command(author, version, about = "Run the deckgym tactical puzzles against a bot", long_about = None)]
struct Cli {
    /// Bot codes to test, comma separated (e.g. "e" or "r,v,e2,e,m").
    #[arg(long, value_delimiter = ',', value_parser = parse_player_code)]
    players: Vec<PlayerCode>,

    /// Times each puzzle is asked. The majority answer is the verdict.
    #[arg(long, default_value_t = 5)]
    trials: usize,

    /// Print the accepted set and the justification for every puzzle, not just the failures.
    #[arg(short, long, default_value_t = false)]
    verbose: bool,
}

fn main() {
    let cli = Cli::parse();
    let codes = if cli.players.is_empty() {
        vec![PlayerCode::E { max_depth: 3 }]
    } else {
        cli.players.clone()
    };

    let puzzles = all_puzzles();
    println!("{} puzzles, {} trials each", puzzles.len(), cli.trials);
    println!(
        "picking uniformly at random scores {:.1}/{} in expectation, so that is the floor",
        chance_score(cli.trials),
        puzzles.len()
    );
    if cli.verbose {
        for puzzle in &puzzles {
            let (accepted, legal) = acceptance(puzzle);
            println!(
                "      {:<52} {accepted} of {legal} legal actions accepted",
                puzzle.name
            );
        }
    }

    for code in &codes {
        println!();
        println!("=== {code:?}");
        let started = std::time::Instant::now();
        let mut outcomes = Vec::with_capacity(puzzles.len());
        for puzzle in &puzzles {
            let outcome = run_puzzle(puzzle, code, cli.trials);
            let mark = if outcome.passed { "PASS" } else { "FAIL" };
            println!(
                "{mark}  {:<52} {}/{}  chose {}",
                outcome.name,
                outcome.votes,
                outcome.trials,
                outcome.chosen.join(", ")
            );
            if cli.verbose || !outcome.passed {
                println!("        wanted: {}", outcome.expected);
                println!("        why:    {}", outcome.why);
            }
            outcomes.push(outcome);
        }
        let passed = pass_count(&outcomes);
        println!(
            "{code:?}: {passed}/{} puzzles in {:.1}s",
            outcomes.len(),
            started.elapsed().as_secs_f64()
        );
    }
}

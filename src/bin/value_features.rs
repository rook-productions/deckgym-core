//! Print the learned value function's feature vector for an exported ply file.
//!
//! ```text
//! cargo run --release --bin value_features -- <ply.json>
//! ```
//!
//! The file is one of the `DIR/<game_id>/ply_NNNN.json` files that
//! `deckgym simulate --data-output DIR` writes. Its `actor` field is the acting player, and its
//! `state` field is parsed back into a `State`, so the numbers printed here are computed by the
//! engine from a real `State` and not by re-reading the JSON.
//!
//! Output is a JSON object with a `features` key holding the 41 numbers in
//! `LEARNED_FEATURE_NAMES` order. That is one of the three shapes
//! `sim/learn/features_crosscheck.py` in the Pocket Lab repo accepts; that script is the test that
//! this binary and `sim/learn/features.py` agree, feature by feature, on real exported positions.
//! `docs/value-features.md` is the specification both of them implement.

use deckgym::players::value_functions::{learned_value_features, LEARNED_FEATURE_NAMES};
use deckgym::State;
use serde_json::{json, Value};
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("usage: value_features <ply.json> [--named]");
        eprintln!("  --named  print a name-to-value map instead of an array");
        return ExitCode::from(2);
    };
    let named = args.any(|arg| arg == "--named");

    match features_for(&path) {
        Ok(features) => {
            let body = if named {
                json!(LEARNED_FEATURE_NAMES
                    .iter()
                    .zip(features.iter())
                    .map(|(name, value)| (name.to_string(), json!(*value)))
                    .collect::<serde_json::Map<String, Value>>())
            } else {
                json!(features)
            };
            let output = json!({"file": path, "features": body});
            println!("{output}");
            ExitCode::SUCCESS
        }
        Err(problem) => {
            eprintln!("value_features: {problem}");
            ExitCode::FAILURE
        }
    }
}

fn features_for(path: &str) -> Result<Vec<f64>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("cannot read {path}: {e}"))?;
    let ply: Value = serde_json::from_str(&text).map_err(|e| format!("{path} is not JSON: {e}"))?;

    let actor = ply
        .get("actor")
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("{path} has no numeric `actor` field"))?;
    if actor > 1 {
        return Err(format!("{path} has actor {actor}, expected 0 or 1"));
    }

    let state_json = ply
        .get("state")
        .ok_or_else(|| format!("{path} has no `state` field"))?;
    let state: State = serde_json::from_value(state_json.clone())
        .map_err(|e| format!("{path} `state` is not a deckgym State: {e}"))?;

    Ok(learned_value_features(&state, actor as usize))
}

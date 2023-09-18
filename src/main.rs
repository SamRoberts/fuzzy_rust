use clap::Parser;
use fuzzy::{Problem, score};
use fuzzy::pattern::Pattern;
use std::error::Error;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The regex pattern to match TEXT.
    pattern: String,

    /// The text to be matched.
    text: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    // TODO extract nicer API out of lib.rs

    let pattern = Pattern::parse(&args.pattern)?;

    let problem = Problem::new(pattern, args.text);

    let state = score(&problem);

    if let Some(trace) = state.trace(&problem) {

        let ix = problem.start_ix();
        let score = state.score_ix(&ix).unwrap();
        println!("score {} at {:?} <-> {:?}", score, problem.pattern[ix.pix], problem.text[ix.tix]);
        for ix in trace.iter() {
            let score = state.score_ix(&ix).unwrap();
            println!("score {} at {:?} <-> {:?}", score, problem.pattern[ix.pix], problem.text[ix.tix]);
        }
        Ok(())
    } else {
        panic!("Failed to solve problem, and not familiar with standard rust error handling yet ...");
    }
}

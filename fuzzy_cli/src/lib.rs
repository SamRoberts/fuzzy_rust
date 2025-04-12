use clap::Parser;
use fuzzy::Output;
use fuzzy::diff_output::DiffOutput;
use fuzzy::table_solution::TableSolution;
use fuzzy::regex_question::RegexQuestion;
use fuzzy::error::Error;
use std::fs;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// File containing the regex pattern to match TEXT.
    pattern: String,

    /// File containing the text to be matched.
    text: String,

    /// PATTERN and TEXT args are raw pattern/text values rather than file names
    #[arg(short, long)]
    inline: bool,
}

pub fn run(args: Args) -> Result<String, Error> {
    let pattern_regex = if args.inline {
        args.pattern
    } else {
        fs::read_to_string(args.pattern)?
    };
    let text = if args.inline {
        args.text
    } else {
        fs::read_to_string(args.text)?
    };

    let question = RegexQuestion { pattern_regex, text };
    run_impl::<DiffOutput>(question)
}

fn run_impl<O: Output>(question: RegexQuestion) -> Result<String, Error> {
    let problem = question.ask()?;
    let problem_core = problem.desugar();
    let solution = TableSolution::solve(&problem_core)?;
    let output = O::new(&solution.score(), &solution.trace());
    Ok(format!("{}", output))
}

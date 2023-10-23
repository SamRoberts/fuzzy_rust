use clap::Parser;
use fuzzy::{Output, Question, Solution};
use fuzzy::diff_output::DiffOutput;
use fuzzy::table_solution::TableSolution;
use fuzzy::regex_question::RegexQuestion;
use fuzzy::error::Error;
use std::fs;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// File containing the regex pattern to match TEXT.
    pattern: String,

    /// File containing the text to be matched.
    text: String,

    /// PATTERN and TEXT args are raw pattern/text values rather than file names
    #[arg(short, long)]
    inline: bool,
}

fn main() -> Result<(), Error> {
    let args = Args::parse();
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
    main_impl::<RegexQuestion, TableSolution, DiffOutput>(question)
}

fn main_impl<Q: Question<Error>, S: Solution<Error>, O: Output>(question: Q) -> Result<(), Error> {
    let problem = question.ask()?;
    let solution = S::solve(&problem)?;
    let output = O::new(&problem, &solution.score(), &solution.trace());
    println!("{}", output);

    Ok(())
}

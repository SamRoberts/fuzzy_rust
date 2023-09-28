use clap::Parser;
use fuzzy::{Output, Question, Solution};
use fuzzy::diff_output::DiffOutput;
use fuzzy::table_solution::TableSolution;
use fuzzy::regex_question::RegexQuestion;
use fuzzy::error::Error;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The regex pattern to match TEXT.
    pattern: String,

    /// The text to be matched.
    text: String,
}

fn main() -> Result<(), Error> {
    let args = Args::parse();
    let question = RegexQuestion {
        pattern_regex: args.pattern,
        text: args.text
    };
    main_impl::<RegexQuestion, TableSolution, DiffOutput>(question)
}

fn main_impl<Q: Question<Error>, S: Solution<Error>, O: Output>(question: Q) -> Result<(), Error> {
    let problem = question.ask()?;
    let solution = S::solve(&problem)?;
    let output = O::new(&problem, solution.score(), solution.trace());
    println!("{}", output);

    Ok(())
}

use clap::Parser;
use fuzzy::{Question, Solution};
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
    main_impl::<RegexQuestion, TableSolution>(question)
}

fn main_impl<Q: Question<Error>, S: Solution<Error>>(question: Q) -> Result<(), Error> {
    let problem = question.ask()?;
    let solution = S::solve(&problem)?;
    for step in solution.trace().iter() {
        println!("{:?}", step);
    }

    Ok(())
}

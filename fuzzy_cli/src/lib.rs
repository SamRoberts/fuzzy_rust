use clap::Parser;
use fuzzy;
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

    let output = fuzzy::fuzzy_match(pattern_regex, text)?;
    Ok(format!("{}", output))
}

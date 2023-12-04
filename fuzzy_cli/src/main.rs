use clap::Parser;
use fuzzy::error::Error;
use fuzzy_cli::{Args, run};

fn main() -> Result<(), Error> {
    let args = Args::parse();
    let output = run(args)?;
    println!("{}", output);
    Ok(())
}

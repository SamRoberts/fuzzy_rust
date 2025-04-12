use fuzzy::diff_output::{Chunk, DiffOutput};
use fuzzy::table_solution::TableSolution;
use fuzzy::regex_question::RegexQuestion;
use lambda_http::{run, service_fn, Body, Error, Request, Response};
use serde::{Serialize, Deserialize};

#[derive(Deserialize)]
struct Args {
    /// The regex pattern to match TEXT.
    pattern: String,

    /// The text to be matched.
    text: String,
}

#[derive(Serialize)]
struct Out {
    score: usize,
    trace: Vec<OutChunk>,
}

#[derive(Serialize)]
enum OutChunk {
    Same(String),
    Taken(String),
    Added(String),
}

impl OutChunk {
    fn from(chunks: &Vec<Chunk>) -> Vec<OutChunk> {
        chunks.iter().flat_map(|chunk|
            match chunk {
                Chunk::Same(same) => vec![
                    OutChunk::Same(same.text.iter().collect()),
                ],
                Chunk::Diff(diff) if diff.taken.is_empty() => vec![
                    OutChunk::Added(diff.added.iter().collect()),
                ],
                Chunk::Diff(diff) if diff.added.is_empty() => vec![
                    OutChunk::Taken(diff.taken.iter().collect()),
                ],
                Chunk::Diff(diff) => vec![
                    OutChunk::Taken(diff.taken.iter().collect()),
                    OutChunk::Added(diff.added.iter().collect()),
                ],
            }
        ).collect()
    }
}

async fn function_handler(event: Request) -> Result<Response<Body>, Error> {
    let body_str = std::str::from_utf8(event.body())?;
    let args = serde_json::from_str::<Args>(body_str)?;

    let problem = RegexQuestion { pattern_regex: args.pattern, text: args.text }.ask()?;
    let problem_core = problem.desugar();
    let solution = TableSolution::solve(&problem_core)?;
    let output = DiffOutput::new(&solution.score, &solution.trace);
    let body = Out { score: solution.score, trace: OutChunk::from(&output.chunks) };
    let body_json = serde_json::to_string(&body)?;

    let resp = Response::builder()
        .status(200)
        .header("content-type", "text/json")
        .body(body_json.into())?;
    Ok(resp)
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        // disable printing the name of the module in every log line.
        .with_target(false)
        // disabling time is handy because CloudWatch will add the ingestion time.
        .without_time()
        .init();

    run(service_fn(function_handler)).await
}

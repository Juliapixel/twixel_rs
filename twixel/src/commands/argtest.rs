use crate::handler::extract::Clap;

#[derive(clap::Parser, Debug)]
pub struct TestArgs {
    #[arg(short, long)]
    target: String,
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,
    rest: String,
}

pub async fn argtest(Clap(args): Clap<TestArgs>) -> String {
    format!("{args:?}")
}

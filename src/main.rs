use clap::Parser;
use dateparser::DateTimeUtc;
use humantime::Duration;
use std::str::FromStr;

#[derive(Clone, Debug)]
enum DateTime {
    DateTime(DateTimeUtc),
    Ago(Duration),
}

impl FromStr for DateTime {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.strip_suffix(" ago") {
            Some(ago) => Self::Ago(Duration::from_str(ago)?),
            None => Self::DateTime(DateTimeUtc::from_str(s)?),
        })
    }
}

#[derive(Debug, Parser)]
struct Args {
    #[clap(long)]
    repo: String,

    #[clap(long)]
    after: Option<DateTime>,

    #[clap(long)]
    before: Option<DateTime>,
}

fn main() {
    let args = Args::parse();
    dbg!(&args);
}

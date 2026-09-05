use std::str::FromStr;

use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "simai.pest"]
struct SimaiParser;

#[derive(Debug, Clone)]
pub struct SimaiFile;

impl FromStr for SimaiFile {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let file = SimaiParser::parse(Rule::file, s);
        if let Err(file) = file {
            panic!("{}", file);
        }
        Ok(SimaiFile)
    }
}

use std::str::FromStr;

use simai_parser::SimaiFile;

#[test]
fn parse() {
    let file = include_str!("montagem_xonada.txt");
    let file = SimaiFile::from_str(file).unwrap();
}

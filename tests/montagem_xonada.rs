use simai_parser::SimaiFile;

#[test]
fn parse() {
    let file = include_str!("montagem_xonada.txt");
    let file = SimaiFile::parse(file).unwrap();
}

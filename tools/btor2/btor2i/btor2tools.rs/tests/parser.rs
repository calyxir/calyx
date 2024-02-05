use btor2tools::{Btor2Parser, Btor2ParserError};
use std::{env::current_dir, fs::read_dir};

#[test]
fn parse_btor2_examples() {
    let examples_dir = current_dir().unwrap().join("btor2tools/examples/btorsim");

    read_dir(&examples_dir)
        .unwrap()
        .map(|e| e.unwrap())
        .filter(|e| e.path().to_str().unwrap().ends_with(".btor2"))
        .for_each(|entry| {
            let path = entry.path();

            println!("parse: {}", path.display());

            Btor2Parser::new()
                .read_lines(&path)
                .unwrap()
                .for_each(|line| {
                    // panics of one something is not printable
                    println!("{:?}", line);
                })
        });
}

#[test]
fn parser_does_not_crash_if_file_does_not_exist() {
    let random_file_name = current_dir().unwrap().join("sakdfjaoisdfewhoiajofjds");

    let mut parser = Btor2Parser::new();

    let result = parser.read_lines(&random_file_name);

    assert!(
        matches!(result, Err(Btor2ParserError::CouldNotOpenFile(_))),
        "parser does not crash on when executed for non existing files"
    );
}

#[test]
fn parser_detects_invalid_btor2_file() {
    let invalid_btor2_file = current_dir().unwrap().join("tests/invalid.btor2");

    let mut parser = Btor2Parser::new();

    let result = parser.read_lines(&invalid_btor2_file);

    assert!(
        matches!(result, Err(Btor2ParserError::SyntaxError(_))),
        "parser does recognize BTOR2 files with invalid synthax"
    );
}

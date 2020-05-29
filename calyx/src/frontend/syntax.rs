use pest_derive::*;

const _GRAMMAR: &str = include_str!("grammar.pest");

#[derive(Parser)]
#[grammar = "frontend/grammar.pest"]
pub struct TestParser;

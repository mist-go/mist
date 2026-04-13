use pest_derive::Parser;

pub mod ast;

#[derive(Parser)]
#[grammar = "./src/parser/grammar.pest"] // relative to src
pub struct MistParser;

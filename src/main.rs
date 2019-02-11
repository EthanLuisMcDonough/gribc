#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;

mod ast;
mod lex;
mod location;
mod operators;
mod util;

fn main() {}

#[cfg(test)]
mod tests;

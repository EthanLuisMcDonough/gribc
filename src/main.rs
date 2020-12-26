#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;

mod ast;
mod lex;
mod location;
mod operators;
mod util;

use std::env;
use std::fs;
use std::path::Path;

macro_rules! err_guard {
    ($next:expr) => {
        match $next {
            Ok(v) => v,
            Err(e) => {
                println!("{:?}", e);
                panic!("{:?}", e);
            },
        };
    };
    ($next:expr, $e:ident => $b:expr) => {
        match $next {
            Ok(v) => v,
            Err($e) => $b,
        };
    }
}

fn main() {
    let path = env::args().last().expect("Error: File not found in cli arguments");
    
    let source = err_guard!(fs::read_to_string(&path), 
        _e => panic!("Error: could not load file {}", path));
    
    let tokens = err_guard!(lex::lex(source.as_str()));
    let tree = err_guard!(ast::ast(tokens.into_iter(), &path));

    println!("{}", serde_json::to_string_pretty(&tree).unwrap_or_default());

    err_guard!(ast::ref_check(&tree));
}

#[cfg(test)]
mod tests;

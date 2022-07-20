#[macro_use]
extern crate serde_derive;
extern crate rand;
extern crate serde;
extern crate serde_json;

mod ast;
mod lex;
mod location;
mod operators;
mod runtime;
mod util;

use std::env;
use std::fs;

macro_rules! err_guard {
    ($next:expr) => {
        match $next {
            Ok(v) => v,
            Err(e) => {
                println!("{:?}", e);
                panic!("{:?}", e);
            }
        }
    };
    ($next:expr, $e:ident => $b:expr) => {
        match $next {
            Ok(v) => v,
            Err($e) => $b,
        }
    };
}

fn main() {
    std::env::set_var("RUST_BACKTRACE", "1");
    let path = env::args()
        .last()
        .expect("Error: File not found in cli arguments");

    let source = err_guard!(fs::read_to_string(&path),
        _e => panic!("Error: could not load file {}", path));

    let tokens = err_guard!(lex::lex(source.as_str()));
    let mut tree = err_guard!(ast::ast(tokens.into_iter(), &path));

    err_guard!(ast::ref_check(&mut tree));
    println!("{}", serde_json::to_string_pretty(&tree).unwrap());

    runtime::execute(
        &tree,
        runtime::RuntimeConfig {
            cleanup_after: 1000,
        },
    );
}

#[cfg(test)]
mod tests;
//cargo rustc -- -C link-args=-Wl,-zstack-size=144016

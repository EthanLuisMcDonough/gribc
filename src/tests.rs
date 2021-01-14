use crate::serde::de::DeserializeOwned;

use ast::{ast, node::{Program, Node, Module}};
use lex::lex;

use std::error::Error;
use std::path::Path;
use std::ffi::OsStr;
use std::fmt::Debug;
use std::{fs, mem};

use util::remove_file;

#[derive(Clone, Copy, Debug)]
struct GenericErr;
impl std::fmt::Display for GenericErr {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(f, "{:?}", self)
    }
}
impl std::error::Error for GenericErr {}

trait Reversible<T> {
    fn invert(self) -> T;
}

impl<A, B> Reversible<Result<A, B>> for Result<B, A> {
    fn invert(self) -> Result<A, B> {
        match self {
            Ok(a) => Err(a),
            Err(a) => Ok(a),
        }
    }
}

// canonicalize paths in loaded test asts
fn canonicalize_ast(program: &mut Program, base: &Path) {
    let old_modules = mem::take(&mut program.modules);

    for (path, module) in old_modules {
        println!("{:?}", &base.join(&path));
        let new_path = fs::canonicalize(&base.join(&path))
            .expect("Invalid path in grib tests!");
        program.set_module(new_path, module);
    }
    
    let mut nodes = program.body.iter_mut();

    while let Some(Node::Import(ref mut import)) = nodes.next() {
        if let Module::Custom(ref mut path) = import.module {
            let new_path = base.join(&path.data)
                .as_path().canonicalize()
                .expect("Invalid path in grib tests!");
            path.data = new_path;
        }
    }
}

fn cmp_grib_json<T: DeserializeOwned + PartialEq + Debug>(
    grib: &str,
    json: &str,
    callback: impl Fn(&str, &Path) -> Result<T, Box<dyn Error>>,
    fix_test: impl Fn(&mut T, &Path),
) -> Result<(), Box<dyn Error>> {
    for file in fs::read_dir(grib)? {
        let file = file?;
        let mut json = serde_json::from_slice::<T>(
            fs::read(
                json.to_owned()
                    + "/"
                    + &file
                        .path()
                        .file_stem()
                        .and_then(OsStr::to_str)
                        .unwrap_or_default()
                    + ".json",
            )?
            .as_slice(),
        )?;

        let mut path = file.path();
        let file_contents = fs::read_to_string(&path)?;

        let grib = callback(&file_contents, path.as_path())?;

        remove_file(&mut path);
        fix_test(&mut json, path.as_path());

        assert_eq!(json, grib);
    }

    Ok(())
}

#[test]
fn ast_test_fail() -> Result<(), Box<dyn std::error::Error>> {
    cmp_grib_json(
        "./tests/ast_fail_tests/grib",
        "./tests/ast_fail_tests/ast",
        |s, path| ast(lex(s)?, path).map(|_| GenericErr.into()).invert(),
        |_, _| ()
    )
}

#[test]
fn ast_test() -> Result<(), Box<dyn std::error::Error>> {
    cmp_grib_json("./tests/ast_tests/grib", "./tests/ast_tests/ast", |s, path| {
        ast(lex(s)?, path).map_err(Box::from)
    }, canonicalize_ast)
}

use crate::serde::de::DeserializeOwned;
use std::error::Error;
use std::path::Path;
use std::ffi::OsStr;
use std::fmt::Debug;

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

fn cmp_grib_json<T: DeserializeOwned + PartialEq + Debug>(
    grib: &str,
    json: &str,
    callback: impl Fn(&str, &Path) -> Result<T, Box<dyn Error>>,
) -> Result<(), Box<dyn Error>> {
    use std::fs;

    for file in fs::read_dir(grib)? {
        let file = file?;
        let json = serde_json::from_slice::<T>(
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

        let path = file.path();

        let grib = callback(&String::from_utf8(fs::read(&path)?)?, path.as_path())?;
        assert_eq!(json, grib);
    }

    Ok(())
}

#[test]
fn ast_test_fail() -> Result<(), Box<dyn std::error::Error>> {
    use ast::ast;
    use lex::lex;

    cmp_grib_json(
        "./tests/ast_fail_tests/grib",
        "./tests/ast_fail_tests/ast",
        |s, path| ast(lex(s)?, path).map(|_| GenericErr.into()).invert(),
    )
}

#[test]
fn ast_test() -> Result<(), Box<dyn std::error::Error>> {
    use ast::ast;
    use lex::lex;

    cmp_grib_json("./tests/ast_tests/grib", "./tests/ast_tests/ast", |s, path| {
        ast(lex(s)?, path).map_err(Box::from)
    })
}

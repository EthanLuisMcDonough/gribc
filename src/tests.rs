use crate::serde::de::DeserializeOwned;
use std::error::Error;
use std::ffi::OsStr;
use std::fmt::Debug;

fn cmp_grib_json<T: DeserializeOwned + PartialEq + Debug>(
    grib: &str,
    json: &str,
    callback: impl Fn(&str) -> Result<T, Box<dyn Error>>,
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
        let grib = callback(&String::from_utf8(fs::read(file.path())?)?)?;
        assert_eq!(json, grib);
    }

    Ok(())
}

#[test]
fn ast_test() -> Result<(), Box<dyn std::error::Error>> {
    use ast::ast;
    use lex::lex;

    cmp_grib_json("./tests/grib", "./tests/ast", |s| {
        ast(lex(s)?).map_err(Box::from)
    })
}

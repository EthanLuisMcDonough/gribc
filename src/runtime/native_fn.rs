use runtime::values::{Callable, GribValue};
use std::io;
use std::io::Read;

macro_rules! native_package {
    ($_:ident [args] $b:block) => { $b };

    ($args:ident [$($param:ident),*] $b:block) => {
        {
            fn closure<'a>( $( $param : GribValue<'a> ),* ) -> GribValue<'a> $b

            let mut a = $args.into_iter();

            $( let $param = a.next().unwrap_or_default(); )*

            closure( $( $param ),* )
        }
    };

    ($name:ident {
        $(
            $fn_name:ident [$str:expr] ($($param:ident),*) $b:block
        )*

    }) => {
        pub enum $name {
            $( $fn_name, )*
        }

        impl $name {
            pub fn fn_name(&self) -> &'static str {
                use self::$name::*;
                match self {
                    $( $fn_name => $str ),*
                }
            }

            pub fn from_str(s: &str) -> Option<Self> {
                use self::$name::*;
                match s {
                    $( $str => Some($fn_name), )*
                    _ => None,
                }
            }
        }

        impl Callable for $name {
            fn call<'a>(&self, args: Vec<GribValue<'a>>) -> GribValue<'a> {
                use self::$name::*;
                match self {
                    $( $fn_name => { native_package!(args[$( $param ),*] $b) }, )*
                }
            }
        }

    };
}

native_package!(NativeConsolePackage {
    Println["println"](str) {
        println!("{}", str.as_str());
        GribValue::Nil
    }
    Error["error"](str) {
        eprintln!("{}", str.as_str());
        GribValue::Nil
    }
    Readline["readline"]() {
        let mut buf = String::new();
        let mut stdin = io::stdin();

        if stdin.read_to_string(&mut buf).is_err() {
            return GribValue::Nil
        }

        GribValue::String(buf)
    }
});

native_package!(NativeFmtPackage {
    ToString["toString"](obj) {
        GribValue::String(obj.to_string())
    }
    ToNumber["toNumber"](obj) {
        unimplemented!()
    }
});

use ast::node::Program;
use runtime::memory::{Gc, Runtime};
use runtime::values::{GribValue, HeapValue};
use std::collections::HashSet;
use std::io;
use std::io::Read;

macro_rules! native_obj {
    ($name:ident | $pkg:ident {
        $(
            $enum:ident -> $str:expr
        ),* $(,)*
    }) => {
        #[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
        pub enum $name {
            $( $enum ( $enum ), )*
        }

        impl $name {
            pub fn fn_name(&self) -> &'static str {
                use self::$name::*;
                match self {
                    $( $enum(e) => e.fn_name(), )*
                }
            }

            pub fn mod_name(&self) -> &'static str {
                use self::$name::*;
                match self {
                    $( $enum( _ ) => $str, )*
                }
            }

            pub fn call(&self, program: &Program, runtime: &mut Runtime, args: Vec<GribValue>)  -> GribValue {
                use self::$name::*;
                match self {
                    $( $enum(e) => e.call(program, runtime, args), )*
                }
            }
        }

        $(
            impl From<$enum> for $name {
                fn from(n: $enum) -> $name {
                    $name::$enum(n)
                }
            }
        )*

        #[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
        pub enum $pkg {
            $( $enum, )*
        }

        impl $pkg {
            const MEMBERS: &'static [&'static str] = &[$( $str ),*];

            pub fn raw_names(&self) -> &'static [&'static str] {
                match self {
                    $(
                        Self::$enum => $enum::MEMBERS,
                    )*
                }
            }

            pub fn fn_from_str(&self, s: &str) -> Option<$name> {
                match self {
                    $(
                        Self::$enum => $enum::from_str(s).map($name::$enum),
                    )*
                }
            }

            pub fn from_str(s: &str) -> Option<Self> {
                match s {
                    $($str => Some(Self::$enum),)*
                    _ => None,
                }
            }

            pub fn get_functions(&self) -> HashSet<&'static str> {
                self.raw_names().iter().map(|f| *f).collect()
            }

            //pub fn
        }
    };
}

macro_rules! native_package {
    (@branch $_:ident $rt:ident $program:ident [args] $b:block) => { $b };
    (@branch $args:ident $rt:ident $program:ident [$($param:ident),*] $b:block) => {
        {
            fn closure( $rt: &mut Runtime, $program: &Program, $( $param: GribValue ),* ) -> GribValue $b

            let mut argument_iterator = $args.into_iter();

            $( let $param = argument_iterator.next().unwrap_or_default(); )*

            closure( $rt, $program, $( $param ),* )
        }
    };

    ($name:ident [$program:ident $rt:ident] {
        $(
            $fn_name:ident [$str:expr] ($($param:ident),*) $b:block
        )*

    }) => {
        #[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
        pub enum $name {
            $( $fn_name, )*
        }

        impl $name {
            const MEMBERS: &'static [&'static str] = &[$( $str ),*];

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

            pub fn call(&self, $program: &Program, $rt: &mut Runtime, mut args: Vec<GribValue>)  -> GribValue {
                use self::$name::*;
                match self {
                    $( $fn_name => { native_package!(@branch args $rt $program [$( $param ),*] $b) }, )*
                }
            }
        }

    };
}

native_package!(NativeConsolePackage[program gc] {
    Println["println"](str) {
        println!("{}", str.as_str(program, gc));
        GribValue::Nil
    }
    Error["error"](str) {
        eprintln!("{}", str.as_str(program, gc));
        GribValue::Nil
    }
    Readline["readline"]() {
        let mut buf = String::new();
        let mut stdin = io::stdin();

        if stdin.read_to_string(&mut buf).is_err() {
            return GribValue::Nil
        }

        GribValue::String(gc.alloc_str(buf))
    }
});

native_package!(NativeFmtPackage[program runtime] {
    ToString["toString"](obj) {
        GribValue::String(obj.to_str(runtime, program))
    }
    ToNumber["toNumber"](obj) {
        GribValue::Number(obj.cast_num(program, &runtime.gc))
    }
});

native_package!(NativeMathPackage[program runtime] {
    Sin["sin"](n) { GribValue::Number(n.cast_num(program, &runtime.gc).sin()) }
    Cos["cos"](n) { GribValue::Number(n.cast_num(program, &runtime.gc).cos()) }
    Tan["tan"](n) { GribValue::Number(n.cast_num(program, &runtime.gc).tan()) }

    Asin["asin"](n) { GribValue::Number(n.cast_num(program, &runtime.gc).asin()) }
    Acos["acos"](n) { GribValue::Number(n.cast_num(program, &runtime.gc).acos()) }
    Atan["atan"](n) { GribValue::Number(n.cast_num(program, &runtime.gc).atan()) }

    Sqrt["sqrt"](n) { GribValue::Number(n.cast_num(program, &runtime.gc).sqrt()) }
    Ln["ln"](n) { GribValue::Number(n.cast_num(program, &runtime.gc).ln()) }
    Log["log"](n) { GribValue::Number(n.cast_num(program, &runtime.gc).log10()) }
    Pow["pow"](base, exp) {
        GribValue::Number(base.cast_num(program, &runtime.gc).powf(exp.cast_num(program, &runtime.gc)))
    }

    Round["round"](n) { GribValue::Number(n.cast_num(program, &runtime.gc).round()) }
    Floor["floor"](n) { GribValue::Number(n.cast_num(program, &runtime.gc).floor()) }
    Ceil["ceil"](n) { GribValue::Number(n.cast_num(program, &runtime.gc).ceil()) }
    Trunc["trunc"](n) { GribValue::Number(n.cast_num(program, &runtime.gc).trunc()) }

    MathConst["mathConst"](s) {
        use std::f64::consts::*;
        GribValue::Number(match s.as_str(program, runtime).as_ref() {
            "pi" | "PI" | "Pi" => PI,
            "e" | "E" => E,
            _ => f64::NAN,
        })
    }
});

fn get_array<'a>(arr_ref: GribValue, gc: &'a mut Gc, fn_name: &str) -> &'a mut Vec<GribValue> {
    if let Some(HeapValue::Array(ref mut arr)) =
        arr_ref.ptr().and_then(move |ptr| gc.heap_val_mut(ptr))
    {
        arr
    } else {
        eprintln!(
            "Invalid first argument supplied to array {} function",
            fn_name
        );
        panic!();
    }
}

native_package!(NativeArrayPackage[program runtime] {
    Push["push"](arr_ref, s) {
        let arr = get_array(arr_ref, &mut runtime.gc, "push");
        arr.push(s);
        GribValue::Number(arr.len() as f64)
    }
    Pop["pop"](arr_ref) {
        let arr = get_array(arr_ref, &mut runtime.gc, "pop");
        arr.pop().unwrap_or_default()
    }
    Len["len"](arr_ref) {
        let arr = get_array(arr_ref, &mut runtime.gc, "len");
        GribValue::Number(arr.len() as f64)
    }
    //RemoveAt["removeAt"](arr_ref, index) {}
});

native_obj!(NativeFunction | NativePackage {
    NativeMathPackage -> "math",
    NativeFmtPackage -> "fmt",
    NativeConsolePackage -> "console",
    NativeArrayPackage -> "array",
});

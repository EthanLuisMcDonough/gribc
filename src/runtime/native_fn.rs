use runtime::memory::Gc;
use runtime::values::{Callable, GribValue, HeapValue};
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

            pub fn call(&self, gc: &mut Gc, args: Vec<GribValue>)  -> GribValue {
                use self::$name::*;
                match self {
                    $( $enum(e) => e.call(gc, args), )*
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
    (@branch $_:ident $gc:ident [args] $b:block) => { $b };
    (@branch $args:ident $gc:ident [$($param:ident),*] $b:block) => {
        {
            fn closure( $gc: &mut Gc, $( $param: GribValue ),* ) -> GribValue $b

            let mut a = $args.into_iter();

            $( let $param = a.next().unwrap_or_default(); )*

            closure( $gc, $( $param ),* )
        }
    };

    ($name:ident [$gc:ident] {
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

            pub fn call(&self, $gc: &mut Gc, mut args: Vec<GribValue>)  -> GribValue {
                use self::$name::*;
                match self {
                    $( $fn_name => { native_package!(@branch args $gc [$( $param ),*] $b) }, )*
                }
            }
        }

    };
}

native_package!(NativeConsolePackage[gc] {
    Println["println"](str) {
        println!("{}", str.as_str(gc));
        GribValue::Nil
    }
    Error["error"](str) {
        eprintln!("{}", str.as_str(gc));
        GribValue::Nil
    }
    Readline["readline"]() {
        let mut buf = String::new();
        let mut stdin = io::stdin();

        if stdin.read_to_string(&mut buf).is_err() {
            return GribValue::Nil
        }

        gc.alloc_str(buf)
    }
});

native_package!(NativeFmtPackage[gc] {
    ToString["toString"](obj) {
        gc.alloc_str(obj.as_str(gc).into_owned())
    }
    ToNumber["toNumber"](obj) {
        GribValue::Number(obj.cast_num(gc))
    }
});

native_package!(NativeMathPackage[gc] {
    Sin["sin"](n) { GribValue::Number(n.cast_num(gc).sin()) }
    Cos["cos"](n) { GribValue::Number(n.cast_num(gc).cos()) }
    Tan["tan"](n) { GribValue::Number(n.cast_num(gc).tan()) }

    Asin["asin"](n) { GribValue::Number(n.cast_num(gc).asin()) }
    Acos["acos"](n) { GribValue::Number(n.cast_num(gc).acos()) }
    Atan["atan"](n) { GribValue::Number(n.cast_num(gc).atan()) }

    Sqrt["sqrt"](n) { GribValue::Number(n.cast_num(gc).sqrt()) }
    Ln["ln"](n) { GribValue::Number(n.cast_num(gc).ln()) }
    Log["log"](n) { GribValue::Number(n.cast_num(gc).log10()) }
    Pow["pow"](base, exp) {
        GribValue::Number(base.cast_num(gc).powf(exp.cast_num(gc)))
    }

    Round["round"](n) { GribValue::Number(n.cast_num(gc).round()) }
    Floor["floor"](n) { GribValue::Number(n.cast_num(gc).floor()) }
    Ceil["ceil"](n) { GribValue::Number(n.cast_num(gc).ceil()) }
    Trunc["trunc"](n) { GribValue::Number(n.cast_num(gc).trunc()) }

    MathConst["mathConst"](s) {
        use std::f64::consts::*;
        GribValue::Number(match s.as_str(gc).as_ref() {
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

native_package!(NativeArrayPackage[gc] {
    Push["push"](arr_ref, s) {
        let arr = get_array(arr_ref, gc, "push");
        arr.push(s);
        GribValue::Number(arr.len() as f64)
    }
    Pop["pop"](arr_ref) {
        let arr = get_array(arr_ref, gc, "pop");
        arr.pop().unwrap_or_default()
    }
    Len["len"](arr_ref) {
        let arr = get_array(arr_ref, gc, "len");
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

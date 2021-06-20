use runtime::memory::Gc;
use runtime::values::{Callable, GribValue, HeapValue};
use std::io;
use std::io::Read;

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
        #[derive(Clone)]
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
        }

        impl Callable for $name {
            fn call(&self, $gc: &mut Gc, mut args: Vec<GribValue>) -> GribValue {
                use self::$name::*;
                match self {
                    $( $fn_name => { native_package!(@branch args $gc [$( $param ),*] $b) }, )*
                }
            }        }

    };
}


native_package!(NativeConsolePackage[gc] {
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

native_package!(NativeFmtPackage[gc] {
    ToString["toString"](obj) {
        GribValue::String(obj.to_string())
    }
    ToNumber["toNumber"](obj) {
        GribValue::Number(obj.cast_num())
    }
});

native_package!(NativeMathPackage[gc] {
    Sin["sin"](n) { GribValue::Number(n.cast_num().sin()) }
    Cos["cos"](n) { GribValue::Number(n.cast_num().cos()) }
    Tan["tan"](n) { GribValue::Number(n.cast_num().tan()) }

    Asin["asin"](n) { GribValue::Number(n.cast_num().asin()) }
    Acos["acos"](n) { GribValue::Number(n.cast_num().acos()) }
    Atan["atan"](n) { GribValue::Number(n.cast_num().atan()) }

    Sqrt["sqrt"](n) { GribValue::Number(n.cast_num().sqrt()) }
    Ln["ln"](n) { GribValue::Number(n.cast_num().ln()) }
    Log["log"](n) { GribValue::Number(n.cast_num().log10()) }
    Pow["pow"](base, exp) {
        GribValue::Number(base.cast_num().powf(exp.cast_num()))
    }

    Round["round"](n) { GribValue::Number(n.cast_num().round()) }
    Floor["floor"](n) { GribValue::Number(n.cast_num().floor()) }
    Ceil["ceil"](n) { GribValue::Number(n.cast_num().ceil()) }
    Trunc["trunc"](n) { GribValue::Number(n.cast_num().trunc()) }

    MathConst["mathConst"](s) {
        use std::f64::consts::*;
        GribValue::Number(match s.as_str().as_ref() {
            "pi" | "PI" => PI,
            "e" | "E" => E,
            _ => f64::NAN,
        })
    }
});
/*
fn get_array<'a>(arr_ref: GribValue, gc: &mut Gc, fn_name: &str) -> &'a mut Vec<GribValue> {
    if let Some(HeapValue::Array(ref mut arr)) = gc.heap_val_mut(arr_ref) {
        arr
    } else {
        eprintln!("Invalid argument supplied to array {} function", fn_name);
        panic!();
    }
}

native_package!(NativeArrayPackage[gc] {
    Push["push"](arr_ref, s) {
        let arr = get_array(arr_ref, gc, "push");
        arr.push(s);
        GribValue::Number(arr.len() as f64)
    }
});
*/
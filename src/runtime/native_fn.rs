use ast::node::Program;
use runtime::memory::{Gc, Runtime};
use runtime::values::{Callable, GribKey, GribString, GribValue, HeapValue, KnownIndex};
use std::borrow::Borrow;
use std::collections::HashSet;
use std::{
    fs,
    io::{self, Read, Write},
    path::Path,
};

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
        }
    };
}

macro_rules! native_package {
    (@branch $args:ident $rt:ident $program:ident [READ_ARGS, $a:ident] $b:block) => {
        {
            fn closure( $rt: &mut Runtime, $program: &Program, $a: Vec<GribValue> ) -> GribValue $b
            closure( $rt, $program, $args )
        }
    };
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

fn print_values(values: Vec<GribValue>, program: &Program, runtime: &Runtime) {
    for val in values {
        if let GribValue::Error(err) = val {
            print!("[ERR: {}]", err.as_str(program, runtime));
        } else {
            print!("{}", val.as_str(program, runtime));
        }
    }
}

native_package!(NativeConsolePackage[program runtime] {
    Print["print"](READ_ARGS, args) {
        print_values(args, program, runtime);
        if io::stdout().flush().is_err() {
            GribValue::err("Error flushing STDOUT")
        } else {
            GribValue::Nil
        }
    }
    Println["println"](READ_ARGS, args) {
        print_values(args, program, runtime);
        println!();
        GribValue::Nil
    }
    PrintError["printError"](s) {
        if let GribValue::Error(err) = s {
            eprintln!("[ERR: {}]", err.as_str(program, runtime));
        } else {
            eprintln!("{}", s.as_str(program, runtime));
        }
        GribValue::Nil
    }
    Readline["readlineSync"]() {
        let mut buf = String::new();
        let mut stdin = io::stdin();

        if stdin.read_to_string(&mut buf).is_err() {
            return GribValue::Nil
        }

        GribValue::String(runtime.alloc_str(buf))
    }
});

native_package!(NativeFmtPackage[program runtime] {
    ToString["toString"](obj) {
        GribValue::String(obj.to_str(runtime))
    }
    ToNumber["toNumber"](obj) {
        GribValue::Number(obj.cast_num(program, &runtime.gc))
    }
    Trim["trim"](string) {
        let string = string.as_str(program, runtime);
        let trimmed = string.trim().to_string();
        GribValue::String(runtime.alloc_str(trimmed))
    }
    Lowercase["lowercase"](string) {
        let string = string.as_str(program, runtime);
        let lower = string.to_lowercase();
        GribValue::String(runtime.alloc_str(lower))
    }
    Uppercase["uppercase"](string) {
        let string = string.as_str(program, runtime);
        let upper = string.to_uppercase();
        GribValue::String(runtime.alloc_str(upper))
    }
});

fn try_hash_key(
    hash: GribValue,
    key: GribValue,
    runtime: &mut Runtime,
    program: &Program,
) -> Option<GribKey> {
    let key_str = key.to_str(runtime);
    runtime
        .gc
        .try_get_hash(hash)
        .map(|hash| hash.key(key_str, program, &runtime.gc))
}

native_package!(NativeHashPackage[program runtime] {
    DeleteKey["deleteKey"](hash, key) {
        GribValue::Bool(try_hash_key(hash.clone(), key, runtime, program)
            .and_then(|key| {
                runtime.gc.try_get_hash_mut(hash)
                    .map(|hash| hash.delete_key(&key))
            }).is_some())
    }
    HashMutable["hashMutable"](hash) {
        GribValue::Bool(runtime
            .gc
            .try_get_hash(hash)
            .map(|hash| hash.is_mutable())
            .unwrap_or(false))
    }
    HasKey["hasKey"](hash, key) {
        let key_str = key.to_str(runtime);
        GribValue::Bool(runtime
            .gc
            .try_get_hash(hash)
            .map(|hash| {
                let key = hash.key(key_str, program, &runtime.gc);
                hash.get_property(&key).is_some()
            }).unwrap_or(false))
    }
    Keys["keys"](hash_val) {
        runtime.gc.try_get_hash(hash_val)
            .map(|hash| hash.keys())
            .map(HeapValue::Array)
            .map(|keys| runtime.alloc_heap(keys))
            .map(GribValue::HeapValue)
            .unwrap_or_default()
    }
});

native_package!(NativeErrPackage[_program _runtime] {
    Err["err"](obj) {
        if obj.is_err() { obj } else { GribValue::Error(obj.into()) }
    }
    IsErr["isErr"](obj) {
        GribValue::Bool(obj.is_err())
    }
    ErrVal["errVal"](obj) {
        if let GribValue::Error(val) = obj {
            *val
        } else {
            GribValue::Nil
        }
    }
});

native_package!(NativeMetaPackage[_program runtime] {
    TypeOf["typeOf"](val) {
        use self::GribValue::*;
        String(GribString::Static(match val {
            Bool(_) => "boolean",
            String(_) => "string",
            Nil => "nil",
            Number(_) => "number",
            Callable(_) => "callable",
            ModuleObject(_) => "module object",
            Error(_) => "error",
            HeapValue(ptr) => match runtime.gc.typed_index(ptr) {
                Some(KnownIndex::Array(_)) => "array",
                Some(KnownIndex::Hash(_)) => "hash",
                _ => "heap object",
            },
        }))
    }
    ClearGc["clearGc"]() {
        runtime.clean();
        GribValue::Nil
    }
    BindFn["bindFn"](fnc_val, target) {
        let mut fnc = fnc_val;
        if let GribValue::Callable(Callable::Lambda { binding, .. }) = &mut fnc {
            *binding = target.ptr();
        }
        fnc
    }
});

native_package!(NativeStrPackage[program runtime] {
    Split["split"](content, delim) {
        let delim = delim.as_str(program, runtime);
        let str_arr = content.as_str(program, runtime)
            .split(delim.as_ref())
            .map(|s| s.to_string())
            .collect::<Vec<String>>();
        let grib_arr = str_arr.into_iter()
            .map(|s| runtime.alloc_str(s))
            .map(GribValue::String)
            .collect::<Vec<_>>();
        let ptr = runtime.alloc_heap(HeapValue::Array(grib_arr));
        GribValue::HeapValue(ptr)
    }
    Strlen["strlen"](obj) {
        let string = obj.as_str(program, runtime);
        GribValue::Number(string.len() as f64)
    }
    Substr["substr"](string, start_val, end_val) {
        let string = string.as_str(program, &runtime);
        let l = string.len() as i64;

        let mut start = start_val.cast_num(program, &runtime.gc) as i64;
        let mut end = end_val.cast_num(program, &runtime.gc) as i64;

        if start < 0 { start = l - start; }
        if end < 0 { end = l - end; }

        start = start.clamp(0, l);
        end = if end_val.is_nil() { l } else { end.clamp(0, l) };

        let top = start.max(end) as usize;
        let bottom = start.min(end) as usize;

        let new_str = string[bottom..top].to_string();
        runtime.alloc_str(new_str).into()
    }
    IndexOfStr["indexOf"](string, search) {
        let string = string.as_str(program, runtime);
        let search = search.as_str(program, runtime);

        GribValue::Number(string.find(search.as_ref())
            .map(|ind| ind as f64).unwrap_or(-1.0))
    }
    Replace["replace"](string, find, replacement) {
        let string = string.as_str(program, runtime);
        let find = find.as_str(program, runtime);
        let replacement = replacement.as_str(program, runtime);
        let rf: &str = find.borrow();
        let result = string.replace(rf, &replacement);
        GribValue::String(runtime.alloc_str(result))
    }
});

macro_rules! guard {
    ($e:expr, $s:expr) => {
        match $e {
            Ok(val) => val,
            Err(_) => return GribValue::err($s),
        }
    };
}

fn write_bytes(path: &Path, contents: &[u8], append: bool) -> GribValue {
    if let Some(parent) = path.parent() {
        guard!(
            fs::create_dir_all(parent),
            "Could not create subdirectories"
        );
    }

    let mut file = guard!(
        fs::OpenOptions::new()
            .write(true)
            .create(true)
            .append(append)
            .open(path),
        "Failed to open file provided"
    );

    file.write(contents)
        .map(|_| GribValue::Nil)
        .unwrap_or(GribValue::err("Failed to write to file"))
}

native_package!(NativeSyncIoPackage[program runtime] {
    ReadText["readText"](obj) {
        let path_str = obj.as_str(program, runtime).into_owned();
        let path = Path::new(&path_str);

        fs::read_to_string(path)
            .map(|s| runtime.alloc_str(s))
            .map(GribValue::String)
            .unwrap_or(GribValue::err("Error reading provided file provided to readText"))
    }
    WriteText["writeText"](path_val, contents_val, append) {
        let path_str = path_val.as_str(program, runtime).into_owned();
        let path = Path::new(&path_str);

        let append = append.truthy(program, &runtime.gc);
        let contents = contents_val.as_str(program, runtime);

        write_bytes(&path, contents.as_bytes(), append)
    }
    ReadBytes["readBytes"](obj) {
        let path_str = obj.as_str(program, runtime).into_owned();
        let path = Path::new(&path_str);

        fs::read(path)
            .map(|arr| {
                arr.into_iter().map(|byte| byte as f64).map(GribValue::Number).collect::<Vec<_>>()
            })
            .map(HeapValue::Array)
            .map(|arr| runtime.alloc_heap(arr))
            .map(GribValue::HeapValue)
            .unwrap_or(GribValue::err("Error reading provided file provided to readText"))
    }
    WriteBytes["writeBytes"](path_val, contents_val, append) {
        let path_str = path_val.as_str(program, runtime).into_owned();
        let path = Path::new(&path_str);
        let append = append.truthy(program, &runtime.gc);

        runtime.gc.try_get_array(contents_val).map(|contents| {
            let converted = contents.iter().map(|val| {
                val.cast_ind(program, &runtime.gc).unwrap_or(0).min(255) as u8
            }).collect::<Vec<u8>>();

            write_bytes(&path, &converted[..], append)
        }).unwrap_or_default()
    }
    IsFile["isFile"](path_val) {
        let path_str = path_val.as_str(program, runtime).into_owned();
        let path = Path::new(&path_str);
        GribValue::Bool(path.is_file())
    }
    IsDir["isDir"](path_val) {
        let path_str = path_val.as_str(program, runtime).into_owned();
        let path = Path::new(&path_str);
        GribValue::Bool(path.is_dir())
    }
    PathContents["pathContents"](path_val) {
        let path_str = path_val.as_str(program, runtime).into_owned();
        let path = Path::new(&path_str);
        GribValue::Bool(path.is_dir())
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

    Min["min"](READ_ARGS, args) {
        let mut smallest = f64::INFINITY;
        for val in args {
            let n = val.cast_num(program, &runtime.gc);
            smallest = n.min(smallest);
        }
        GribValue::Number(smallest)
    }
    Max["max"](READ_ARGS, args) {
        let mut largest = -f64::INFINITY;
        for val in args {
            let n = val.cast_num(program, &runtime.gc);
            largest = n.max(largest);
        }
        GribValue::Number(largest)
    }

    Random["random"]() {
        GribValue::Number(rand::random())
    }

    MathConst["mathConst"](s) {
        use std::f64::consts::*;
        GribValue::Number(match s.as_str(program, runtime).as_ref() {
            "pi" | "PI" | "Pi" => PI,
            "e" | "E" => E,
            _ => f64::NAN,
        })
    }
});

const NO_ARRAY: &'static str = "Functon provided non-array value";
native_package!(NativeArrayPackage[program runtime] {
    Push["push"](arr_ref, s) {
        if let Some(arr) = runtime.gc.try_get_array_mut(arr_ref) {
            arr.push(s);
            GribValue::Number(arr.len() as f64)
        } else {
            GribValue::err(NO_ARRAY)
        }
    }
    Pop["pop"](arr_ref) {
        if let Some(arr) = runtime.gc.try_get_array_mut(arr_ref) {
            arr.pop().unwrap_or_default()
        } else {
            GribValue::err(NO_ARRAY)
        }
    }
    ArrLen["arrlen"](arr_ref) {
        if let Some(arr) = runtime.gc.try_get_array(arr_ref) {
            GribValue::Number(arr.len() as f64)
        } else {
            GribValue::err(NO_ARRAY)
        }
    }
    RemoveAt["removeAt"](arr_ref, index) {
        let ind = index.cast_ind(program, &runtime.gc);
        if let Some(arr) = runtime.gc.try_get_array_mut(arr_ref) {
            if let Some(i) = ind.filter(|i| i < &arr.len()) {
                arr.remove(i)
            } else {
                GribValue::Nil
            }
        } else {
            GribValue::err(NO_ARRAY)
        }
    }
    InsertAt["insertAt"](arr_ref, index, value) {
        let ind = index.cast_ind(program, &runtime.gc);
        if let Some(arr) = runtime.gc.try_get_array_mut(arr_ref) {
            if let Some(i) = ind.filter(|i| i < &arr.len()) {
                arr.insert(i, value);
            }
            GribValue::Nil
        } else {
            GribValue::err(NO_ARRAY)
        }
    }
    CopyArr["copyArr"](arr) {
        if let Some(arr) = runtime.gc.try_get_array(arr).cloned() {
            runtime.alloc_heap(HeapValue::Array(arr)).into()
        } else {
            GribValue::err(NO_ARRAY)
        }
    }
    Append["append"](target, source) {
        let source_op = runtime.gc.try_get_array(source).cloned();
        let target_op = runtime.gc.try_get_array_mut(target.clone());
        if let (Some(mut source), Some(target_arr)) = (source_op, target_op) {
            target_arr.append(&mut source);
            target
        } else {
            GribValue::err(NO_ARRAY)
        }
    }
    Concat["concat"](READ_ARGS, args) {
        let mut new_arr = Vec::new();
        for arr_ref in args {
            let arr_op = runtime.gc.try_get_array(arr_ref).cloned();
            if let Some(mut array) = arr_op {
                new_arr.append(&mut array);
            } else {
                return GribValue::err(NO_ARRAY);
            }
        }
        runtime.alloc_heap(HeapValue::Array(new_arr)).into()
    }
    Slice["slice"](arr, start_val, end_val) {
        let mut start = start_val.cast_num(program, &runtime.gc) as i64;
        let mut end = end_val.cast_num(program, &runtime.gc) as i64;

        if let Some(arr) = runtime.gc.try_get_array(arr).cloned() {
            let l = arr.len() as i64;
            start = start.clamp(0, l);
            end = if end_val.is_nil() { l }
                else { end.clamp(0, l) };

            let top = start.max(end) as usize;
            let bottom = start.min(end) as usize;

            let new_arr = arr[bottom..top].to_vec();
            let ptr = runtime.alloc_heap(HeapValue::Array(new_arr));

            GribValue::HeapValue(ptr)
        } else {
            GribValue::err(NO_ARRAY)
        }
    }
});

native_obj!(NativeFunction | NativePackage {
    NativeMathPackage -> "math",
    NativeFmtPackage -> "fmt",
    NativeConsolePackage -> "console",
    NativeArrayPackage -> "array",
    NativeErrPackage -> "err",
    NativeSyncIoPackage -> "syncio",
    NativeStrPackage -> "str",
    NativeHashPackage -> "hash",
});

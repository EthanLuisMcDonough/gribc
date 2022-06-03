

## Intro to Grib
### Comments
Anything behind the `@` symbol is a comment. Comments are portions of code that are ignored by the interpreter:
```
@ I’m a comment
2 + 3 + 4; @ Everything left of "@" is NOT a comment
```
### Expressions
Expressions are portions of code that yield a result. For example, `abc[i].age + 3`, `myFunction(3)`, and `["Hello"]` are all expressions. They are instructions that the interpreter follows to generate values. In Grib, expressions consist of operations (like addition and multiplication), function calls, variable references, and literal values (numbers, lambdas, hashes, arrays, etc).
### Variables
A variable is a named slot programmers can use to store values. Variable names are case sensitive, alphanumeric, and must start with a letter. Mutable variables are defined with `decl` (short for “declaration”) keyword and immutable variables are defined with `im` (short for “immutable”). Declarations require a semicolon at the end.
```
im unchangeable = 23; @ This variable cannot be reassigned
decl count = 2; @ This variable can
```
**Note**: mutable objects (i.e. mutable hashes or arrays) assigned to immutable variables can still be altered. The variable value itself still cannot be reassigned.

Multiple variables can be declared and initialized in one declaration:
```
decl a = 1, b = 2;
```
All immutable variables must be initialized with an expression. Uninitialized mutable variables are set to nil by default.

Grib is a block scoped language. A scope is defined by a group of curly brackets, and all variables in one scope are visible to nested scopes. A variable cannot be defined if it shares a name with another variable defined in the same scope.
```
im a = 1, b = 2;
{ @ New scope
	decl a = 2;
	im c = 3;
	@ a is mutable here because the mutable
	@ definition is being used in this scope
	one = 1;
	@ a, b and c are all visible in this scope
}
@ c is no longer visible out there
@ Likewise, a is not mutable in this scope
```
Grib is a dynamically typed language. This means that a variable’s data type is determined while the program is running. Variables can be initialized with and reassigned to values of any type.

Grib programs are checked before they are run. If a nonexistent variable name is referenced or an immutable value is reassigned, the program will simply not run.
### Data types
**Numbers**: Grib treats all numbers as floating point (rational numbers), meaning there are no data types limited to only integers.
```
55 @ whole number
0.4 @ just decimal
3.923
```
Grib numbers have two special values: `Infinity` and `NaN` (Not a Number). You can get them by performing special math operations:
```
4 / 0 @ Infinity
sqrt(-4) @ NaN
```
_Note:_ Grib’s numbers are made up of 64 bits and adhere to the [IEEE-754](https://standards.ieee.org/ieee/754/4211/) 2008 floating point specification.

**Booleans**: Booleans are binary values that can be either true or false. They’re used to represent the result of a comparison.
```
3 > 4 @ this comparison yields false
false @ the false value as a literal value
true @ true as a literal value
```
**Strings**: Strings are representations of text. Every character between a set of quotes (the " character) is included in the string. Strings can be used to store user input and display text output.
```
"words wrapped in quotes"
```
In Grib strings, `\` is called the escape character. The escape character can be used alongside other characters to represent special characters inside the string:
```
"Quotes can be \"used\" inside strings if you prefix them with a slash"
```
| Character | Escape combo |
|--|--|
| Newline (enter) | \n |
| Tab | \t |
| " (Quote) | \" |
| \ (Slash) | \\\\ |

**Arrays**: Arrays are ordered lists of data.
```
im arr = [4, "my string", ["nested array", 2]];
```
Array values can be accessed using the index access notation: `item[index]`. Indices start at zero and span up to one minus the array’s length. For example, the above array’s second value, `"my string"`, can be accessed using `arr[1]`. Likewise, the first element of the inner array can be accessed with `arr[2][0]`.

Hashes: In other languages, these are sometimes called “records”, “objects”, “maps”, “associative arrays”, or “dictionaries”. Hashes are a collection of unique keys that are each associated with a value. Immutable hashes are defined using `# {...}` syntax and mutable hashes are created using `$ {...}` syntax.
```
# {
	description: "collection of values",
	"string key": nil,
	nested: $ {
		sub: true,
		count: 1,
		message: "this *nested* hash is mutable"
	}, @ trailing commas are optional
}
```
The right hand key values can be written either raw strings or identifiers. You can access items in a hash using a property access notation: `myHash.keyName`. Values can also be accessed via index access notation: `myHash["keyName"]`.

**Callables**: Callables are values that can be called with function notation: `function(val1, val2...)`. These values include function references, auto-properties, and lambdas. These concepts will be elaborated on in the functions section.

**Nil**: Grib’s `nil` is a special value meant to represent nothing. It is a value returned when a nonexistent property is accessed, a function has no return value, or a non-callable value is called.

**Module objects**: Module objects are immutable hash-like objects that store an imported module’s functions. This data type will be covered in the modules section.

**Errors**: Error objects are special objects that can be created and detected using functions in the native err module. These values are returned from functions to represent that an error has occurred.
```
im e = err("my message"); @ create an error
isErr(e) @ check if value is an error
errVal(e) @ get value stored in the error
```
### Control Flow
Loops and conditionals allow code to be executed only when certain conditions are met. `if`/`else if`/`else` statements can control whether a block of code is run:
```
if 3 > 4 { ... }
else if 5 > 4 { ... }
else { ... }
```
If statements are defined by a conditional keyword followed by an expression. Else if blocks are only run if all previous if and else if blocks do not run. Similarly, `else` clauses only execute if none of the previous blocks have. An `if` statement must have at least one `if` block, as many `else if` blocks as it needs , and at most one else block at the end.

Loops are blocks of code that are run for as long as their condition holds true:
```
while i < 4 { ... } @ will execute for as long as i is less than 4
for decl i = 0; i < 4; i = i + 1 { ... }
```
While loops are simple in that they only consist of an expression and a body. The loop checks the condition to decide whether to execute the block. It continues doing this until it finds that the expression is false. For loops consist of one declaration, two expressions, and a body. Variables defined in the declaration are not accessible outside the loop. The first expression is the condition and the second expression is run after each time the block is executed. The for loop in the second example loops through number 0 to 3.
### Statements

Statements are like expressions that do not yield values. Statements include loops, conditional blocks, imports, function definitions, imports, returns, and declarations. Grib files can only contain statements. Expressions are found inside statements, and they can be evaluated as statements if they are followed by a semicolon.
```
im str = "my message"; @ declaration statement
if strlen(str) > 3 { ... } @ control flow statement
1 + 2; @ “naked” expression statement
1 + 2 @ Error! Semicolon required!
```
### Operators
Much like operators in a math expression, Grib operators have precedence. Here is the list of Grib’s operators from highest to lowest precedence.
| Type | Operators |
|--|--|
| Unary | `~`, `!`, `-` |
| Multiplication/Division | `/`, `*`, `%` |
| Addition/Subtraction | `+`, `-` |
| Comparison | `>`, `<`, `<=`, `>=`, `==`, `!=` |
| Logical AND | `&&` |
| Logical OR | `\|\|` |
| Assignment | `=`, `+=`, `-=`, `*=`, `/=`, `%=` |

Because Grib is loosely typed, operators can be used on values that don't make sense.  For example, `#{ } + 23` is a valid expression, but it evaluates to `NaN` because the hash is converted into `NaN`.   This process of automatic type conversion is called type coercion.  Here's a table describing how different binary operators behave with operands of different types: 

| First | Operator | Second |  | Result |
|--|--|--|--|--|
| Number | `+`, `-`, `*`, `/`, `%`, `>(=)`, `<(=)`, `+=`, `-=`, `*=`, `/=`, `%=` | Anything | = | Second value will be coerced into a number. |
| Anything | `+`, `-`, `*`, `/`, `%`, `>(=)`, `<(=)` | Number | = | The first value will be coerced into a number. |
| String | `+`, `+=` | Anything | = | Second value will be converted into a string. |
| String | `*`, `*=` | Anything | = | Second value will be converted into a whole number >= 0.  The string will be repeated that many times if possible. |
| Array | `+` | Anything | = | A new array with the second value added to the end will be returned. |
| Array | `+=` | Anything | = | The right value will be pushed into the array on the left hand side.  Said array will be returned by the expression. |
| Array | `*` | Anything | = | Second value will be converted into a whole number >= 0.  A new array will be created containing elements in the left-hand array repeated N times (shallow copy) |
| Array | `*=` | Anything | = | Same as previous, but a new array will not be created.  Repeating elements will be appeneded to the end of the existing array.  If the array is multiplied by zero, the array will be emptied. |
| Error | any | Anything | = | The left-hand error will be returned |
| Anything but an error | any | Error | = | The right-hand error will be returned |

In most cases, values will be converted to numbers when the binary operators `+`, `-`, `*`, `/`, `%`, `>(=)`, or `<(=)` are used.

`~` and unary `-` are both unary digit negation.  They operate in the exact same way and coerce their operand into a number.  `!` is the logical negation operator.  It coerces the value into a boolean and returns that boolean's logical opposite.

`&&` and `||` don't coerce values, but they do test whether values are "truthy".  A value that isn't zero, `nil`, an error, an empty string, or `false` is truthy.  `&&` returns either the first false value or the last truthy value if both are truthy.  Likewise, `||` returns the first truthy value or the second false value if both operands are false.

`!=`, and `==` never coerce values.  If two values don't have matching types, they aren't equal.
### Functions
Functions (also known as procedures) are self-contained code blocks that can be fed values through variables called parameters. Parameters are located between the two pipes. Functions are used to modularize and reuse code. They are made up of statements and can return a value when called by using the return keyword:
```
@ “add” is the name of our function
proc add |firstNum secondNum| {
	if firstNum > secondNum {
		congratulate(firstNum);  
	}
	return firstNum + secondNum; @ Return their sum
}

@ function without parameters or return value
proc newline || { print("\n"); }

@ Function referenced in other function
proc congratulate |value| {
	print(value + " is greatest!");
	newline();  
}

@ Calls the add function supplying 3 and 2 as parameters
@ Then, feed the returned value into the println function to display it
println(add(3, 2)); @ prints out “3” is the best and then prints 5
```
Unlike other blocked statements, functions do not capture outer scope. Functions can call other functions (as well as themselves), but variables outside of the function are not accessible inside the function. Non-function values are passed into the function through parameters. Parameters are given values from left to right, so if fewer values than expected are supplied to a function, the undefined variables on the right are set to `nil`. All parameter variables are mutable and visible only inside the function.
### Lambdas
Think of lambdas as local callable values. Unlike functions, they are able to capture outer scope and do not require an explicit return to yield a value.
```
decl outer = 2; @ variable defined in outer scope  
im fig1 = lam |param1 param2| {  
	im local = 23;  
	@ explicit return  
	@ this is the value that will be returned  
	@ when the lambda is called  
	return local * param1 + param2 * outer;
};

decl fig2 = lam |p1| { p1 * 3 }; @ implicit return lambda.
@ The semicolon-less expression is evaluated and the result is returned

im fig3 = lam || { 23 }; @ lambda without any params. Returns 23
```
These callables can be called exactly how one would call a regular function:
```
fig1(0, 1); @ evaluates to 2  
local = 3;  
fig1(0, 1); @ evaluates to 3 because outer is set to 3 now
```
*Note:* lambdas are treated as any other expression value and cannot be exported.

When defined in hashes, lambdas bind the this keyword to the current hash:
```
im person = # {
	age: 13,
	@ this refers to the “person” hash
	print: lam || { println(this.age)  } @ prints 13
};
```
If a value that isn't a lambda or function is invoked, the expression will return an error.  The arguments will still be evaluated.
### Auto-properties
Auto-properties are special lambdas that can only be defined inside hashes. They’re like lambdas, but they are executed when their property is accessed or assigned. In other languages, they’re called “getters” and “setters”. Here’s an example of how one might implement a Vector object:
```
im vector = $ {
	x: 1, y: 2, z: 3,
	len { @ note the lack of colon
		@ the getters and setters have bodies and params
		@ just like lambdas
		get  ||  { sqrt(this.x * this.y * this.z) },
		set |newLen| {
			@ calls the lambda defined in get
			im old = this.len; @ calls the lambda defined in get  
			this.x = (this.x / old) * newLen;
			this.y = (this.y / old) * newLen;
			this.z = (this.z / old) * newLen;
		}
	}
};  
println(vector.len); @ outputs 2.449

@ changing a value would automatically cause the value returned by len to change as well
```
Getters require no parameters but should return a value. Setters require exactly one parameter. Auto-properties can also capture mutable variables in scope:
```
decl age = 13;
im person = # {
	age {
		@ both values are bound to the “age” variable currently in scope
		@ This binding will persist even outside the scope, 
		@ the variable will be kept alive by the garbage collector
		get age,
		set age
	}
};

person.age = 21;  
println(age == person.age); @ prints true
```
Note that we can mutate the age property even though the hash is defined as immutable. We aren’t changing any values defined inside the hash, we’re only using the hash as a proxy for that mutable variable.
### Modules
Modules can be imported using `import ... from "module";` syntax:
```
@ import function1 and function2 from a local module file  
import |function1 function2| from  "./local_module.grib";

@ import all functions from Grib’s native math package  
import * from "math";

@ Create module object containing all functions in native syncio package
import moduleObj from "syncio";
```
Individual functions can be imported from a module by listing them between pipes. All of the functions in a module can be imported using the wildcard (`*`) import symbol. If the token between `import` and `from` is just a variable name, that variable will be set to a module object. Module objects behave identically to immutable hashes. Each property in a module object corresponds with a function defined in its module. For example, `moduleObj.readFile` would point to the native `readFile` function in the `syncio` package.

Imports have to be made at the beginning of a Grib file. Variables defined by an import statement are immutable, but you can "hide" them with new declarations:
```
import |cos sin| from "math";

decl cos = 2;  
cos = 3; @ cos is now mutable, but sin isn’t
```
Imported module files can only contain functions. Function definitions that are not prefixed with the public keyword cannot be imported.
```
@ Current file: module.grib  
public proc getMax |first second| {
	if first > second {
		return first;  
	} else {
		return second;
	}
}

proc private || {
	println("I can only be called inside module.grib");
}
```
### Idiomatic Grib
Grib is a procedural programming language that puts an emphasis on encapsulation.  Putting together what we know, we can rewrite the previous vector example in a more idiomatic fashion:
```
public proc newVec |x y z| {
	return # {
		x { get x, set x },
		y { get y, set y },
		z { get z, set z },
		len {
			get || { sqrt(x + y + z) },
			set |newLen| {
				im old = this.len;
				x = (x / old) * newLen;
				y = (y / old) * newLen;
				z = (z / old) * newLen;
			}
		},
		unit: lam || {
			im old = this.len;
			return newVec(x / old, y / old, z / old);
		},
		scale: lam |scalar| {
			x *= scalar;
			y *= scalar;
			z *= scalar;
		}
	};
}
```

## Standard Library Reference

The functions in each package can be imported by replacing the filename in an import statement with a native package name. If any of the functions listed in this directory are provided invalid parameters, they will attempt to coerce those values into the correct types. If this is impossible, an error is returned instead of the normal value.

*Note:* Although Grib is dynamically typed, each function will be listed in a manner that explicitly declares its return and parameter types.

### `"syncio"`

Syncio is a package for synchronously performing IO operations. All of the functions in this package return an error with the message starting with "IO:" if they fail.
| Function | Description |
|--|--|
| `newSocket(ip: string): Socket` | Creates a synchronous TCP connection to the given IP address. The returned value is an immutable hash containing the methods `close()`, `write(byteArray)`, and `read()`. close closes the connection, `write` attempts to write an array of numbers (that will be coerced into bytes) to the connection, and `read` attempts to read an array of bytes from the connection. |
| `readFile(path: string): string` | Reads a file front to back and returns the file contents as a string. |
| `writeFile(path: string, fileContents: string, append: boolean)` | Writes a string to the given file. The file is grated if it does not yet exist. The contents are appended if the append flag is set to true. Otherwise, any existing file contents are overwritten. |
| `pathContents(path: string): array` | Returns an array containing filenames and folders that are inside the given directory. |
| `isFile(path: string): boolean` | Checks if the provided path points to a file. |
| `isDirectory(path: string): boolean` | Checks if the provided path points to a directory. |

### `"array"`
Functions related to interacting with arrays.
| Function | Description |
|--|--|
| `push(a: array, v: any): number` | Pushes a value into the provided array and returns the number of elements in the array after the operation. This function alters the array passed into the function. |
| `pop(a: array): any` | Removes the last item in an array and returns the removed value. |
| `arrlen(a: array): number` | Returns the number of elements in an array. |
| `copyArr(a: array): array` | Creates a shallow copy of the array (the array itself is cloned, but the elements inside the array are not). |
| `removeAt(a: array, index: number): any` | Removes an item from the array at the given index. The removed item is returned. |
| `slice(a: array, start: number, end: number): array` | Returns a copy of the given array that spans from the first index to the end index. Allows the programmer to get a “slice” of the given array. |
| `concat(a: array, b: array): array` | Returns a copy of `a` with all elements in `b` appended to the end of it. |
| `append(a: array, b: array): array` | Adds all elements in array `b` to the end of array `a`. |

### `"hash"`
Functions related to hashes. These functions also work on module objects.
| Function | Description |
|--|--|
| `keys(h: hash): array` | Returns an array of the given hash’s keys. |
| `hashMutable(h: hash): boolean` | Returns `true` if the hash is mutable and false otherwise. |
| `hasKey(h: hash, key: string): boolean` | Check if the given hash has a key. This differs from checking whether a hash’s key is nil when a hash property is explicitly set to `nil`. |
| `deleteKey(a: hash, key: string): boolean` | Removes the given key from the hash. This function returns true if the key was successfully removed. It returns false when the given hash is immutable. |

### `"console"`
Functions for reading from and writing to the console.
| Function | Description |
|--|--|
| `print(v: any)` | Prints the given value to the standard output (console). |
| `println(v: any)` | Prints the value to the console followed by a newline. |
| `printError(v: any)` | Prints the given value to the standard error output (STDERR). |
| `readLineSync(a: array): string` | Reads in a line from the standard input (STDIN). This function halts the program until a line can be read. |

### `"err"`
Functions for creating and reading error objects.
| Function | Description |
|--|--|
| `err(message: string)` | Creates an error with the provided message. |
| `isErr(v: any): boolean` | Checks if the provided value is an error. |

### `"str"`
| Function | Description |
|--|--|
| `split(str: string, separator: string): array` | Takes `str` and splits it into an array of substrings that were separated by `separator`. |
| `indexOf(str: string, sub: string): number` | Looks for the position of a given substring inside a larger string. -1 will be returned if the substring is not found. |
| `strlen(str: string): number` | Returns the number of characters in a string. |

### `"meta"`
| Function | Description |
|--|--|
| `typeOf(v: any): string` | Returns the data type of the provided value as a string. This value can be `"string"`, `"array"`, `"hash"`, `"error"`, `"callable"`, `"number"`, `"boolean"`, or `"module object"`. |
| `clearGc()` | Halts the program to clean out the garbage collector. The garbage collector runs automatically, but this function allows the programmer more control over it. |

### `"math"`
| Function | Description |
|--|--|
| `sin(n: number): number` | Returns the sine of the number in radian mode. |
| `cos(n: number): number` | Returns the cosine of the given number in radian mode. |
| `tan(n: number): number` | Returns the tangent of the given number in radian mode. |
| `ln(n: number): number` | Returns the natural logarithm of the given number. |
| `pow(base: number, exponent: number): number` | Raises base to the power of exponent. |
| `round(n: number): number` | Rounds the given number to the nearest integer. |
| `floor(n: number): number` | Rounds a number down. |
| `ceil(n: number): number` | Rounds a number up. |
| `trunc(n: number): number` | Strips a number of its exponent portion. |
| `random(): number` | Returns a random number between 0 and 1. |

### "fmt"
| Function | Description |
|--|--|
| `toNumber(n: string): number` | Attempts to parse the given string to a number. |
| `toString(v: any): string` | Converts the given value to a string. |

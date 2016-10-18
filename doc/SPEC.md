# Scarab 0.3 Language Specification

This document describes the syntax, runtime model and core library of Scarab, a dynamic Lisp-family
language with a fluent and extensible syntax.

## Syntax

A parsed Scarab program or expression is a list, containing values and possibly other lists. All
Scarab source is encoded in UTF-8.

### Tokens

These tokens are either described qualitatively, or with regular expressions ignoring whitespace.

Except for newlines between elements of a `form-list`, whitespace only separates tokens and has no syntactic relevance.

`COMMENT`
: `# .* \n`  
May appear anywhere within a line (though is of course ignored within strings).

`ALPHA`
: any Unicode alphabetical character

`PUNCT`
: any Unicode punctuation character, or `_` or `-`, not including any of the following: `#"',{}()[]`

`DIGIT`
: any Unicode numeric character

`IDENTIFIER`
: `(ALPHA | PUNCT) (ALPHA | PUNCT | DIGIT)*`

`NUMBER`
: `-? [0-9]+ (.[0-9]+)?`

`STRING-CONTENT`
: `\[nrt"'\\n] | [^\n"]`  
Escaped newlines within strings are included in the tokenized result, but any whitespace at the
beginning of the following line is not.

`BACKQUOTE-STRING-CONTENT`
: `` \` | [^`] ``
: Any escaped characters besides `` ` `` are preserved; `` `blah\n \\ \` blah` `` tokenizes the same as
`` "blah \\n \\\\ ` blah" ``. Also, backquoted strings can span newlines.

`STRING`
: ``" STRING-CONTENT* " | ` BACKQUOTE-STRING-CONTENT* ` ``

### Structure

These syntactic elements are described using ABNF. A valid Scarab program consists of a `form-list`,
while a Scarab expression consists of a 'form'.

`form-list`
: `form? ([\n,] form?)*  
Commas or newlines are required between elements, but excess separators are ignored.

`form`
: `value+`

`value`
: `IDENTIFIER | NUMBER | STRING | ' value | list`

`list`
: `{ form-list } | ( form ) | [ expression-list ]`

`expression-list`
: `value identifier value (identifier value)*`  
The `identifier` between all elements of a given expression list must be the same (see below).

### Syntactic Transformations

Both `form-list`s and `expression-list`s are transformed into normal lists after being parsed.

A `form-list` is parsed into a list containing lists for each one of the forms inside it.

Before: `{a b, c d}`
After: `((a b) (c d))`

Before: `{a b, c { e, f g } }`
After: `((a b) (c ((e) (f g))))`

An `expression-list` is parsed into a single list, with the operator between elements removed and
prepended once to the front.

Before: `[2 + a + b]`
After: `(+ 2 a b)`


### Valid Examples

~~~
print "Hello, World!"
print (+ 2 2)
print [2 + 2], print [4 * a * b]
if [a = b] {
	print `what's up
dog`
}
~~~

### Invalid Examples

~~~
set 2meirl4meirl "quickly dated reference" # Identifiers cannot begin with a number
set y [4 * x + 2] # Expressions cannot contain multiple operators
~~~

## Runtime Model

### Values and Types

Scarab is a dynamically typed language, with a small set of built-in types:

* `boolean`
* `function`
* `integer`
* `list`
* `nil`: nothing.
* `quoted`: a wrapped value.
* `real`: floating point number.
* `record-type`: a defined set of keys.
* `record`: a set of key-value pairs, conforming to a particular `record-type`.
* `string`: a set of bytes.
* `symbol`: an identifier.

Three of these types (`list`, `quoted` and `symbol`) are special: they evaluate to something besides
themselves. Every other kind of value is by exclusion called an **atom**.

#### `boolean`

There are two values of this type, known as `true` and `false`.

#### `function`

Either a Scarab function, containing a name, argument list, form list and closure scope, or a native
function.

Example: `function name (arg1 arg2 ...) {form 1, form 2, ...}`

#### `integer`

A signed integer of the largest supported precision (usually 64 bits).

Example: `49876`

#### `list`

A read-only list of values. Unlike many Lisps, this is not exposed as a linked list of cons cells.
However, an empty `list` is still equivalent to `nil`.

When evaluated, is run as a form.

Example: `(a 1 "abcd")`

#### `nil`

Equivalent to null in many other languages. There is only one value of this type, called `nil`.

#### `quoted`

A wrapper for another value.

When evaluated, returns that value.

Example: `'abc` (evaluates to the symbol `abc`).

#### `real`

A floating-point value of the largest supported precision (usually 64 bits).

Example: `469.43`

#### `record-type`

A collection of keys, defining a particular record structure, similar to a `struct` type in C or a
class in many languages (though without intrinsic methods, inheritance, or any other OOP
functionality).

Example: `record-type rectangle (width height)`

#### `record`

A particular instance of an already-defined `record-type`.

Example: `record rectangle 16 9`

#### `string`

A set of bytes. Is length-limited, not null-terminated.

Example: `"creepies and crawlies"`

#### `symbol`

An identifier. Every symbol is an interned singleton; a symbol is the same value as (not just
equal to) any other symbol with the same contents.

When evaluated, the contents of the symbol are looked up in the current scope and the given value is
returned.

Example: `long-identifier-name-with-dashes`

### Errors

Errors in Scarab are lists where the first value is a symbol defining the type of error and the
second value, if present, is a string describing the error.

For instance: `('not-function "First value in form is not a function")`

If a given operation raises an error, the evaluation of the current form list immediately stops and
raises the same error. Beyond that, the behavior may vary, but usually the error is raised again by
each level in the stack until the root interpreter is reached.

### Interpreters and Scopes

All code in Scarab is run within an interpreter with its own scope. This starts as the root
interpreter, running in the global scope. Any blocks inside that scope, such as if statements,
are run with a new interpreter and a new scope whose parent is the current scope. When variables are
looked up, the current scope is checked, then the parent, and so on until there are no more parents
to check. If the variable has not been found, a `'no-such-variable` error is raised.

The global scope has a read-only parent, the *builtins scope*, where all built-in functions are
defined. This scope does not have any parent.

Any functions that are defined have the current scope saved as their closure scope, and, when
called, run within a new scope containing their argument values whose parent is that closure scope.

### Evaluation

Scarab, being a Lisp-family language, is centered entirely around evaluating forms. This follows
these steps:

1. Evaluate the first value in the list, according to the rules above.
2. If the result is not a function:
    1. If the result is an atom and there are no other values in the form, the result is the atom.
    2. Otherwise, raise a `'not-function` error.
3. Otherwise, the argument list is prepared:
    1. The number of remaining values in the form is checked against the number of allowable
	   arguments for the given function; if invalid, a `'wrong-num-arguments' error is raised.
    2. Each value in turn is checked against the function's argument definition. If the argument
	   is defined as `'verbatim`, the value is directly used as an argument. Otherwise, it is
       evaluated, and the result used as an argument.
4. Finally, the function is called and the returned value is the result.

### Method Binding

Scarab allows functions to be bound as methods to any type, including any builtin type and any
defined `record-type`. These bindings are stored on the root interpreter, similar to but separate
from builtin functions.

## Builtin Functions

These functions are defined in the *builtins scope*, defined above. Their input and output values
and types are defined using the following signature format:

~~~
function-name:
    arg1: string,
    arg2: integer | string,
    arg3: any,
    real,
    arg5: any 'verbatim,
    arg6? = default: string,
    integer...
    -> string
~~~

Each argument may have a name, and/or one or more types that the argument value may be. In order,
the above example arguments:

1. May only take a value of type `string`.
2. May take a value of type `integer` or `string`.
3. May take a value of any type.
4. Does not have a name, may take a value of type `real`.
5. May take a value of any type, which is not evaluated before being given to the function.
6. May take a value of the type `string`; may be elided, in which case it has a value of `default`.
7. May take 0 or more extra arguments of the type `integer`.

In the above example, the function returns a value of type `string`. This may be omitted if the
function returns nothing.

List arguments are specified as follows:

~~~
lookup:
    names: (symbol 'verbatim, ...)
    map: ((name: symbol 'verbatim, value: any) ...)
    -> (any, ...)
~~~

This function takes two arguments: a list of symbols, and a list of two-element lists containing a
symbol and a value. It returns a list of values.

A function may have multiple signatures with different types. If any of these functions are called
with not enough or too many arguments, a `'wrong-num-arguments` error is raised. If any of the
arguments are of an invalid type, a `'wrong-argument-type` error is raised.

### `+` - adds numbers

~~~
+: integer, integer, integer... -> integer
   real | integer, real | integer, real | integer... -> real
~~~

Adds multiple numbers. If all arguments are `integer`s, returns an `integer`. Otherwise, returns a `real`.

### `=` - sets values

`=: name: symbol 'verbatim, value: any`

Binds `value` to `name` within the current scope.

### `@` - calls methods

`@: object: any, method-name: symbol 'verbatim, args: any... -> any`

Looks up `method-name` for the type of `object`. If it is found, then the given function is called
with `object` as the first argument, then anything in `args`.

### `atom?` - tests a value for atomicity

`atom?: any -> boolean`

Returns `true` if the argument is an *atom*, as defined above.

### `def` - defines functions

~~~
def:
    name: symbol `verbatim,
    arguments: (
        name: symbol 'verbatim |
        (name: symbol 'verbatim, type?: type | (type, ...), option: symbol...)
    ),
    body: list
    -> function
~~~

Defines a function in the current scope with the name `name` and returns it..

The argument definition is a list of either plain symbols or more complex definitions. If `type` is
specified, the passed argument must be one of the given types.

Options are specified as quoted symbols, like so: `(arg-name 'option1 'option2)`. Currently, the
following options can be defined:

`'verbatim`
: Any values for this argument are note evaluated before being passed to the function.
`'default` (must be followed by a value)
: The argument may be elided, and has the value of the following expression (evaluated each time the
function is called.

`body` should be a list of forms, which are run whenever the function is called. The value of the
last form in `body` is the return value of the function.

### `def-method` - defines methods

~~~
def-method:
    type: type,
    name: symbol `verbatim,
    arguments: ...,
    body: list
    -> function
~~~

Creates a function (see `def` for the format of `arguments`), attaches it as a method named
`name` to `type` and returns it. 

### `eval` - evaluates a single form

`eval: list -> any`

Evaluates list as a single form and returns the result.

### `first` - returns the first value of a list

`first: list -> any`

Returns the first value of a list, if any, otherwise `nil`.

### `get-key` - looks up members of a record

`get-key: record, key: symbol 'verbatim -> any`

Looks up the given key in the record, raises a `'no-such-key` error if not found.

### `lambda` - creates an unnamed function

`lambda: arguments: ..., body: list -> function`

Creates a function (see `def` for the format of `arguments`) and returns it. 

### `let` - runs code with a set of bound variables

~~~
let:
   bindings: ((name: symbol 'verbatim, value: any) ...)
   body: list
~~~

Runs `body` as a list of forms within a new scope containing the given variables, and returns the
result of the last form.

### `make` - creates a record

`make: record-type, any, any...`

Creates a record of the given type, with values matching the number and order of the keys in the
original `record-type`.

### `print` - prints values

`print: any, any...`

Prints out each of the given values in a rough string representation, separated by spaces and
followed by a newline.

### `quote` - wraps a value

`quote: any -> any`

Exists mainly as the parsed form of `'expression`. When called, returns its argument.

### `record-type` - creates a new record-type

~~~
record-type:
    name: symbol 'verbatim,
    members: (symbol 'verbatim, ...)
~~~

Creates a new `record-type` named `name` with members named `members`, and stores it in the current
scope.

### `rest` - returns the rest of a list

~~~
rest: list -> list
~~~

Returns a list containing all but the first element of the given list, or nil.

# Language specification (WIP)

Scarab is a statically-typed Lisp-family language. It has syntax extensions for improved readability and built-in support for actor based concurrency.

## Syntax

Scarab's syntax is mostly that of a simple Lisp, with the small addition of two new list syntaxes.

### Lexical structure

The syntax consists of the following tokens:

```
delimiters = ( | ) | [ | ] | { | } | ' | ;
newline = "\n" # Significant inside form lists, ignored otherwise. 
integer = [1-9][0-9]* | 0x[1-9a-fA-F]+ | 0b[01]+
string = "[^"]*" # May include unescaped newlines.
identifier = [[ any character not a prefix of above classes ]]
```

### Grammar

```
value -> integer | string | identifier | quoted | list | operator-list | form-list
quoted -> ' value
list -> ( value* )
operator-list -> [ value (identifier value)+ ]
implicit-form-list -> (value* (; | newline))*
form-list -> { implicit-form-list }
```

The last two list styles, `operator-list` and `form-list`, are only seen at parsing time, and are converted like so:

```
[a op b op c op d] == (op a b c d) *
{a b; c d} == ((a b) (c d))
```

\* All operators in an operator list must be identical.

An implicit form list, without braces, is the form of a Scarab program.

# Types

Scarab's type system consists of a small set of basic types and structural types consisting of combinations of those types.

## Basic types

The following basic value types map roughly to the options for `value` from the grammar:

* Integer: a signed integer type as wide as the underlying architecture efficiently supports.
* String: a possibly-empty sequence of UTF-8 characters.
* Identifier: a sequence of UTF-8 characters.
* Boolean: the special identifier `true` or `false`.
* Nil: the special identifier `nil`.
* Quoted: a wrapper around any other value.
* Cell: a container holding a left and right value.

## Structural types

These types are combinations of the above types.

* List: a nested combination of Cells forming a singly-linked list. Each level of the list is a Cell where the left side is an element in the list and the right side is a Cell following the same pattern or `nil`. For instance, the list `(a b c)` maps to `Cell(a, Cell(b, Cell(c, nil)))`.
* Program: a List of Lists.
* Any: any type.

# Program structure

Each statement in a program is an expression. These expressions can be any type, and are evaluated as follows:

* Integer, String, and Boolean: evaluate to themselves (are _atoms_).
* List: evaluated as a function call.
* Quoted: TBD

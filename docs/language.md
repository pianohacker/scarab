# Language specification (WIP)

Scarab is a statically-typed Lisp-family language. It has syntax extensions for improved readability and built-in support for actor based concurrency.

## Syntax

Scarab's syntax is mostly that of a simple Lisp, with the small addition of two new list syntaxes.

### Lexical structure

The syntax consists of the following tokens:

```
delimiters = ( | ) | [ | ] | { | } | ,
newline = "\n" # Significant inside form lists, ignored otherwise. 
integer = [1-9][0-9]* | 0x[1-9a-fA-F]+ | 0b[01]+
string = "[^"]*" # May include unescaped newlines.
identifier = [[ any character that does not begin one of the types above ]]+
```

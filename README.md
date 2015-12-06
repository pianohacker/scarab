# Scarab

Scarab is an interpreted Lisp-family language with focuses on:

  * Easy and fluid metaprogramming
  * Clear syntax
  * Rapid startup time

## Building

First, make sure that you have CMake, Glib and its development headers installed.

Then, create a build directory and compile:

```console
$ mkdir build
$ cd build
$ cmake ..
$ make
```

The interpreter will then be at `build/scarab`.

## Language basics

The most noticeable addition to Scarab's syntax, as compared to other Lisp-like languages, is two
new list syntaxes. The classic syntax for a list still works as expected:

```
> '(a b 1 2)
(a b 1 2)
```

But there are two new kinds of lists: an operator list and a form list.

Mathematical operators in Lisp have always been awkward, as infix operator notation is used in
almost every other setting. In Scarab, a list surrounded in square brackets will be turned into
infix notation:

```
> '[a + b]
(+ a b)
```

This works for multiple arguments and for nested expressions:

```
> '[a + b + c]
(+ a b c)
> '[[a * 3] + d]
(+ (* a 3) d)
```

This isn't a full expression parser, so `[a * 3 + d]` will not work; all of the operators in such a
list must be the same.

Form lists, surrounded by curly braces, allow cleaner definition of sequences of forms (or
statements):

```
> '{foo 1 2, bar 3 4}
((foo 1 2) (bar 3 4))
```

Each of the lists within the top level list is separated by commas. This works perfectly fine across
multiple lines, but a newline does not automatically separate lists. A comma is required between
each list (and may be left after the last list, for simplicity's sake):

```
{
    foo 1 2,
    bar 3 4,
}
```

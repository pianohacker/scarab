/*
 * Copyright (C) 2015 Jesse Weaver <pianohacker@gmail.com>
 *
 * This library is free software; you can redistribute it and/or
 * modify it under the terms of the GNU Lesser General Public
 * License as published by the Free Software Foundation; either
 * version 3 of the License, or (at your option) any later version.
 *
 * This library is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
 * Lesser General Public License for more details.
 *
 * You should have received a copy of the GNU Lesser General Public
 * License along with this library; if not, write to the Free Software
 * Foundation, Inc., 51 Franklin St, Fifth Floor, Boston, MA  02110-1301  USA
 */

// These are Scarab's builtin functions, which form much of what would be in the interpreter in
// other languages.

#include <glib.h>
#include <limits.h>
#include <stdio.h>
#include <string.h>

#include "eval.h"
#include "list.h"
#include "util.h"
#include "value.h"

// # Utility functions

// This parses a list of argument descriptions into its component pieces.
static long _parse_arg_desc(KhValue *arg_desc, char ***func_argnames) {
	long argc = kh_list_length(arg_desc);
	*func_argnames = g_malloc(sizeof(char*) * argc);

	long i = 0;
	KH_ITERATE(arg_desc) (*func_argnames)[i++] = elem->d_left->d_str;

	return argc;
}

// In order to create a function from its argument list and underlying form, we have to:
static KhValue* _create_func(KhContext *ctx, const gchar *name, KhValue *arg_desc, KhValue *form, bool is_direct) {
	char **func_argnames;
	// First, parse the argument names into a more palatable form.
	long func_argc = _parse_arg_desc(arg_desc, &func_argnames);

	// Then, we have to create a function definition and a value to wrap that function.
	return kh_new_func(kh_func_new(name, form, func_argc, func_argc, func_argnames, kh_context_get_scope(ctx), is_direct));
}

// # Builtin definitions
// ## `+` - add 1 or more integers
//
// Takes 1 or more integer arguments and returns their sum.
static KhValue* add(KhContext *ctx, long argc, KhValue **argv) {
	int result = 0;

	for (int i = 0; i < argc; i++) {
		result += argv[i]->d_int;
	}

	return kh_new_int(result);
}

// ## `.` - get fields of things
static KhValue* get_field(KhContext *ctx, long argc, KhValue **argv) {
	KhValue *value = kh_eval(ctx, argv[0]);
	_REQUIRE(value);
	KhValue *result = kh_get_field(ctx, value, argv[1]->d_str);

	if (result == NULL) KH_FAIL(bad-field, "no such field %s", argv[1]->d_str);

	return result;
}

// ## `@` - call methods
//
// Given a thing, a method name, and a set of arguments, calls the given method on that thing.
static KhValue* call_field(KhContext *ctx, long argc, KhValue **argv) {
	KhValue *value = kh_eval(ctx, argv[0]);
	_REQUIRE(value);

	return kh_call_field(ctx, value, argv[1]->d_str, argc - 2, argv + 2);
}

// ## `def` - defines functions
//
// Defines a new function and adds it to the symbol table. Takes the name of the function, a list of
// names and a list of forms that are the body of the function:
//    
//     def foobar (a b c) {print foo, [1 + 1]}
//
// Where the value of the last form in the body is the return value of the function.
static KhValue* def(KhContext *ctx, long argc, KhValue **argv) {
	kh_scope_add(kh_context_get_scope(ctx), argv[0]->d_str, _create_func(ctx, argv[0]->d_str, argv[1], argv[2], false));

	return kh_nil;
}

// ## `def-direct` - defines direct functions
//
// As above, but defines a direct function (where the arguments to the function are not evaluated
// before being passed).
static KhValue* def_direct(KhContext *ctx, long argc, KhValue **argv) {
	kh_scope_add(kh_context_get_scope(ctx), argv[0]->d_str, _create_func(ctx, argv[0]->d_str, argv[1], argv[2], true));

	return kh_nil;
}

// ## `eval` - evaluate forms
//
// Evaluates the given form in the current scope.
static KhValue* eval(KhContext *ctx, long argc, KhValue **argv) {
	return kh_eval(ctx, argv[0]);
}

// ## `inspect` - returns a string describing a value
//
// This will return a string describing the contents of the given value. This may not be directly
// parsable, as it is intended for human consumption.
static KhValue* inspect(KhContext *ctx, long argc, KhValue **argv) {
	return kh_new_string_take(kh_inspect(argv[0]));
}

// ## `lambda` - define an inline function
//
// Same as def, but returns the function value instead of adding it to the symbol table.
static KhValue* lambda(KhContext *ctx, long argc, KhValue **argv) {
	return _create_func(ctx, "*lambda*", argv[0], argv[1], false);
}

// ## `let` - evaluate forms with local variable values
//
// Takes two arguments, a list of variable definitions and a form to evaluate with those definitions
// in play. For instance:
//
//     let {a 1, b 2} {[a + b]}
//
// will return 3.
static KhValue* let(KhContext *ctx, long argc, KhValue **argv) {
	KhScope *let_scope = kh_context_new_scope(ctx);

	KH_ITERATE(argv[0]) {
		kh_scope_add(let_scope, elem->d_left->d_left->d_str, kh_eval(ctx, elem->d_left->d_right->d_left));
	}

	kh_context_set_scope(ctx, let_scope);
	KhValue *result = kh_eval(ctx, argv[1]);
	_REQUIRE(result);
	kh_context_pop_scope(ctx);

	return result;
}

// ## `print` - prints values to the console
//
// Prints all arguments to the screen (after string conversion), separated with spaces and
// terminated with a space.
static KhValue* print(KhContext *ctx, long argc, KhValue **argv) {
	for (long i = 0; i < argc; i++) {
		KhValue *str = kh_call_field_values(ctx, argv[i], "to-string", NULL);
		fputs(str->d_str, stdout);
		if (i != argc - 1) putchar(' ');
	}

	putchar('\n');

	return kh_nil;
}

// ## `quote` - returns values unevaluated
//
// Returns its argument unevaluated.
static KhValue* quote(KhContext *ctx, long argc, KhValue **argv) {
	return argv[0];
}

// ## `set` - set values in the current scope
//
// Sets the symbol with the given name to the given value.
static KhValue* set(KhContext *ctx, long argc, KhValue **argv) {
	kh_scope_add(kh_context_get_scope(ctx), argv[0]->d_str, argv[1]);

	return kh_nil;
}

#define _REG_VARARGS(name, func, min_argc, max_argc, is_direct) kh_scope_add(_builtins_scope, #name, kh_new_func(kh_func_new_c(#name, func, min_argc, max_argc, is_direct)));
#define _REG(name, func, argc, is_direct) _REG_VARARGS(name, func, argc, argc, is_direct)

void _register_builtins(KhScope *_builtins_scope) {
	_REG_VARARGS(+, add, 1, LONG_MAX, false);
	_REG(., get_field, 2, true);
	_REG(=, set, 2, true);
	_REG_VARARGS(@, call_field, 2, LONG_MAX, true);
	_REG(def, def, 3, true);
	_REG(def-direct, def_direct, 3, true);
	_REG(eval, eval, 1, false);
	_REG(inspect, inspect, 1, false);
	_REG(inspect-direct, inspect, 1, true);
	_REG(lambda, lambda, 2, true);
	_REG(let, let, 2, true);
	_REG_VARARGS(print, print, 0, LONG_MAX, false);
	_REG(quote, quote, 1, true);
}

#define _START_THING(name) thing = kh_new_thing(); kh_scope_add(kh_context_get_scope(ctx), #name, thing)
#define _THING_REG_VARARGS(name, func, min_argc, max_argc, is_direct) kh_set_field(ctx, thing, #name, kh_new_func(kh_func_new_c(#name, func, min_argc, max_argc, is_direct)));
#define _THING_REG(name, func, argc, is_direct) _THING_REG_VARARGS(name, func, argc, argc, is_direct)

#define _REQUIRE_SELF_IS(t) if (!KH_IS(argv[0], t)) KH_FAIL(bad-self, "Method must be called on %s, not %s", kh_value_type_name(t), kh_value_type_name(argv[0]->type))

static KhValue* int_to_string(KhContext *ctx, long argc, KhValue **argv) {
	_REQUIRE_SELF_IS(KH_INT);
	return kh_new_string_take(kh_strdupf("%ld", argv[0]->d_int));
}

static KhValue* string_to_string(KhContext *ctx, long argc, KhValue **argv) {
	_REQUIRE_SELF_IS(KH_STRING);
	return argv[0];
}

static KhValue* string_to_symbol(KhContext *ctx, long argc, KhValue **argv) {
	_REQUIRE_SELF_IS(KH_STRING);
	return kh_new_symbol(argv[0]->d_str);
}

static KhValue* symbol_to_string(KhContext *ctx, long argc, KhValue **argv) {
	_REQUIRE_SELF_IS(KH_SYMBOL);
	return kh_new_string(argv[0]->d_str);
}

void _register_globals(KhContext *ctx) {
	KhValue *thing;

	_START_THING(int);
	_THING_REG(to-string, int_to_string, 1, false);

	_START_THING(string);
	_THING_REG(to-string, string_to_string, 1, false);
	_THING_REG(to-symbol, string_to_symbol, 1, false);

	_START_THING(cell);

	_START_THING(symbol);
	_THING_REG(to-string, symbol_to_string, 1, false);

	_START_THING(func);
}

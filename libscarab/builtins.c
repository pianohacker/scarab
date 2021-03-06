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

#include <gc.h>
#include <glib.h>
#include <limits.h>
#include <stdio.h>
#include <string.h>

#include "eval.h"
#include "list.h"
#include "record.h"
#include "util.h"
#include "value.h"

// # Utility functions

// This parses a list of symbols into a C array of strings.
//
// It both returns the length and NULL-terminates the array to allow for different argument passing
// strategies.
static long _extract_symbol_list(KhValue *str_list, const char ***strings) {
	long length = kh_list_length(str_list);
	*strings = GC_MALLOC(sizeof(char*) * (length + 1));

	long i = 0;
	KH_ITERATE(str_list) (*strings)[i++] = KH_SYMBOL(elem)->value;
	(*strings)[i++] = NULL;

	return length;
}

// In order to create a function from its argument list and underlying form, we have to:
static KhValue* _create_func(KhContext *ctx, const gchar *name, KhValue *arg_desc, KhValue *form, bool is_direct) {
	const char **func_argnames;
	// First, parse the argument names into a more palatable form.
	long func_argc = _extract_symbol_list(arg_desc, &func_argnames);

	// Then, we have to create a function definition and a value to wrap that function.
	return kh_func_new(name, form, func_argc, func_argc, func_argnames, kh_context_get_scope(ctx), is_direct);
}

// # Builtin definitions
// ## `+` - add 1 or more integers
//
// Takes 1 or more integer arguments and returns their sum.
static KhValue* add(KhContext *ctx, long argc, KhValue **argv) {
	int result = 0;

	for (int i = 0; i < argc; i++) {
		result += KH_INT(argv[i])->value;
	}

	return kh_int_new(result);
}

// ## `atom?` - true if the argument is an atom
//
// That is, a simple value that returns itself when evaluated.
static KhValue* atom(KhContext *ctx, long argc, KhValue **argv) {
	return kh_is_atom(argv[0]) ? kh_int_new(1) : kh_nil;
}

// ## `=` - set values in the current scope
//
// Sets the symbol with the given name to the given value.
static KhValue* set(KhContext *ctx, long argc, KhValue **argv) {
	KhValue *value = kh_eval(ctx, argv[1]);
	_REQUIRE(value);
	kh_scope_add(kh_context_get_scope(ctx), KH_SYMBOL(argv[0])->value, value);

	return kh_nil;
}

// ## `@` - call methods
//
// Takes a value, method name and an optional number of arguments, and returns the result of calling
// that method on the given object.
static KhValue* call_method(KhContext *ctx, long argc, KhValue **argv) {
	KhValue *value = kh_eval(ctx, argv[0]);
	_REQUIRE(value);

	KhFunc *method = kh_method_lookup(ctx, KH_VALUE_TYPE(value), KH_SYMBOL(argv[1])->value);
	KH_FAIL_UNLESS(method, undefined-method, "%s", KH_SYMBOL(argv[1])->value);

	long call_argc = argc - 1;
	KhValue *call_argv[call_argc];

	// To prevent re-evaluation
	call_argv[0] = kh_quoted_new(value);
	for (long i = 1; i < call_argc; i++) call_argv[i] = argv[i + 1];

	return kh_apply(ctx, method, call_argc, call_argv);
}


// ## `def` - defines functions
//
// Defines a new function and adds it to the symbol table. Takes the name of the function, a list of
// argument names and a list of forms that are the body of the function:
//
//     def foobar (a b c) {print foo, [1 + 1]}
//
// Where the value of the last form in the body is the return value of the function.
static KhValue* def(KhContext *ctx, long argc, KhValue **argv) {
	kh_scope_add(kh_context_get_scope(ctx), KH_SYMBOL(argv[0])->value, _create_func(ctx, KH_SYMBOL(argv[0])->value, argv[1], argv[2], false));

	return kh_nil;
}

// ## `def-direct` - defines direct functions
//
// As above, but defines a direct function (where the arguments to the function are not evaluated
// before being passed).
static KhValue* def_direct(KhContext *ctx, long argc, KhValue **argv) {
	kh_scope_add(kh_context_get_scope(ctx), KH_SYMBOL(argv[0])->value, _create_func(ctx, KH_SYMBOL(argv[0])->value, argv[1], argv[2], true));

	return kh_nil;
}

// ## `def-method` - defines methods
//
// Defines a new function and binds it with the given name to the given type. Takes the type, name
// of the function, a list of argument names and a list of forms that are the body of the function:
//
//     def-method type foobar (self a b c) {print foo, [1 + 1]}
//
// Where the value of the last form in the body is the return value of the function.
static KhValue* def_method(KhContext *ctx, long argc, KhValue **argv) {
	KhValue *type = kh_eval(ctx, argv[0]);
	_REQUIRE(type);
	KhValue *func = _create_func(ctx, KH_SYMBOL(argv[1])->value, argv[2], argv[3], false);

	kh_method_add(ctx, type, KH_SYMBOL(argv[1])->value, KH_FUNC(func));

	return kh_nil;
}

// ## `eval` - evaluate forms
//
// Evaluates the given form in the current scope.
static KhValue* eval(KhContext *ctx, long argc, KhValue **argv) {
	return kh_eval(ctx, argv[0]);
}

// ## `first` - returns the first element of a list
//
// Like `car`, returns the first element of a list.
static KhValue* first(KhContext *ctx, long argc, KhValue **argv) {
	return KH_CELL(argv[0])->left;
}

// ## `get-key` - gets a key from a record
//
// Gets a given key in a record.

static KhValue* get_key(KhContext *ctx, long argc, KhValue **argv) {
	KhValue *record_value = kh_eval(ctx, argv[0]);
	_REQUIRE(record_value);

	KhValue *value = kh_record_get(KH_RECORD(record_value), KH_STRING(argv[1])->value);
	KH_FAIL_UNLESS(value, unknown-key, "No such key %s in record", KH_STRING(argv[1])->value);

	return value;
}

// ## `inspect` - returns a string describing a value
//
// This will return a string describing the contents of the given value. This may not be directly
// parsable, as it is intended for human consumption.
static KhValue* inspect(KhContext *ctx, long argc, KhValue **argv) {
	return kh_string_new_take(kh_inspect(argv[0]));
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
		kh_scope_add(let_scope, KH_STRING(KH_CELL(elem)->left)->value, kh_eval(ctx, KH_CELL(KH_CELL(elem)->right)->left));
	}

	kh_context_set_scope(ctx, let_scope);
	KhValue *result = kh_eval(ctx, argv[1]);
	_REQUIRE(result);
	kh_context_pop_scope(ctx);

	return result;
}

// ## `make` - creates a record
//
// Creates a new record of the given type. The correct number of values must be provided in the same
// order as the original record type.
static KhValue* make(KhContext *ctx, long argc, KhValue **argv) {
	const KhRecordType *type = KH_RECORD_TYPE(argv[0]);
	long num_keys = kh_record_type_get_num_keys(type);
	long num_provided = argc - 1;
	if (num_provided != num_keys) {
		KH_FAIL(invalid-make, "Tried to create record with %d values, expected %d", num_provided, num_keys);
	}

	KhValue *values[num_keys + 1];
	for (int i = 0; i < num_keys; i++) {
		values[i] = argv[i + 1];
	}
	values[num_keys] = NULL;

	return kh_record_new_from_values(type, values);
}

// ## `print` - prints values to the console
//
// Prints all arguments to the screen (after string conversion), separated with spaces and
// terminated with a space.
static KhValue* print(KhContext *ctx, long argc, KhValue **argv) {
	for (long i = 0; i < argc; i++) {
		// TODO: Make to-string again once bindings are ready
		char *str = kh_inspect(argv[i]);
		fputs(str, stdout);
		if (i != argc - 1) putchar(' ');
	}

	putchar('\n');

	return kh_nil;
}

// ## `quote` - returns values unevaluated
static KhValue* quote(KhContext *ctx, long argc, KhValue **argv) {
	return argv[0];
}

// ## `record-type` - Creates a new record type
//
// Creates a new record type with the given name and list of members.
static KhValue* record_type(KhContext *ctx, long argc, KhValue **argv) {
	const char **members;
	_extract_symbol_list(argv[1], &members);

	KhValue *type = kh_record_type_new(members);
	kh_scope_add(kh_context_get_scope(ctx), KH_SYMBOL(argv[0])->value, type);

	return type;
}

// ## `rest` - returns all but the first element of a list
//
// Like `cdr`, returns all but the first element of a list.
static KhValue* rest(KhContext *ctx, long argc, KhValue **argv) {
	return KH_IS_CELL(argv[0]) ? KH_CELL(argv[0])->right : kh_nil;
}

#define _REG_VARARGS(name, func, min_argc, max_argc, is_direct) kh_scope_add(_builtins_scope, name, kh_func_new_c(#name, func, min_argc, max_argc, is_direct));
#define _REG(name, func, argc, is_direct) _REG_VARARGS(name, func, argc, argc, is_direct)

void _register_builtins(KhScope *_builtins_scope) {
	_REG_VARARGS("+", add, 1, LONG_MAX, false);
	_REG("=", set, 2, true);
	_REG_VARARGS("@", call_method, 2, LONG_MAX, true);
	_REG("atom?", atom, 1, false);
	_REG("def", def, 3, true);
	_REG("def-direct", def_direct, 3, true);
	_REG("def-method", def_method, 3, true);
	_REG("eval", eval, 1, false);
	_REG("first", first, 1, false);
	_REG("get-key", get_key, 2, true);
	_REG("inspect", inspect, 1, false);
	_REG("inspect-direct", inspect, 1, true);
	_REG("lambda", lambda, 2, true);
	_REG("let", let, 2, true);
	_REG_VARARGS("make", make, 1, LONG_MAX, false);
	_REG_VARARGS("print", print, 0, LONG_MAX, false);
	_REG("quote", quote, 1, true);
	_REG("record-type", record_type, 2, true);
	_REG("rest", rest, 1, false);
}

// # Builtin methods
// ## `string`
// ### `to-string` - returns a new string with the same contents
static KhValue* string_to_string(KhContext *ctx, long argc, KhValue **argv) {
	return kh_string_new(KH_STRING(argv[0])->value);
}

#define _METH_VARARGS(type, name, func, min_argc, max_argc, is_direct) kh_method_add(ctx, (KhValue*) type, name, KH_FUNC(kh_func_new_c(#name, func, min_argc, max_argc, is_direct)));
#define _METH(type, name, func, argc, is_direct) _METH_VARARGS(type, name, func, argc, argc, is_direct)

void _register_methods(KhContext *ctx) {
	_METH(KH_STRING_TYPE, "to-string", string_to_string, 1, false);
}

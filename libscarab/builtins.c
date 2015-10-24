#include <glib.h>
#include <limits.h>
#include <stdio.h>
#include <string.h>

#include "eval.h"
#include "list.h"
#include "util.h"
#include "value.h"

// Utility functions
//
// This parses a list of argument descriptions into its component pieces.
static long _parse_arg_desc(KhValue *arg_desc, char ***func_argnames) {
	long argc = kh_list_length(arg_desc);
	*func_argnames = g_malloc(sizeof(char*) * argc);

	long i = 0;
	KH_ITERATE(arg_desc) (*func_argnames)[i++] = elem->d_left->d_str;

	return argc;
}

// Creates a function value.
static KhValue* _create_func(KhContext *ctx, const gchar *name, KhValue *arg_desc, KhValue *form, bool is_direct) {
	char **func_argnames;
	long func_argc = _parse_arg_desc(arg_desc, &func_argnames);

	return kh_new_func(kh_func_new(name, form, func_argc, func_argc, func_argnames, kh_context_get_scope(ctx), is_direct));
}

// Builtin definitions
static KhValue* add(KhContext *ctx, long argc, KhValue **argv) {
	int result = 0;

	for (int i = 0; i < argc; i++) {
		result += argv[i]->d_int;
	}

	return kh_new_int(result);
}

static KhValue* call_field(KhContext *ctx, long argc, KhValue **argv) {
	KhValue *value = kh_eval(ctx, argv[0]);
	_REQUIRE(value);

	return kh_call_field(ctx, value, argv[1]->d_str, argc - 2, argv + 2);
}

static KhValue* eval(KhContext *ctx, long argc, KhValue **argv) {
	return kh_eval(ctx, argv[0]);
}

static KhValue* def(KhContext *ctx, long argc, KhValue **argv) {
	kh_scope_add(kh_context_get_scope(ctx), argv[0]->d_str, _create_func(ctx, argv[0]->d_str, argv[1], argv[2], false));

	return kh_nil;
}

static KhValue* def_direct(KhContext *ctx, long argc, KhValue **argv) {
	kh_scope_add(kh_context_get_scope(ctx), argv[0]->d_str, _create_func(ctx, argv[0]->d_str, argv[1], argv[2], true));

	return kh_nil;
}

static KhValue* inspect(KhContext *ctx, long argc, KhValue **argv) {
	return kh_new_string_take(kh_inspect(argv[0]));
}

static KhValue* get_field(KhContext *ctx, long argc, KhValue **argv) {
	KhValue *value = kh_eval(ctx, argv[0]);
	_REQUIRE(value);
	KhValue *result = kh_get_field(ctx, value, argv[1]->d_str);

	if (result == NULL) KH_FAIL(bad-field, "no such field %s", argv[1]->d_str);

	return result;
}

static KhValue* lambda(KhContext *ctx, long argc, KhValue **argv) {
	return _create_func(ctx, "*lambda*", argv[0], argv[1], false);
}

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

static KhValue* print(KhContext *ctx, long argc, KhValue **argv) {
	for (long i = 0; i < argc; i++) {
		KhValue *str = kh_call_field_values(ctx, argv[i], "to-string", NULL);
		fputs(str->d_str, stdout);
		if (i != argc - 1) putchar(' ');
	}

	putchar('\n');

	return kh_nil;
}

static KhValue* set(KhContext *ctx, long argc, KhValue **argv) {
	kh_scope_add(kh_context_get_scope(ctx), argv[0]->d_str, argv[1]);

	return kh_nil;
}

static KhValue* quote(KhContext *ctx, long argc, KhValue **argv) {
	return argv[0];
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

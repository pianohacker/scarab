#include <glib.h>
#include <limits.h>

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

// Builtin definitions
static KhValue* _add(KhContext *ctx, long argc, KhValue **argv) {
	int result = 0;

	for (int i = 0; i < argc; i++) {
		result += argv[i]->d_int;
	}

	return kh_new_int(result);
}

static KhValue* _create_func(KhContext *ctx, const gchar *name, KhValue *arg_desc, KhValue *form, bool is_direct) {
	char **func_argnames;
	long func_argc = _parse_arg_desc(arg_desc, &func_argnames);

	return kh_new_func(kh_func_new(name, form, func_argc, func_argc, func_argnames, kh_context_get_scope(ctx), is_direct));
}

static KhValue* _eval(KhContext *ctx, long argc, KhValue **argv) {
	return kh_eval(ctx, argv[0]);
}

static KhValue* _def(KhContext *ctx, long argc, KhValue **argv) {
	kh_scope_add(kh_context_get_scope(ctx), argv[0]->d_str, _create_func(ctx, argv[0]->d_str, argv[1], argv[2], false));

	return kh_nil;
}

static KhValue* _def_direct(KhContext *ctx, long argc, KhValue **argv) {
	kh_scope_add(kh_context_get_scope(ctx), argv[0]->d_str, _create_func(ctx, argv[0]->d_str, argv[1], argv[2], true));

	return kh_nil;
}

static KhValue* _inspect(KhContext *ctx, long argc, KhValue **argv) {
	return kh_new_string_take(kh_inspect(argv[0]));
}

static KhValue* _get_field(KhContext *ctx, long argc, KhValue **argv) {
	KhValue *value = kh_eval(ctx, argv[0]);
	_REQUIRE(value);
	KhValue *result = kh_get_field(ctx, value, argv[1]->d_str);

	if (result == NULL) KH_FAIL(bad-field, "no such field %s", argv[1]->d_str);

	return result;
}

static KhValue* _lambda(KhContext *ctx, long argc, KhValue **argv) {
	return _create_func(ctx, "*lambda*", argv[0], argv[1], false);
}

static KhValue* _let(KhContext *ctx, long argc, KhValue **argv) {
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

static KhValue* _set(KhContext *ctx, long argc, KhValue **argv) {
	kh_scope_add(kh_context_get_scope(ctx), argv[0]->d_str, argv[1]);

	return kh_nil;
}

static KhValue* _quote(KhContext *ctx, long argc, KhValue **argv) {
	return argv[0];
}

#define _REG_VARARGS(name, func, min_argc, max_argc, is_direct) kh_scope_add(_builtins_scope, #name, kh_new_func(kh_func_new_c(#name, func, min_argc, max_argc, is_direct)));
#define _REG(name, func, argc, is_direct) _REG_VARARGS(name, func, argc, argc, is_direct)

void _register_builtins(KhScope *_builtins_scope) {
	_REG_VARARGS(+, _add, 1, LONG_MAX, false);
	_REG(., _get_field, 2, true);
	_REG(=, _set, 2, true);
	_REG(def, _def, 3, true);
	_REG(def-direct, _def_direct, 3, true);
	_REG(eval, _eval, 1, false);
	_REG(inspect, _inspect, 1, false);
	_REG(inspect-direct, _inspect, 1, true);
	_REG(lambda, _lambda, 2, true);
	_REG(let, _let, 2, true);
	_REG(quote, _quote, 1, true);
}

#define _START_THING(name) thing = kh_new_thing(); kh_scope_add(kh_context_get_scope(ctx), #name, thing)
#define _THING_REG_VARARGS(name, func, min_argc, max_argc, is_direct) kh(_builtins_scope, #name, kh_new_func(kh_func_new_c(#name, func, min_argc, max_argc, is_direct)));
#define _THING_REG(name, func, argc, is_direct) _THING_REG_VARARGS(name, func, argc, argc, is_direct)

void _register_globals(KhContext *ctx) {
	KhValue *thing;

	_START_THING(int);
	_START_THING(string);
	_START_THING(cell);
	_START_THING(symbol);
	_START_THING(func);
}

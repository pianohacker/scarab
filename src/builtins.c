#include <glib.h>

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

static KhValue* _eval(KhContext *ctx, long argc, KhValue **argv) {
	return kh_eval(ctx, argv[0]);
}

static KhValue* _inspect(KhContext *ctx, long argc, KhValue **argv) {
	return kh_new_string_take(kh_inspect(argv[0]));
}

static KhValue* _lambda(KhContext *ctx, long argc, KhValue **argv) {
	char **func_argnames;
	long func_argc = _parse_arg_desc(argv[0], &func_argnames);

	return kh_new_func(kh_func_new("*lambda*", argv[1], func_argc, func_argnames, kh_context_get_scope(ctx), false));
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

static KhValue* _quote(KhContext *ctx, long argc, KhValue **argv) {
	return argv[0];
}

#define _REG(name, func, argc) kh_scope_add(_builtins_scope, #name, kh_new_func(kh_func_new_c(#name, func, argc, false)));
#define _REG_DIRECT(name, func, argc) kh_scope_add(_builtins_scope, #name, kh_new_func(kh_func_new_c(#name, func, argc, true)));

void _register_builtins(KhScope *_builtins_scope) {
	_REG(+, _add, 2);
	_REG(eval, _eval, 1);
	_REG(inspect, _inspect, 1);
	_REG_DIRECT(inspect-direct, _inspect, 1);
	_REG_DIRECT(lambda, _lambda, 2);
	_REG_DIRECT(let, _let, 2);
	_REG_DIRECT(quote, _quote, 1);
}

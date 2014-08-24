#include <glib.h>
#include <limits.h>
#include <stdbool.h>
#include <string.h>

#include "eval.h"
#include "list.h"
#include "util.h"
#include "value.h"

// Core context object; holds information on current execution context.
// This will change as execution goes through various scopes and environments.
struct _KhContext {
	KhScope *global_scope;
	KhScope *scope;
	KhValue *error;

	GHashTable *field_sets;
	GHashTable *prototypes;
};

// Scope object; holds a set of values and a pointer to the parent.
//
// For example, a function might have a scope that links to the global scope, which itself links to
// the builtins scope.
struct _KhScope {
	KhScope *parent;
	GHashTable *table;
};

struct _KhFunc {
	const gchar *name;

	KhValue *form;
	KhScope *scope;
	long min_argc;
	long max_argc;
	char **argnames;

	KhCFunc c_func;

	bool is_direct;
};

static KhScope *_builtins_scope = NULL;

extern void _register_builtins(KhScope *_builtins_scope);
extern void _register_globals(KhContext *ctx);

// Create a new scope. NULL can be passed as the parent, which is usually only done for either the
// builtins scope or sandboxed scopes.
KhScope* kh_scope_new(KhScope *parent) {
	KhScope *scope = g_slice_new0(KhScope);
	scope->parent = parent;
	// Can use g_direct_equal as variables are referenced by symbols, which are interned
	scope->table = g_hash_table_new(g_str_hash, g_direct_equal);

	return scope;
}

void kh_scope_add(KhScope *scope, char *name, KhValue *val) {
	// This cast is okay as the interned string is guaranteed to continue to exist
	g_hash_table_replace(scope->table, (gchar*) g_intern_string(name), val);
}

KhValue* kh_scope_lookup(KhScope *scope, char *name) {
	// Walk the chain of scopes up as far as we can
	for (; scope != NULL; scope = scope->parent) {
		KhValue *value = g_hash_table_lookup(scope->table, name);
		if (value) return value;
	}

	return NULL;
}

KhContext* kh_context_new() {
	static bool core_init_done = false;
	
	// Have to initialize core values that are used by all execution contexts
	if (!core_init_done) {
		kh_nil = kh_new(KH_NIL);

		_builtins_scope = kh_scope_new(NULL);
		_register_builtins(_builtins_scope);

		core_init_done = true;
	}

	KhContext *ctx = g_slice_new0(KhContext);
	ctx->global_scope = ctx->scope = kh_scope_new(_builtins_scope); // The global scope for the new context
	ctx->field_sets = g_hash_table_new(g_direct_hash, g_direct_equal); // The mapping of KhValue locations to field sets
	ctx->prototypes = g_hash_table_new(g_direct_hash, g_direct_equal); // The mapping of KhValues to their prototype KhValues

	_register_globals(ctx);

	return ctx;
}

KhScope* kh_context_get_scope(KhContext *ctx) {
	return ctx->scope;
}

void kh_context_set_scope(KhContext *ctx, KhScope *scope) {
	ctx->scope = scope;
}

KhScope* kh_context_new_scope(KhContext *ctx) {
	return kh_scope_new(ctx->scope);
}

KhScope* kh_context_push_scope(KhContext *ctx) {
	KhScope *scope = kh_context_new_scope(ctx);
	ctx->scope = scope;

	return scope;
}

KhScope* kh_context_pop_scope(KhContext *ctx) {
	KhScope *scope = ctx->scope;
	g_assert(scope->parent != NULL);
	ctx->scope = scope->parent;

	return scope;
}

void kh_set_error(KhContext *ctx, KhValue *error) {
	ctx->error = error;
}

KhValue* kh_get_error(KhContext *ctx) {
	return ctx->error;
}

KhFunc* kh_func_new(const gchar *name, KhValue *form, long min_argc, long max_argc, char **argnames, KhScope *scope, bool is_direct) {
	KhFunc *result = g_slice_new0(KhFunc);
	result->name = g_strdup(name);
	result->form = form;
	result->min_argc = min_argc;
	result->max_argc = max_argc;
	result->argnames = argnames;
	result->scope = scope;
	result->is_direct = is_direct;

	return result;
}

KhFunc* kh_func_new_c(const gchar *name, KhCFunc c_func, long min_argc, long max_argc, bool is_direct) {
	KhFunc *result = g_slice_new0(KhFunc);
	result->name = g_strdup(name);
	result->c_func = c_func;
	result->min_argc = min_argc;
	result->max_argc = max_argc;
	result->is_direct = is_direct;

	return result;
}

const gchar* kh_func_get_name(KhFunc *func) {
	return func->name;
}

static GHashTable* _get_field_set(KhContext *ctx, KhValue *value, gboolean autovivify) {
	GHashTable *result = g_hash_table_lookup(ctx->field_sets, value);

	if (result == NULL && autovivify) {
		result = g_hash_table_new(g_str_hash, g_direct_equal);
		g_hash_table_replace(ctx->field_sets, value, result);
	}

	return result;
}

static KhValue* _get_prototype(KhContext *ctx, KhValue *value) {
	KhValue *result = g_hash_table_lookup(ctx->prototypes, value);

	if (result == NULL) {
		gchar *global_name = NULL;
		switch (value->type) {
			case KH_INT: global_name = "int"; break;
			case KH_STRING: global_name = "string"; break;
			case KH_CELL: global_name = "cell"; break;
			case KH_SYMBOL: global_name = "symbol"; break;
			case KH_FUNC: global_name = "func"; break;
			default: break;
		}

		if (global_name) return kh_scope_lookup(ctx->global_scope, (gchar*) g_intern_string(global_name));
	}

	return result;
}

KhValue* kh_get_field(KhContext *ctx, KhValue *value, const gchar *name) {
	if (value == kh_nil) return NULL;

	name = g_intern_string(name);

	while (value != NULL) {
		GHashTable *field_set = _get_field_set(ctx, value, false);

		if (field_set) {
			KhValue *result = g_hash_table_lookup(field_set, name);

			if (result) return result;
		}

		value = _get_prototype(ctx, value);
	}

	return NULL;
}

bool kh_set_field(KhContext *ctx, KhValue *value, const gchar *name, KhValue *content) {
	if (value == kh_nil) {
		KH_ERROR(bad-field, "cannot set properties on nil");	
		return false;
	}

	const gchar *intern_name = g_intern_string(name);

	GHashTable *field_set = _get_field_set(ctx, value, true);
	// We're playing fast and loose with casting as we never destroy keys
	g_hash_table_replace(field_set, (gchar*) intern_name, content);

	return true;
}

KhValue* kh_eval(KhContext *ctx, KhValue *form) {
	KhValue *value;

	switch (form->type) {
		case KH_NIL:
		case KH_INT:
		case KH_STRING:
		case KH_FUNC:
		case KH_THING:
			return form;
		case KH_SYMBOL:
			value = kh_scope_lookup(ctx->scope, form->d_str);

			if (value == NULL) KH_FAIL(undefined-variable, "%s", form->d_str);

			return value;
		case KH_CELL:
			break;
	}

	// Easiest way to resolve symbols/lambdas referring to funcs
	KhValue *head = kh_eval(ctx, form->d_left);
	_REQUIRE(head);

	long form_len = kh_list_length(form);
	if (!KH_IS_FUNC(head)) {
		if (form_len == 1) {
			return head;
		} else {
			KH_FAIL(not-func, "Tried to evaluate %s as a function", kh_inspect(head));
		}
	}

	long argc = form_len - 1;
	KhValue *argv[argc];

	int i = 0;
	KH_ITERATE(form->d_right) argv[i++] = elem->d_left;

	return kh_apply(ctx, head->d_func, argc, argv);
}

KhValue* kh_apply(KhContext *ctx, KhFunc *func, long argc, KhValue **argv) {
	if (!func->is_direct) {
		for (long i = 0; i < argc; i++) {
			argv[i] = kh_eval(ctx, argv[i]);
			_REQUIRE(argv[i]);
		}
	}

	if (argc < func->min_argc || argc > func->max_argc) {
		if (func->max_argc == LONG_MAX) {
			KH_FAIL(invalid-call, "Called %s with %ld arguments, expected %ld or more", func->name, argc, func->min_argc);
		} else if (func->min_argc == func->max_argc) {
			KH_FAIL(invalid-call, "Called %s with %ld arguments, expected %ld", func->name, argc, func->min_argc);
		} else {
			KH_FAIL(invalid-call, "Called %s with %ld arguments, expected between %ld and %ld", func->name, argc, func->min_argc, func->max_argc);
		}
	}

	if (func->c_func) {
		return func->c_func(ctx, argc, argv);
	} else {
		KhScope *prev_scope = kh_context_get_scope(ctx);
		KhScope *func_scope = kh_scope_new(func->scope);

		for (long i = 0; i < argc; i++) {
			kh_scope_add(func_scope, func->argnames[i], argv[i]);
		}

		kh_context_set_scope(ctx, func_scope);
		KhValue *result = kh_eval(ctx, func->form);
		_REQUIRE(result);
		kh_context_set_scope(ctx, prev_scope);

		return result;
	}
}

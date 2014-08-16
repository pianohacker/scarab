#include <glib.h>
#include <stdbool.h>
#include <string.h>

#include "eval.h"
#include "list.h"
#include "value.h"

// Core context object; holds information on current execution context.
// This will change as execution goes through various scopes and environments.
struct _KhContext {
	KhScope *scope;
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
	KhValue *form;
	KhScope *scope;
	long argc;
	char **argnames;

	KhCFunc c_func;

	bool is_direct;
};

static KhScope *_builtins_scope = NULL;

extern void _register_builtins(KhScope *_builtins_scope);

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
	g_hash_table_insert(scope->table, (gchar*) g_intern_string(name), val);
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
	ctx->scope = kh_scope_new(_builtins_scope); // This is the global scope for the new context

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

KhValue* kh_eval(KhContext *ctx, KhValue *form) {
	KhValue *value;

	switch (form->type) {
		case KH_NIL:
		case KH_INT:
		case KH_STRING:
		case KH_FUNC:
			return form;
		case KH_SYMBOL:
			value = kh_scope_lookup(ctx->scope, form->d_str);

			return value == NULL ? kh_nil : value;
		case KH_CELL:
			break;
	}

	// Easiest way to resolve symbols/lambdas referring to funcs
	KhValue *head = kh_eval(ctx, form->d_left);

	long form_len = kh_list_length(form);
	if (!KH_IS_FUNC(head)) {
		if (form_len == 1) {
			return head;
		} else {
			return kh_nil;
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
		for (long i = 0; i < argc; i++) argv[i] = kh_eval(ctx, argv[i]);
	}

	if (func->c_func) {
		return func->c_func(ctx, argc, argv);
	} else {
		KhScope *prev_scope = kh_context_get_scope(ctx);
		KhScope *func_scope = kh_scope_new(func->scope);

		if (argc != func->argc) return kh_nil;

		for (long i = 0; i < argc; i++) {
			kh_scope_add(func_scope, func->argnames[i], argv[i]);
		}

		kh_context_set_scope(ctx, func_scope);
		KhValue *result = kh_eval(ctx, func->form);
		kh_context_set_scope(ctx, prev_scope);

		return result;
	}
}

KhFunc* kh_func_new(KhValue *form, long argc, char **argnames, KhScope *scope, bool is_direct) {
	KhFunc *result = g_slice_new0(KhFunc);
	result->form = form;
	result->argc = argc;
	result->argnames = argnames;
	result->scope = scope;
	result->is_direct = is_direct;

	return result;
}

KhFunc* kh_func_new_c(KhCFunc c_func, bool is_direct) {
	KhFunc *result = g_slice_new0(KhFunc);
	result->c_func = c_func;
	result->is_direct = is_direct;

	return result;
}

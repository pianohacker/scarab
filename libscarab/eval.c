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

// This is the core of Scarab's evaluator. It contains the scope and function structures,
// the execution context that all code in a given environment runs within, and the s-expression
// evaluator.
//
// As Scarab is a Lisp-family language, much of the behavior that would be runtime-level in other
// languages is defined in the builtin functions. Check `builtins.c` for those.

#include <glib.h>
#include <limits.h>
#include <stdarg.h>
#include <stdbool.h>
#include <string.h>

#include "eval.h"
#include "list.h"
#include "util.h"
#include "value.h"

// # Scopes

// A given scope is rather simple; it only contains a link to its parent (if any) and a map of the
// variables in it.
struct _KhScope {
	KhScope *parent;
	GHashTable *table;
};

// Note that Scarab's scoping is lexical. When a scope is created for a new function definition,
// its parent is the defining function.

// Since there is no reason to define the builtin functions multiple times, a parent for the global
// scope is constructed once. This scope is de-facto read-only, as no code can execute in the
// builtins scope and thus no code can change its variables.
static KhScope *_builtins_scope = NULL;
extern void _register_builtins(KhScope *_builtins_scope);

KhScope* kh_scope_new(KhScope *parent) {
	KhScope *scope = GC_NEW(KhScope);
	scope->parent = parent;
	// Can use g_direct_equal as variables are referenced by symbols, which are interned
	scope->table = g_hash_table_new(g_str_hash, g_direct_equal);

	return scope;
}

void kh_scope_add(KhScope *scope, char *name, KhValue *val) {
	// This cast is okay, as the interned string is guaranteed to continue to exist.
	g_hash_table_replace(scope->table, (gchar*) g_intern_string(name), val);
}

KhValue* kh_scope_lookup(KhScope *scope, char *name) {
	for (; scope != NULL; scope = scope->parent) {
		KhValue *value = g_hash_table_lookup(scope->table, name);
		if (value) return value;
	}

	return NULL;
}

// ## Things

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


// # Contexts

// Each separate Scarab execution environment has a context, which contains the scopes, global
// definitions and other status information for that environment.
struct _KhContext {
	// Currently, each context has only a single global scope. This scope contains, among other
	// things, the prototype things for the base types.
	KhScope *global_scope;
	// As we move through different functions, the current active scope will change.
	KhScope *scope;
	// We also have to keep track of the most recent error, so it is available after the
	// interpreter's stack has unwound.
	KhValue *error;

	// Instead of holding the child values of each thing within the given `KhValue`, we store them
	// as a global map-of-maps.
	GHashTable *field_sets;
	// A similar approach is used for the prototype links of each thing.
	GHashTable *prototypes;
};

// This function has to be called with the full context so that the global things can have their
// fields set.
//
// Also, as the base types can be extended, it has to be called for every new context.
extern void _register_globals(KhContext *ctx);

KhContext* kh_context_new() {
	static bool core_init_done = false;
	
	// This is the singleton logic for the builtins scope (and a few other small details).
	if (!core_init_done) {
		// For instance, we only need one nil (and this way, we can compare it pointerwise).
		kh_nil = kh_new(KH_NIL);

		_builtins_scope = kh_scope_new(NULL);
		_register_builtins(_builtins_scope);

		core_init_done = true;
	}

	KhContext *ctx = GC_NEW(KhContext);
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

// # Functions

// Each function record has to contain both the information to validate and bind function parameters
// and the actual code (whether native or Scarab).
//
// Also, a function can be direct, which means that its arguments are not evaluated before being
// passed to the function. This is similar to upvars in Tcl, and is our current cheap replacement
// for macros.
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

KhFunc* kh_func_new(const gchar *name, KhValue *form, long min_argc, long max_argc, char **argnames, KhScope *scope, bool is_direct) {
	KhFunc *result = GC_NEW(KhFunc);
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
	KhFunc *result = GC_NEW(KhFunc);
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

// # Evaluator

// This evaluator is a classic Lisp-family evaluator, with (currently) no optimizations such as
// bytecode compilation.
KhValue* kh_eval(KhContext *ctx, KhValue *form) {
	KhValue *value;

	// ## Atomic values

	switch (form->type) {
		// All of the below atomic types evaluate to themselves.
		case KH_NIL:
		case KH_INT:
		case KH_STRING:
		case KH_FUNC:
		case KH_THING:
			return form;

		// Evaluating a symbol will look it up in the current and all containing scopes, returning
		// an error if it does not exist.
		case KH_SYMBOL:
			value = kh_scope_lookup(ctx->scope, form->d_str);

			if (value == NULL) KH_FAIL(undefined-variable, "%s", form->d_str);

			return value;

		// This is a value with a preceding `'`, which should be treated as if it were atomic.
		case KH_QUOTED:
			return form->d_quoted;

		// Finally, if we get an actual form, we get to the interesting bit.
		case KH_CELL:
			break;
	}

	// # Forms
	//
	// First, we have to evaluate the first item in the form to figure out what we're calling.
	KhValue *head = kh_eval(ctx, form->d_left);
	_REQUIRE(head);

	// If the result of that evaluation wasn't a function, we either:
	long form_len = kh_list_length(form);
	if (!KH_IS_FUNC(head)) {
		if (form_len == 1) {
			// return it unmodified if there were no arguments, or:
			return head;
		} else {
			// yell if there were arguments, as this is probably an error.
			//
			// It may be worth doing this in all cases, as this would match Scheme and catch cases
			// where the user thought they were calling a function that takes no arguments.
			KH_FAIL(not-func, "Tried to evaluate %s as a function", kh_inspect(head));
		}
	}

	// Once that error checking is done, we then make a list of all the arguments and pass it to
	// `apply`.
	long argc = form_len - 1;
	KhValue *argv[argc];

	int i = 0;
	KH_ITERATE(form->d_right) argv[i++] = elem->d_left;

	return kh_apply(ctx, head->d_func, argc, argv);
}

KhValue* kh_apply(KhContext *ctx, KhFunc *func, long argc, KhValue **argv) {
	// If this is not a direct function, we have to get the value of each of the arguments.
	if (!func->is_direct) {
		for (long i = 0; i < argc; i++) {
			argv[i] = kh_eval(ctx, argv[i]);
			_REQUIRE(argv[i]);
		}
	}

	// Currently, argument validation is limited to checking argument counts.
	if (argc < func->min_argc || argc > func->max_argc) {
		// It's worth noting that `LONG_MAX` is being used as a cheap way of saying "can accept an
		// infinite number of arguments."
		if (func->max_argc == LONG_MAX) {
			KH_FAIL(invalid-call, "Called %s with %ld arguments, expected %ld or more", func->name, argc, func->min_argc);
		} else if (func->min_argc == func->max_argc) {
			KH_FAIL(invalid-call, "Called %s with %ld arguments, expected %ld", func->name, argc, func->min_argc);
		} else {
			KH_FAIL(invalid-call, "Called %s with %ld arguments, expected between %ld and %ld", func->name, argc, func->min_argc, func->max_argc);
		}
	}

	if (func->c_func) {
		// Evaluating C functions is easy; we just pass off the arguments to the native
		// implementation.
		return func->c_func(ctx, argc, argv);
	} else {
		// If it's a Scarab function, we have to create a new scope whose parent is the scope where
		// the function was defined. Lexical scoping, ladies and gentlemen.
		//
		// We also need to save the old scope to restore it at the end.
		KhScope *prev_scope = kh_context_get_scope(ctx);
		KhScope *func_scope = kh_scope_new(func->scope);
		kh_context_set_scope(ctx, func_scope);

		// Each of the argument values needs to be bound within this new scope.
		for (long i = 0; i < argc; i++) {
			kh_scope_add(func_scope, func->argnames[i], argv[i]);
		}

		// Finally, we evaluate the function's body and restore the old scope.
		KhValue *result = kh_eval(ctx, func->form);
		_REQUIRE(result);
		kh_context_set_scope(ctx, prev_scope);

		return result;
	}
}

KhValue* kh_call_field(KhContext *ctx, KhValue *self, char *method, long argc, KhValue **argv) {
	KhValue *head = kh_get_field(ctx, self, method);
	if (!head) KH_FAIL(bad-field, "no such method %s", method);
	if (!KH_IS_FUNC(head)) KH_FAIL(not-func, "Tried to evaluate %s as a function", kh_inspect(head));

	long apply_argc = argc + 1;
	KhValue *apply_argv[apply_argc];

	apply_argv[0] = kh_new_quoted(self); // Copy over the object
	memcpy(apply_argv + 1, argv, sizeof(KhValue*) * (apply_argc - 1)); // Copy over the arguments

	return kh_apply(ctx, head->d_func, apply_argc, apply_argv);
}

KhValue* kh_call_field_values(KhContext *ctx, KhValue *self, char *method, ...) {
	long argc = 0;

	// We have to know the number of arguments in advance to allocate stack storage, so we have to
	// pre-scan the varargs list.
	va_list args;
	va_start(args, method);

	while (va_arg(args, KhValue*)) argc++;

	va_end(args);

	KhValue *argv[argc];

	if (argc) {
		va_start(args, method);

		for (long i = 0; i < argc; i++) argv[i] = va_arg(args, KhValue*);

		va_end(args);
	}

	return kh_call_field(ctx, self, method, argc, argv);
}

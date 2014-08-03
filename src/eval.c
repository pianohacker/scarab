#include <glib.h>
#include <stdbool.h>
#include <string.h>

#include "eval.h"
#include "list.h"
#include "value.h"

struct _KhContext {
};

struct _KhFunc {
	KhCFunc c_func;
	bool is_direct;
};

static GHashTable *_builtin_namespace = NULL;

extern void _register_builtins(GHashTable *_builtin_namespace);

KhContext* kh_context_new() {
	static bool core_init_done = false;
	
	if (!core_init_done) {
		kh_nil = kh_new(KH_NIL);

		// Can use g_direct_equal as functions are referenced by symbols, which are interned
		_builtin_namespace = g_hash_table_new(g_str_hash, g_direct_equal);
		_register_builtins(_builtin_namespace);

		core_init_done = true;
	}

	return g_slice_new0(KhContext);
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
			value = g_hash_table_lookup(_builtin_namespace, form->d_str);

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
	KhFunc *func = head->d_func;
	form = form->d_right;

	if (func->is_direct) {
		KH_ITERATE(form) argv[i++] = form->d_left;
	} else {
		KH_ITERATE(form) argv[i++] = kh_eval(ctx, form->d_left);
	}

	return head->d_func->c_func(ctx, argc, argv);
}

KhFunc* kh_func_new_c(KhCFunc c_func, bool is_direct) {
	KhFunc *result = g_slice_new0(KhFunc);
	result->c_func = c_func;
	result->is_direct = is_direct;

	return result;
}

#ifndef __EVAL_H__
#define __EVAL_H__

#include <stdbool.h>

#include "value.h"

typedef struct _KhContext KhContext;
typedef struct _KhScope KhScope;

KhScope* kh_scope_new(KhScope *parent);
void kh_scope_add(KhScope *scope, char *name, KhValue *val);

KhContext* kh_context_new();
KhScope* kh_context_get_scope(KhContext *ctx);
void kh_context_set_scope(KhContext *ctx, KhScope *scope);
KhScope* kh_context_new_scope(KhContext *ctx);
KhScope* kh_context_push_scope(KhContext *ctx);
KhScope* kh_context_pop_scope(KhContext *ctx);

#define KH_FAIL_VAL(val) { kh_set_error(ctx, (val)); return NULL; }
#define KH_FAIL(type, msg, ...) KH_FAIL_VAL(kh_new_cell(kh_new_symbol(#type), kh_new_cell(kh_new_string_take(g_strdup_printf(msg, __VA_ARGS__)), kh_nil)))

void kh_set_error(KhContext *ctx, KhValue *error);
KhValue* kh_get_error(KhContext *ctx);

KhValue* kh_eval(KhContext *ctx, KhValue *form);
KhValue* kh_apply(KhContext *ctx, KhFunc *func, long argc, KhValue **argv);

typedef KhValue* (*KhCFunc)(KhContext *ctx, long argc, KhValue **argv);
KhFunc* kh_func_new(const gchar *name, KhValue *form, long min_argc, long max_argc, char **argnames, KhScope *scope, bool is_direct);
KhFunc* kh_func_new_c(const gchar *name, KhCFunc c_func, long min_argc, long max_argc, bool is_direct);
const gchar* kh_func_get_name(KhFunc *func);

#endif

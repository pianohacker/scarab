#ifndef __EVAL_H__
#define __EVAL_H__

#include <stdbool.h>

#include "strfuncs.h"
#include "value.h"

typedef struct _KhContext KhContext;
typedef struct _KhScope KhScope;
typedef struct _KhFunc KhFunc;

KhScope* kh_scope_new(KhScope *parent);
void kh_scope_add(KhScope *scope, const char *name, KhValue *val);

KhContext* kh_context_new();
KhScope* kh_context_get_scope(KhContext *ctx);
void kh_context_set_scope(KhContext *ctx, KhScope *scope);
KhScope* kh_context_new_scope(KhContext *ctx);
KhScope* kh_context_push_scope(KhContext *ctx);
KhScope* kh_context_pop_scope(KhContext *ctx);

#define KH_ERROR(type, msg, ...) kh_set_error(ctx, kh_cell_new(kh_symbol_new(#type), kh_cell_new(kh_string_new_take(kh_strdupf(msg, ##__VA_ARGS__)), kh_nil)))
#define KH_FAIL(type, msg, ...) { KH_ERROR(type, msg, __VA_ARGS__); return NULL; }
#define KH_FAIL_UNLESS(x, type, msg, ...) if (!(x)) KH_FAIL(type, msg, __VA_ARGS__)

void kh_set_error(KhContext *ctx, KhValue *error);
KhValue* kh_get_error(KhContext *ctx);

typedef KhValue* (*KhCFunc)(KhContext *ctx, long argc, KhValue **argv);
KhValue* kh_func_new(const gchar *name, KhValue *form, long min_argc, long max_argc, const char **argnames, KhScope *scope, bool is_direct);
KhValue* kh_func_new_c(const gchar *name, KhCFunc c_func, long min_argc, long max_argc, bool is_direct);
const gchar* kh_func_get_name(const KhFunc *func);

void kh_method_add(KhContext *ctx, KhValue *type, const char *name, KhFunc *func);
KhFunc* kh_method_lookup(KhContext *ctx, KhValue *type, const char *name);

KhValue* kh_eval(KhContext *ctx, KhValue *form);
KhValue* kh_apply(KhContext *ctx, KhFunc *func, long argc, KhValue **argv);
KhValue* kh_apply_values(KhContext *ctx, KhFunc *func, ...);

bool kh_is_atom(KhValue *value);

#endif

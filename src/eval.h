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

KhValue* kh_eval(KhContext *ctx, KhValue *form);

typedef KhValue* (*KhCFunc)(KhContext *ctx, long argc, KhValue **argv);
KhFunc* kh_func_new_c(KhCFunc c_func, bool is_direct);

#endif

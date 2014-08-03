#ifndef __EVAL_H__
#define __EVAL_H__

#include <stdbool.h>

#include "value.h"

typedef struct _KhContext KhContext;

KhContext* kh_context_new();
KhValue* kh_eval(KhContext *ctx, KhValue *form);

typedef KhValue* (*KhCFunc)(KhContext *ctx, long argc, KhValue **argv);
KhFunc* kh_func_new_c(KhCFunc c_func, bool is_direct);

#endif

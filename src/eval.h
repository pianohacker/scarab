#ifndef __EVAL_H__
#define __EVAL_H__

#include "value.h"

typedef struct _KhContext KhContext;

KhContext* kh_context_new();
KhValue* kh_eval(KhContext *ctx, KhValue *form);

#endif

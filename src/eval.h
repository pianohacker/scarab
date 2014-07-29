#ifndef __EVAL_H__
#define __EVAL_H__

#include "value.h"

typedef struct _ScarabContext ScarabContext;

ScarabContext* scarab_context_new();
ScarabValue* scarab_eval(ScarabContext *ctx, ScarabValue *form);

#endif

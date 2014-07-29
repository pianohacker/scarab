#ifndef __LIST_H__
#define __LIST_H__

#include "value.h"

#define KH_ITERATE(list) for (; list != kh_nil; list = list->d_right)

KhValue* kh_list_append(KhValue *list, KhValue *value);
KhValue* kh_list_prepend(KhValue *list, KhValue *value);

#endif

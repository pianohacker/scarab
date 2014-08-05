#ifndef __LIST_H__
#define __LIST_H__

#include "value.h"

#define KH_ITERATE(list) for (KhValue *elem = list; elem != kh_nil; elem = elem->d_right)

long kh_list_length(KhValue *list);

KhValue* kh_list_append(KhValue *list, KhValue *value);
KhValue* kh_list_prepend(KhValue *list, KhValue *value);

#endif

#ifndef __LIST_H__
#define __LIST_H__

#include "value.h"

#define _KH_FIRST(list) (list == kh_nil ? NULL : KH_CELL(list)->left)
#define KH_ITERATE(list) for (KhValue *list_elem = list, *elem __attribute__((unused)) = _KH_FIRST(list); list_elem != kh_nil; list_elem = KH_CELL(list_elem)->right, elem = _KH_FIRST(list_elem))

long kh_list_length(KhValue *list);

KhValue* kh_list_append(KhValue *list, KhValue *value);
KhValue* kh_list_prepend(KhValue *list, KhValue *value);

#endif

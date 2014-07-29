#ifndef __LIST_H__
#define __LIST_H__

#include "value.h"

#define SCARAB_ITERATE(list) for (; list != scarab_nil; list = list->d_right)

ScarabValue* scarab_list_append(ScarabValue *list, ScarabValue *value);
ScarabValue* scarab_list_prepend(ScarabValue *list, ScarabValue *value);

#endif

#ifndef __LIST_H__
#define __LIST_H__

#include "value.h"

ScarabValue* scarab_list_append(ScarabValue *list, ScarabValue *value);
ScarabValue* scarab_list_prepend(ScarabValue *list, ScarabValue *value);

#endif

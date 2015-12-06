#ifndef __RECORD_H__
#define __RECORD_H__

#include <stdbool.h>

#include "value.h"

typedef struct _KhRecordType KhRecordType;

KhRecordType* kh_record_type_new(char* const *keys);

typedef struct _KhRecord KhRecord;
KhRecord* kh_record_new(KhRecordType *type, char* const *keys, const* KhValue *values);
bool kh_record_set(KhRecord *record, const char *key, KhValue *value);
KhValue* kh_record_get(KhRecord *record, const char *key);

#endif

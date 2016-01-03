#ifndef __RECORD_H__
#define __RECORD_H__

#include <stdbool.h>

#include "value.h"

KhRecordType* kh_record_type_new(char* const *keys);
long kh_record_type_get_num_keys(const KhRecordType *type);

KhRecord* kh_record_new(const KhRecordType *type, char* const *keys, KhValue* const *values);
KhRecord* kh_record_new_from_values(const KhRecordType *type, KhValue* const *values);
const KhRecordType* kh_record_get_type(const KhRecord *record);
bool kh_record_set(KhRecord *record, const char *key, KhValue *value);
KhValue* kh_record_get(const KhRecord *record, const char *key);
bool kh_record_foreach(const KhRecord *record, bool (*callback)(const char*, const KhValue*, void*), void *userdata);

#endif

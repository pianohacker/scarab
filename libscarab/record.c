/*
 * Copyright (C) 2015 Jesse Weaver <pianohacker@gmail.com>
 *
 * This library is free software; you can redistribute it and/or
 * modify it under the terms of the GNU Lesser General Public
 * License as published by the Free Software Foundation; either
 * version 3 of the License, or (at your option) any later version.
 *
 * This library is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
 * Lesser General Public License for more details.
 *
 * You should have received a copy of the GNU Lesser General Public
 * License along with this library; if not, write to the Free Software
 * Foundation, Inc., 51 Franklin St, Fifth Floor, Boston, MA  02110-1301  USA
 */

// Scarab's base object type is the record, which is a key-value map with a fixed set of keys
// depending on the record's type.

#include <gc.h>
#include <stdbool.h>
#include <string.h>

#include "record.h"
#include "value.h"

// # Record types
//
// A record type holds all the defined keys for that type and any other bookkeeping information.

struct _KhRecordType {
	KhValue base;

	int num_keys;
	const char **keys;
};

// We expect the array of strings to be `NULL`-terminated.
KhValue* kh_record_type_new(const char **keys) {
	KhRecordType *type = _KH_NEW_BASIC(KH_RECORD_TYPE_TYPE, KhRecordType);

	int num_keys = 0;
	while (keys[num_keys]) num_keys++;

	type->num_keys = num_keys;
	type->keys = GC_MALLOC(sizeof(char*) * num_keys);

	for (int i = 0; i < num_keys; i++) {
		type->keys[i] = GC_STRDUP(keys[i]);
	}

	return (KhValue*) type;
}

long kh_record_type_get_num_keys(const KhRecordType *type) {
	return type->num_keys;
}

// # Records
//
// Each record has a link back to its type and a list of values in the same order as the original
// keys.

struct _KhRecord {
	KhValue base;

	KhValue *values[0];
};

KhValue* kh_record_new(const KhRecordType *type, const char **keys, KhValue* const *values) {
	KhRecord *record = GC_MALLOC(sizeof(KhRecord) + type->num_keys * sizeof(KhValue*));
	((KhValue*) record)->type = (KhValue*) type;

	int num_keys = 0;
	while (keys[num_keys]) num_keys++;

	// We do this slightly backwards (and without `kh_record_set`) to make it easier to fill in
	// `nil`-defaults.
	for (int i = 0; i < type->num_keys; i++) {
		int j;
		for (j = 0; j < num_keys; j++) {
			if (strcmp(type->keys[i], keys[j]) == 0) {
				record->values[i] = values[j];
				break;
			}
		}

		// Record values default to `nil`.
		if (j == num_keys) record->values[i] = kh_nil;
	}

	return (KhValue*) record;
}

KhValue* kh_record_new_from_values(const KhRecordType *type, KhValue* const *values) {
	KhRecord *record = GC_MALLOC(sizeof(KhRecord) + type->num_keys * sizeof(KhValue*));
	((KhValue*) record)->type = (KhValue*) type;

	// We copy in all the values we got, and set any stragglers to nil.
	int i;

	for (i = 0; values[i] && i < type->num_keys; i++) record->values[i] = values[i];
	for (; i < type->num_keys; i++) record->values[i] = kh_nil;

	return (KhValue*) record;
}

const KhRecordType* kh_record_get_type(const KhRecord *record) {
	return (KhRecordType*) KH_VALUE_TYPE(record);
}

// Both setting and getting values in records work basically the same way; the key is searched for
// in the record type's key list, and the matching value is set/returned.
bool kh_record_set(KhRecord *record, const char *key, KhValue *value) {
	const KhRecordType *type = (KhRecordType*) KH_VALUE_TYPE(record);

	for (int i = 0; i < type->num_keys; i++) {
		if (strcmp(type->keys[i], key) == 0) {
			record->values[i] = value;
			return true;
		}
	}
	
	return false;
}

KhValue* kh_record_get(const KhRecord *record, const char *key) {
	const KhRecordType *type = (KhRecordType*) KH_VALUE_TYPE(record);

	for (int i = 0; i < type->num_keys; i++) {
		if (strcmp(type->keys[i], key) == 0) {
			return record->values[i];
		}
	}
	
	return NULL;
}

bool kh_record_foreach(const KhRecord *record, bool (*callback)(const char*, const KhValue*, void*), void *userdata) {
	const KhRecordType *type = kh_record_get_type(record);

	for (int i = 0; i < type->num_keys; i++) {
		if (!callback(type->keys[i], record->values[i], userdata)) return false;
	}

	return true;
}

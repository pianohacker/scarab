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
	int num_keys;
	char **keys;
};

// We expect the array of strings to be `NULL`-terminated.
KhRecordType* kh_record_type_new(char* const *keys) {
	KhRecordType *type = GC_NEW(KhRecordType);

	int num_keys = 0;
	while (keys[num_keys]) num_keys++;

	type->num_keys = num_keys;
	type->keys = GC_MALLOC(sizeof(char*) * num_keys);

	for (int i = 0; i < num_keys; i++) {
		type->keys[i] = GC_STRDUP(keys[i]);
	}

	return type;
}

// # Records
//
// Each record has a link back to its type and a list of values in the same order as the original
// keys.

struct _KhRecord {
	const KhRecordType *type;
	KhValue **values;
};

KhRecord* kh_record_new(const KhRecordType *type, char* const *keys, KhValue* const *values) {
	KhRecord *record = GC_NEW(KhRecord);
	record->type = type;
	record->values = GC_MALLOC(sizeof(KhValue*) * type->num_keys);

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

	return record;
}

// Both setting and getting values in records work basically the same way; the key is searched for
// in the record type's key list, and the matching value is set/returned.
bool kh_record_set(KhRecord *record, const char *key, KhValue *value) {
	const KhRecordType *type = record->type;

	for (int i = 0; i < type->num_keys; i++) {
		if (strcmp(type->keys[i], key) == 0) {
			record->values[i] = value;
			return true;
		}
	}
	
	return false;
}

KhValue* kh_record_get(const KhRecord *record, const char *key) {
	const KhRecordType *type = record->type;

	for (int i = 0; i < type->num_keys; i++) {
		if (strcmp(type->keys[i], key) == 0) {
			return record->values[i];
		}
	}
	
	return NULL;
}

bool kh_record_foreach(const KhRecord *record, bool (*callback)(const char*, const KhValue*, void*), void *userdata) {
	const KhRecordType *type = record->type;

	for (int i = 0; i < type->num_keys; i++) {
		if (!callback(type->keys[i], record->values[i], userdata)) return false;
	}

	return true;
}

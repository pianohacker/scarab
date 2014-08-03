#include <stdio.h>
#include <stdlib.h>

#include "list.h"
#include "value.h"

long kh_list_length(KhValue *list) {
	long length = 0;

	KH_ITERATE(list) length++;

	return length;
}

KhValue* kh_list_append(KhValue *list, KhValue *value) {
	KhValue *new_tail = kh_new_cell(value, kh_nil);
	if (KH_IS_CELL(list)) {
		KhValue *tail = list;

		// Iterate while we have valid cells
		while (KH_IS_CELL(tail->d_right)) tail = tail->d_right;
		
		tail->d_right = new_tail;

		return list;
	} else if (KH_IS_NIL(list)) {
		// Empty, create a list and return it
		return new_tail;
	} else {
		puts("WTF"); abort();
	}
}

KhValue* kh_list_prepend(KhValue *list, KhValue *value) {
	if (KH_IS_CELL(list) || KH_IS_NIL(list)) {
		return kh_new_cell(value, list);
	} else {
		puts("WTF"); abort();
	}
}

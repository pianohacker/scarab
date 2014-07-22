#include <stdio.h>
#include <stdlib.h>

#include "list.h"
#include "value.h"

ScarabValue* scarab_list_append(ScarabValue *list, ScarabValue *value) {
	ScarabValue *new_tail = scarab_new_cell(value, scarab_nil);
	if (SCARAB_IS_CELL(list)) {
		ScarabValue *tail = list;

		// Iterate while we have valid cells
		while (SCARAB_IS_CELL(tail->right)) tail = tail->right;
		
		tail->right = new_tail;

		return list;
	} else if (SCARAB_IS_NIL(list)) {
		// Empty, create a list and return it
		return new_tail;
	} else {
		puts("WTF"); abort();
	}
}

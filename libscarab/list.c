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

// These are utilities for the basic linked list structure that holds all Scarab code.

#include <stdio.h>
#include <stdlib.h>

#include "list.h"
#include "value.h"

// It may be worth caching this within the list.
long kh_list_length(KhValue *list) {
	long length = 0;

	KH_ITERATE(list) length++;

	return length;
}

KhValue* kh_list_append(KhValue *list, KhValue *value) {
	KhValue *new_tail = kh_cell_new(value, kh_nil);
	if (KH_IS_CELL(list)) {
		KhCell *tail = KH_CELL(list);

		// Iterate while we have valid cells.
		while (KH_IS_CELL(tail->right)) tail = KH_CELL(tail->right);
		
		tail->right = new_tail;

		return list;
	} else if (KH_IS_NIL(list)) {
		// Empty, create a list and return it.
		return new_tail;
	} else {
		// FIXME: this is obviously a stopgap for better error handling.
		puts("WTF"); abort();
	}
}

KhValue* kh_list_prepend(KhValue *list, KhValue *value) {
	if (KH_IS_CELL(list) || KH_IS_NIL(list)) {
		return kh_cell_new(value, (KhValue*) list);
	} else {
		puts("WTF"); abort();
	}
}

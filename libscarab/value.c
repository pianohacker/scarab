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

#include <assert.h>
#include <gc.h>
#include <glib.h>
#include <stdbool.h>

// We have to include this first due to some circular-reference mess in the struct definitions.
#include "value.h"

#include "eval.h"
#include "record.h"

static char *_value_type_names[] = {
	"nil",
	"int",
	"string",
	"cell",
	"symbol",
	"func",
	"quoted",
	"record-type",
};

/*const char *kh_value_type_name(KhValueType type) {*/
	/*return _value_type_names[type];*/
/*}*/

KhValue* kh_nil = NULL;

KhValue* kh_nil_new() {
	assert(kh_nil == NULL);

	return _KH_NEW_BASIC(KH_NIL_TYPE, KhValue);
}

KhValue* kh_string_new_take(char *val) {
	KhString *str = _KH_NEW_BASIC(KH_STRING_TYPE, KhString);
	str->value = val;

	return (KhValue *) str;
}

KhValue* kh_string_new(const char *val) {
	return kh_string_new_take(GC_STRDUP(val));
}

KhValue* kh_int_new(long val) {
	KhInt *int_ = _KH_NEW_BASIC(KH_INT_TYPE, KhInt);
	int_->value = val;

	return (KhValue *) int_;
}

KhValue* kh_cell_new(KhValue *left, KhValue *right) {
	KhCell *cell = _KH_NEW_BASIC(KH_CELL_TYPE, KhCell);
	cell->left = left;
	cell->right = right;

	return (KhValue *) cell;
}

KhValue* kh_symbol_new(const char *val) {
	KhSymbol *symbol = _KH_NEW_BASIC(KH_SYMBOL_TYPE, KhSymbol);
	symbol->value = (char *) g_intern_string(val);

	return (KhValue *) symbol;
}

KhValue* kh_quoted_new(KhValue *val) {
	KhQuoted *quoted = _KH_NEW_BASIC(KH_QUOTED_TYPE, KhQuoted);
	quoted->value = val;

	return (KhValue *) quoted;
}

// For _inspect_cell
static void _inspect(const KhValue *value, GString *result);

static void _inspect_int(const KhInt *int_, GString *result) {
	g_string_append_printf(result, "%ld", int_->value);
}

static void _inspect_string(const KhString *string, GString *result) {
	char *repr = g_strescape(string->value, "");
	g_string_append_c(result, '"');
	g_string_append(result, repr);
	g_string_append_c(result, '"');
	g_free(repr);
}

static void _inspect_cell(const KhCell *cell, GString *result, bool in_cell) {
	if (!in_cell) g_string_append_c(result, '(');

	if (KH_IS_CELL(cell->right)) {
		_inspect(cell->left, result);
		g_string_append_c(result, ' ');
		_inspect_cell(KH_CELL(cell->right), result, true);
	} else if (cell->right == kh_nil) {
		_inspect(cell->left, result);
	} else {
		_inspect(cell->left, result);
		g_string_append(result, " . ");
		_inspect(cell->right, result);
	}

	if (!in_cell) g_string_append_c(result, ')');
}

static void _inspect_func(const KhFunc *func, GString *result) {
	g_string_append_printf(result, "*function \"%s\"*", kh_func_get_name(func));
}

static bool _inspect_record_pair_cb(const char *key, const KhValue *value, void *userdata) {
	GString *result = (GString*) userdata;

	g_string_append_c(result, ' ');
	g_string_append(result, key);
	g_string_append_c(result, ' ');
	_inspect(value, result);

	return true;
}

static void _inspect(const KhValue *value, GString *result) {
	if (KH_IS_BASIC(value)) {
		switch (KH_BASIC_TYPE(value->type)) {
			case KH_NIL_TYPE:
				g_string_append(result, "nil");
				break;
			case KH_INT_TYPE:
				_inspect_int(KH_INT(value), result);
				break;
			case KH_STRING_TYPE:
				_inspect_string(KH_STRING(value), result);
				break;
			case KH_CELL_TYPE:
				_inspect_cell(KH_CELL(value), result, false);
				break;
			case KH_SYMBOL_TYPE:
				g_string_append(result, KH_SYMBOL(value)->value);
				break;
			case KH_FUNC_TYPE:
				_inspect_func(KH_FUNC(value), result);
				break;
			case KH_QUOTED_TYPE:
				g_string_append(result, "(quote ");
				_inspect(KH_QUOTED(value)->value, result);
				g_string_append_c(result, ')');
				break;
			case KH_RECORD_TYPE_TYPE:
				g_string_append(result, "*record-type*");
				break;
		}
	} else if (KH_IS_RECORD(value)) {
		g_string_append(result, "(*record");
		kh_record_foreach(KH_RECORD(value), _inspect_record_pair_cb, result);
		g_string_append_c(result, ')');
	}
}

char* kh_inspect(const KhValue *value) {
	GString *result = g_string_new("");

	_inspect(value, result);

	return g_string_free(result, FALSE);
}

#include <glib.h>
#include <stdbool.h>

#include "value.h"

ScarabValue* scarab_nil = NULL;

ScarabValue* scarab_new(ScarabValueType type) {
	ScarabValue *value = g_slice_new0(ScarabValue);
	value->type = type;

	return value;
}

ScarabValue* scarab_new_string(const char *val) {
	ScarabValue *value = scarab_new(SCARAB_STRING);
	value->d_str = g_strdup(val);

	return value;
}

ScarabValue* scarab_new_int(long val) {
	ScarabValue *value = scarab_new(SCARAB_INT);
	value->d_int = val;

	return value;
}

ScarabValue* scarab_new_cell(ScarabValue *left, ScarabValue *right) {
	ScarabValue *value = scarab_new(SCARAB_CELL);
	value->d_left = left;
	value->d_right = right;

	return value;
}

// For _inspect_cell
void _inspect(ScarabValue *value, GString *result);

void _inspect_int(ScarabValue *value, GString *result) {
	g_string_append_printf(result, "%ld", value->d_int);
}

void _inspect_string(ScarabValue *value, GString *result) {
	char *repr = g_strescape(value->d_str, "");
	g_string_append(result, repr);
	g_free(repr);
}

void _inspect_cell(ScarabValue *value, GString *result, bool in_cell) {
	if (!in_cell) g_string_append_c(result, '(');

	if (value->d_right->type == SCARAB_CELL) {
		_inspect(value->d_left, result);
		g_string_append_c(result, ' ');
		_inspect_cell(value->d_right, result, true);
	} else if (value->d_right->type == SCARAB_NIL) {
		_inspect(value->d_left, result);
	} else {
		_inspect(value->d_left, result);
		g_string_append(result, " . ");
		_inspect(value->d_left, result);
	}

	if (!in_cell) g_string_append_c(result, ')');
}

void _inspect(ScarabValue *value, GString *result) {
	switch (value->type) {
		case SCARAB_NIL:
			g_string_append(result, "nil");
			break;
		case SCARAB_INT:
			_inspect_int(value, result);
			break;
		case SCARAB_STRING:
			_inspect_string(value, result);
			break;
		case SCARAB_CELL:
			_inspect_cell(value, result, false);
			break;
	}
}

const char* scarab_inspect(ScarabValue *value) {
	GString *result = g_string_new("");

	_inspect(value, result);

	return g_string_free(result, FALSE);
}

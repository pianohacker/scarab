#include <glib.h>
#include <stdbool.h>

#include "value.h"

KhValue* kh_nil = NULL;

KhValue* kh_new(KhValueType type) {
	KhValue *value = g_slice_new0(KhValue);
	value->type = type;

	return value;
}

KhValue* kh_new_string_take(const char *val) {
	KhValue *value = kh_new(KH_STRING);
	value->d_str = val;

	return value;
}

KhValue* kh_new_string(const char *val) {
	return kh_new_string_take(g_strdup(val));
}

KhValue* kh_new_int(long val) {
	KhValue *value = kh_new(KH_INT);
	value->d_int = val;

	return value;
}

KhValue* kh_new_cell(KhValue *left, KhValue *right) {
	KhValue *value = kh_new(KH_CELL);
	value->d_left = left;
	value->d_right = right;

	return value;
}

KhValue* kh_new_symbol(const char *val) {
	KhValue *value = kh_new(KH_SYMBOL);
	value->d_str = g_intern_string(val);

	return value;
}

KhValue* kh_new_func(KhFunc *val) {
	KhValue *value = kh_new(KH_FUNC);
	value->d_func = val;

	return value;
}

// For _inspect_cell
static void _inspect(KhValue *value, GString *result);

static void _inspect_int(KhValue *value, GString *result) {
	g_string_append_printf(result, "%ld", value->d_int);
}

static void _inspect_string(KhValue *value, GString *result) {
	char *repr = g_strescape(value->d_str, "");
	g_string_append_c(result, '"');
	g_string_append(result, repr);
	g_string_append_c(result, '"');
	g_free(repr);
}

static void _inspect_cell(KhValue *value, GString *result, bool in_cell) {
	if (!in_cell) g_string_append_c(result, '(');

	if (value->d_right->type == KH_CELL) {
		_inspect(value->d_left, result);
		g_string_append_c(result, ' ');
		_inspect_cell(value->d_right, result, true);
	} else if (value->d_right->type == KH_NIL) {
		_inspect(value->d_left, result);
	} else {
		_inspect(value->d_left, result);
		g_string_append(result, " . ");
		_inspect(value->d_left, result);
	}

	if (!in_cell) g_string_append_c(result, ')');
}

static void _inspect(KhValue *value, GString *result) {
	switch (value->type) {
		case KH_NIL:
			g_string_append(result, "nil");
			break;
		case KH_INT:
			_inspect_int(value, result);
			break;
		case KH_STRING:
			_inspect_string(value, result);
			break;
		case KH_CELL:
			_inspect_cell(value, result, false);
			break;
		case KH_SYMBOL:
			g_string_append(result, value->d_str);
			break;
		case KH_FUNC:
			g_string_append(result, "*internal-function*");
			break;
	}
}

const char* kh_inspect(KhValue *value) {
	GString *result = g_string_new("");

	_inspect(value, result);

	return g_string_free(result, FALSE);
}

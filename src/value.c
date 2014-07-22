#include <glib.h>

#include "value.h"

ScarabValue* scarab_new(ScarabValueType type) {
	ScarabValue *value = g_slice_new(ScarabValue);
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
	value->d_cell.left = left;
	value->d_cell.right = right;

	return value;
}

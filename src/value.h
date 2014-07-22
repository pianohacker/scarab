#ifndef __VALUE_H__
#define __VALUE_H__

#include <assert.h>

#include "cell.h"

#define SCARAB_IS(val, t) ((val)->type == (t))
#define SCARAB_IS_NIL(val) SCARAB_IS(val, SCARAB_NIL)
#define SCARAB_IS_INT(val) SCARAB_IS(val, SCARAB_INT)
#define SCARAB_IS_STRING(val) SCARAB_IS(val, SCARAB_STRING)
#define SCARAB_IS_CELL(val) SCARAB_IS(val, SCARAB_CELL)
#define SCARAB_ASSERT_IS(val, t) assert(SCARAB_IS(val, t));

typedef enum {
	SCARAB_NIL = 0,
	SCARAB_INT,
	SCARAB_STRING,
	SCARAB_CELL,
} ScarabValueType;

typedef struct ScarabValue {
	ScarabValueType type;
	
	union {
		char *d_str;
		long d_int;
		struct {
			ScarabValue *d_left;
			ScarabValue *d_right;
		};
	};
};

extern ScarabValue *scarab_nil;

ScarabValue* scarab_new(ScarabValueType type);
ScarabValue* scarab_new_string(const char *val);
ScarabValue* scarab_new_int(long val);
ScarabValue* scarab_new_cell(ScarabValue *left, ScarabValue *right);

#endif

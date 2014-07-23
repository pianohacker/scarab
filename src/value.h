#ifndef __VALUE_H__
#define __VALUE_H__

#include <assert.h>

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
	SCARAB_SYMBOL,
} ScarabValueType;

typedef struct _ScarabValue {
	ScarabValueType type;
	
	union {
		char *d_str;
		long d_int;
		struct {
			struct _ScarabValue *d_left;
			struct _ScarabValue *d_right;
		};
	};
} ScarabValue;

extern ScarabValue *scarab_nil;

ScarabValue* scarab_new(ScarabValueType type);
ScarabValue* scarab_new_int(long val);
ScarabValue* scarab_new_string(const char *val);
ScarabValue* scarab_new_cell(ScarabValue *left, ScarabValue *right);
ScarabValue* scarab_new_symbol(const char *val);

const char* scarab_inspect(ScarabValue *value);

#endif

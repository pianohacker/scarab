#ifndef __VALUE_H__
#define __VALUE_H__

#include <assert.h>

#define KH_IS(val, t) ((val)->type == (t))
#define KH_IS_NIL(val) KH_IS(val, KH_NIL)
#define KH_IS_INT(val) KH_IS(val, KH_INT)
#define KH_IS_STRING(val) KH_IS(val, KH_STRING)
#define KH_IS_CELL(val) KH_IS(val, KH_CELL)
#define KH_IS_SYMBOL(val) KH_IS(val, KH_SYMBOL)
#define KH_IS_FUNC(val) KH_IS(val, KH_FUNC)
#define KH_ASSERT_IS(val, t) assert(KH_IS(val, t));

typedef enum {
	KH_NIL = 0,
	KH_INT,
	KH_STRING,
	KH_CELL,
	KH_SYMBOL,
	KH_FUNC,
	KH_THING,
	KH_QUOTED,
	KH_RECORD_TYPE,
	KH_RECORD,
} KhValueType;

const char *kh_value_type_name(KhValueType type);

// This is here for the KhValue declaration below, and cannot be filled in until eval.h
typedef struct _KhFunc KhFunc;

typedef struct _KhValue {
	KhValueType type;
	
	union {
		char *d_str;
		long d_int;
		struct {
			struct _KhValue *d_left;
			struct _KhValue *d_right;
		};
		KhFunc *d_func;
		struct _KhValue *d_quoted;
		KhRecordType *d_record_type;
		KhRecord *d_record;
	};
} KhValue;

extern KhValue *kh_nil;

KhValue* kh_new(KhValueType type);
KhValue* kh_new_int(long val);
KhValue* kh_new_string(const char *val);
KhValue* kh_new_string_take(char *val);
KhValue* kh_new_cell(KhValue *left, KhValue *right);
KhValue* kh_new_symbol(const char *val);
KhValue* kh_new_func(KhFunc *val);
KhValue* kh_new_quoted(KhValue *val);
KhValue* kh_new_thing();
KhValue* kh_new_record_type(KhRecordType *type);
KhValue* kh_new_record(KhRecord *record);

char* kh_inspect(KhValue *value);

#endif

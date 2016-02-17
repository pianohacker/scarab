#ifndef __VALUE_H__
#define __VALUE_H__

#include <assert.h>

typedef enum {
	KH_NIL_TYPE = 0,
	KH_INT_TYPE,
	KH_STRING_TYPE,
	KH_CELL_TYPE,
	KH_SYMBOL_TYPE,
	KH_FUNC_TYPE,
	KH_QUOTED_TYPE,
	KH_RECORD_TYPE_TYPE,
	KH_NUM_BASIC_TYPES,
} KhBasicType;

#define KH_BASIC_TYPE(type) ((KhBasicType) type)
#define KH_IS_TYPE(type) (KH_BASIC_TYPE(type) < KH_NUM_BASIC_TYPES || KH_IS_RECORD_TYPE(type))

#define KH_VALUE_TYPE(val) (((KhValue*) val)->type)
#define KH_IS(val, t) (((KhValue*) val)->type == (KhValue*) (t))
#define KH_IS_BASIC(val) (KH_BASIC_TYPE(val->type) < KH_NUM_BASIC_TYPES)
#define KH_IS_NIL(val) KH_IS(val, KH_NIL_TYPE)
#define KH_IS_INT(val) KH_IS(val, KH_INT_TYPE)
#define KH_IS_STRING(val) KH_IS(val, KH_STRING_TYPE)
#define KH_IS_CELL(val) KH_IS(val, KH_CELL_TYPE)
#define KH_IS_SYMBOL(val) KH_IS(val, KH_SYMBOL_TYPE)
#define KH_IS_FUNC(val) KH_IS(val, KH_FUNC_TYPE)
#define KH_IS_QUOTED(val) KH_IS(val, KH_QUOTED_TYPE)
#define KH_IS_RECORD_TYPE(val) KH_IS(val, KH_RECORD_TYPE_TYPE)
#define KH_IS_RECORD(val) (!KH_IS_BASIC(val) && KH_IS_RECORD_TYPE(val->type))

#define _KH_CHECKED_CAST(val, t, struct_type) ({ assert(KH_IS(val, t)); (struct_type*) val; })
#define KH_INT(val) _KH_CHECKED_CAST(val, KH_INT_TYPE, KhInt)
#define KH_STRING(val) _KH_CHECKED_CAST(val, KH_STRING_TYPE, KhString)
#define KH_CELL(val) _KH_CHECKED_CAST(val, KH_CELL_TYPE, KhCell)
#define KH_SYMBOL(val) _KH_CHECKED_CAST(val, KH_SYMBOL_TYPE, KhSymbol)
#define KH_FUNC(val) _KH_CHECKED_CAST(val, KH_FUNC_TYPE, KhFunc)
#define KH_QUOTED(val) _KH_CHECKED_CAST(val, KH_QUOTED_TYPE, KhQuoted)
#define KH_RECORD_TYPE(val) _KH_CHECKED_CAST(val, KH_RECORD_TYPE_TYPE, KhRecordType)
#define KH_RECORD(val) (assert(KH_IS_RECORD(val)), (KhRecord*) val)

#define _KH_NEW_BASIC(t, struct_type) ({ struct_type *result = GC_MALLOC(sizeof(struct_type)); ((KhValue*) result)->type = (KhValue *) t; result; })

typedef struct _KhValue {
	struct _KhValue *type;
} KhValue;

//const char *kh_value_type_name(KhValue *type);

typedef struct {
	KhValue base;

	long value;
} KhInt;

typedef struct {
	KhValue base;

	char *value;
} KhString;

typedef struct {
	KhValue base;

	const char *value;
} KhSymbol;

typedef struct {
	KhValue base;

	KhValue *left;
	KhValue *right;
} KhCell;

typedef struct {
	KhValue base;

	KhValue *value;
} KhQuoted;

extern KhValue *kh_nil;

KhValue* kh_nil_new();
KhValue* kh_int_new(long val);
KhValue* kh_string_new(const char *val);
KhValue* kh_string_new_take(char *val);
KhValue* kh_symbol_new(const char *val);
KhValue* kh_cell_new(KhValue *left, KhValue *right);
KhValue* kh_quoted_new(KhValue *val);

char* kh_inspect(const KhValue *value);

#endif

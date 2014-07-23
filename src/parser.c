/*
 * Copyright (C) 2014 Jesse Weaver <pianohacker@gmail.com>
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

#include <errno.h>
#include <glib.h>
#include <glib-object.h>
#include <stdarg.h>
#include <stdlib.h>
#include <string.h>

#include "error.h"
#include "list.h"
#include "parser.h"
#include "tokenizer.h"

typedef struct {
	ScarabTokenizer *tokenizer;
	ScarabToken *peek_token;
} ScarabParserContext;

#define EXPECT(...) if (!_expect(self, token, err, __VA_ARGS__, 0)) return NULL;
#define REQUIRE(expr) if (!expr) return NULL;

static bool _read(ScarabParserContext *self, ScarabToken **token, GError **err) {
	if (self->peek_token) {
		ScarabToken *result = self->peek_token;
		self->peek_token = NULL;
		*token = result;

		return true;
	} else {
		if (!scarab_tokenizer_next(self->tokenizer, token, err)) {
			return false;
		} else {
			return true;
		}
	}
}

static bool _peek(ScarabParserContext *self, ScarabToken **token, GError **err) {
	if (self->peek_token) {
		*token = self->peek_token;

		return true;
	} else {
		if (!scarab_tokenizer_next(self->tokenizer, &(self->peek_token), err)) {
			return false;
		} else {
			*token = self->peek_token;
			return true;
		}
	}
}

static void _consume(ScarabParserContext *self) {
	g_assert(self->peek_token != NULL);

	self->peek_token = NULL;
}

static void _error(ScarabParserContext *self, ScarabToken *token, ScarabSyntaxError err_type, char *msg, GError **err) {
	g_set_error(err,
		SCARAB_SYNTAX_ERROR,
		err_type,
		"%s in %s, line %d, column %d",
		msg,
		scarab_tokenizer_get_filename(self->tokenizer),
		token->line,
		token->col
	);
	scarab_token_free(token);
}

static bool _expect(ScarabParserContext *self, ScarabToken *token, GError **err, ...) {
	va_list args;
	ScarabTokenType type;

	va_start(args, err);

	while (type = va_arg(args, ScarabTokenType), type != 0 && token->type != type);

	va_end(args);

	if (type == 0) {
		GString *err_string = g_string_new("");
		g_string_sprintf(err_string, "Unexpected %s, expected one of: ", scarab_token_type_name(token->type));

		va_start(args, err);
		type = va_arg(args, ScarabTokenType);

		g_string_append(err_string, scarab_token_type_name(type));
		while (type = va_arg(args, ScarabTokenType), type != 0) {
			g_string_append(err_string, ", ");
			g_string_append(err_string, scarab_token_type_name(type));
		}

		va_end(args);

		_error(
			self,
			token, 
			SCARAB_SYNTAX_ERROR_MALFORMED,
			err_string->str,
			err
		);

		g_string_free(err_string, TRUE);

		return false;
	} else {
		return true;
	}
}

//> Parser Functions
static bool _token_is_value(ScarabToken *token) {
	// The cast to int is largely to shut up the gcc enum niceties.
	switch ((int) token->type) {
		case '(':
		case '[':
		case T_NUMBER:
		case T_IDENTIFIER:
		case T_STRING:
			return true;
		default:
			return false;
	}
}

static ScarabValue* _parse_number(ScarabParserContext *self, GError **err) {
	ScarabToken *token;
	REQUIRE(_read(self, &token, err));

	char *end;
	errno = 0;
	long value = strtol(token->val, &end, 10);

	if (errno) {
		_error(self, token, SCARAB_SYNTAX_ERROR_BAD_LITERAL, "Long integer out of range", err);

		return NULL;
	}

	ScarabValue *result = scarab_new_int(value);

	scarab_token_free(token);

	return result;
}

/*static bool _parse_value(ScarabParserContext *self, GValue *value) {*/
	/*ScarabToken *token;*/
	/*REQUIRE(_read(self, &token));*/

	/*int sign = 1;*/
	
	/*retry:*/
	/*switch ((int) token->type) {*/
		/*case '-':*/
			/*sign = -1;*/
			/*scarab_token_free(token);*/

			/*REQUIRE(_read(self, &token));*/
			/*EXPECT(T_NUMBER, T_LONGINTEGER, T_DAYS, T_TIME_PART);*/

			/*goto retry;*/

		/*case T_NUMBER:*/
		/*case T_LONGINTEGER:*/
			/*return _parse_number(self, value, token, sign);*/

		/*case T_DATE_PART:*/
			/*return _parse_datetime(self, value, token);*/

		/*case T_DAYS:*/
		/*case T_TIME_PART:*/
			/*return _parse_timespan(self, value, token, sign);*/

		/*case T_BOOLEAN:*/
			/*g_value_init(value, G_TYPE_BOOLEAN);*/

			/*if (strcmp(token->val, "true") == 0 || strcmp(token->val, "on") == 0) {*/
				/*g_value_set_boolean(value, TRUE);*/
			/*} else if (strcmp(token->val, "false") == 0 || strcmp(token->val, "off") == 0) {*/
				/*g_value_set_boolean(value, FALSE);*/
			/*}*/

			/*break;*/

		/*case T_NULL:*/
			/*g_value_init(value, G_TYPE_POINTER);*/
			/*g_value_set_pointer(value, NULL);*/
			/*break;*/

		/*case T_STRING:*/
			/*g_value_init(value, G_TYPE_STRING);*/
			/*g_value_set_string(value, token->val);*/
			/*break;*/

		/*case T_CHAR:*/
			/*g_value_init(value, SCARAB_TYPE_UNICHAR);*/
			/*scarab_gvalue_set_unichar(value, g_utf8_get_char(token->val));*/
			/*break;*/

		/*case T_BINARY:*/
			/*g_value_init(value, SCARAB_TYPE_BINARY);*/

			/*gsize len;*/
			/*guchar *data = g_base64_decode(token->val, &len);*/
			/*scarab_gvalue_take_binary(value, g_byte_array_new_take(data, len));*/

			/*break;*/

		/*default:*/
			/*g_return_val_if_reached(false);*/
	/*}*/

	/*scarab_token_free(token);*/
	/*return true;*/
/*}*/

// For the list parsers
static ScarabValue* _parse_value(ScarabParserContext *self, GError **err);

static ScarabValue* _parse_closed_list(ScarabParserContext *self, ScarabTokenType terminator, GError **err) {
	ScarabValue *result = scarab_nil;

	ScarabToken *token;
	while (true) {
		REQUIRE(_peek(self, &token, err));

		if (!_token_is_value(token)) {
			if (terminator == ')') {
				EXPECT(')');
			} else {
				EXPECT(terminator, ',');
			}

			break;
		}

		ScarabValue *new_value = _parse_value(self, err);

		if (!new_value) return NULL;

		result = scarab_list_append(result, new_value);
	}

	return result;
}

static ScarabValue* _parse_open_list(ScarabParserContext *self, ScarabTokenType terminator, GError **err) {
	ScarabValue *result = scarab_nil;

	ScarabToken *token;
	while (true) {
		REQUIRE(_peek(self, &token, err));

		if (_token_is_value(token)) {
			ScarabValue *new_value = _parse_closed_list(self, terminator, err);

			if (!new_value) return NULL;

			result = scarab_list_append(result, new_value);

			REQUIRE(_peek(self, &token, err));
		}

		EXPECT(',', terminator);

		// This is retarded but arguably so is manual memory management
		if (token->type == terminator) {
			break;
		} else {
			_consume(self);
			scarab_token_free(token);
		}
	}

	return result;
}

static ScarabValue* _parse_value(ScarabParserContext *self, GError **err) {
	ScarabToken *token;
	ScarabValue *new_value;

	REQUIRE(_peek(self, &token, err));

	if (token->type == '(') {
		_consume(self);
		scarab_token_free(token);

		new_value = _parse_closed_list(self, ')', err);

		if (new_value) {
			REQUIRE(_read(self, &token, err));
			EXPECT(')');
			scarab_token_free(token);
		}
	} else if (token->type == '[') {
		_consume(self);
		scarab_token_free(token);

		new_value = _parse_open_list(self, ']', err);

		if (new_value) {
			REQUIRE(_read(self, &token, err));
			EXPECT(']');
			scarab_token_free(token);
		}
	} else if (token->type == T_NUMBER) {
		new_value = _parse_number(self, err);
	}

	return new_value;
}

static ScarabValue* _parse(ScarabParserContext *self, GError **err) {
	ScarabValue* result = _parse_open_list(self, T_EOF, err);

	if (result) {
		ScarabToken *token;
		REQUIRE(_read(self, &token, err));
		EXPECT(T_EOF);
		scarab_token_free(token);
	}

	return result;
}

ScarabValue* scarab_parse_string(const char *str, GError **err) {
	ScarabParserContext *self = g_slice_new0(ScarabParserContext);
	self->tokenizer = scarab_tokenizer_new_from_string(str, err);

	if (!self->tokenizer) {
		return NULL;
	}

	return _parse(self, err);
}

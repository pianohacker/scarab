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

#include <errno.h>
#include <gc.h>
#include <glib.h>
#include <glib-object.h>
#include <stdarg.h>
#include <stdlib.h>
#include <string.h>

#include "error.h"
#include "list.h"
#include "parser.h"
#include "strfuncs.h"
#include "tokenizer.h"
#include "util.h"

typedef struct {
	KhTokenizer *tokenizer;

	bool has_peek;
	KhToken peek_token;
} KhParserContext;

#define EXPECT(...) if (!_expect(self, token, err, __VA_ARGS__, 0)) return NULL;

static bool _read(KhParserContext *self, KhToken *token, GError **err) {
	if (self->has_peek) {
		*token = self->peek_token;
		self->has_peek = false;

		return true;
	} else {
		return kh_tokenizer_next(self->tokenizer, token, err);
	}
}

static bool _peek(KhParserContext *self, KhToken *token, GError **err) {
	if (self->has_peek) {
		*token = self->peek_token;

		return true;
	} else {
		if (kh_tokenizer_next(self->tokenizer, &(self->peek_token), err)) {
			*token = self->peek_token;
			self->has_peek = true;
			return true;
		} else {
			self->has_peek = false;
			return false;
		}
	}
}

static void _consume(KhParserContext *self) {
	g_assert(self->has_peek);

	self->has_peek = false;
}

static void _error(KhParserContext *self, KhToken token, KhSyntaxError err_type, char *msg, GError **err) {
	g_set_error(err,
		KH_SYNTAX_ERROR,
		err_type,
		"%s in %s, line %d, column %d",
		msg,
		kh_tokenizer_get_filename(self->tokenizer),
		token.line,
		token.col
	);
}

static bool _expect(KhParserContext *self, KhToken token, GError **err, ...) {
	va_list args;
	KhTokenType type;

	va_start(args, err);

	while (type = va_arg(args, KhTokenType), type != 0 && token.type != type);

	va_end(args);

	if (type == 0) {
		GString *err_string = g_string_new("");
		g_string_sprintf(err_string, "Unexpected %s, expected one of: ", kh_token_type_name(token.type));

		va_start(args, err);
		type = va_arg(args, KhTokenType);

		g_string_append(err_string, kh_token_type_name(type));
		while (type = va_arg(args, KhTokenType), type != 0) {
			g_string_append(err_string, ", ");
			g_string_append(err_string, kh_token_type_name(type));
		}

		va_end(args);

		_error(
			self,
			token, 
			KH_SYNTAX_ERROR_MALFORMED,
			err_string->str,
			err
		);

		g_string_free(err_string, TRUE);

		return false;
	} else {
		return true;
	}
}

static bool _ignore_newlines(KhParserContext *self, GError **err) {
	while (true) {
		KhToken token = KH_TOKEN_EMPTY;
		_REQUIRE(_peek(self, &token, err));
		if (token.type == '\n') {
			_consume(self);
		} else {
			break;
		}
	}

	return true;
}

//> Parser Functions
static bool _token_type_is_value(KhTokenType token_type) {
	// The cast to int is largely to shut up the gcc enum niceties.
	switch ((int) token_type) {
		case '\'':
		case '(':
		case '[':
		case '{':
		case T_NUMBER:
		case T_IDENTIFIER:
		case T_STRING:
			return true;
		default:
			return false;
	}
}

static KhValue* _parse_number(KhParserContext *self, GError **err) {
	KhToken token = KH_TOKEN_EMPTY;
	_REQUIRE(_read(self, &token, err));

	char *end;
	errno = 0;
	long value = strtol(token.val, &end, 10);

	if (errno) {
		_error(self, token, KH_SYNTAX_ERROR_BAD_LITERAL, "Long integer out of range", err);

		return NULL;
	}

	KhValue *result = kh_int_new(value);

	return result;
}

static KhValue* _parse_string(KhParserContext *self, GError **err) {
	KhToken token = KH_TOKEN_EMPTY;
	_REQUIRE(_read(self, &token, err));

	KhValue *result = kh_string_new(token.val);

	return result;
}

static KhValue* _parse_identifier(KhParserContext *self, GError **err) {
	KhToken token = KH_TOKEN_EMPTY;
	_REQUIRE(_read(self, &token, err));

	KhValue *result;

	if (strcmp(token.val, "nil") == 0) {
		result = kh_nil;
	} else {
		result = kh_symbol_new(token.val);
	}

	return result;
}

// For the list parsers
static KhValue* _parse_value(KhParserContext *self, GError **err);

static KhValue* _parse_operator_list(KhParserContext *self, KhTokenType terminator, GError **err) {
	KhValue *result = kh_nil;
	KhValue *operator = NULL;
	KhToken token = KH_TOKEN_EMPTY;

	_REQUIRE(_ignore_newlines(self, err));
	_REQUIRE(_peek(self, &token, err));

	if (!_token_type_is_value(token.type)) {
		_REQUIRE(_ignore_newlines(self, err));
		EXPECT(terminator);
		return result;
	}

	while (true) {
		_REQUIRE(_ignore_newlines(self, err));
		_REQUIRE(_peek(self, &token, err));

		if (!_token_type_is_value(token.type)) {
			_error(
				self,
				token,
				KH_SYNTAX_ERROR_MALFORMED,
				kh_strdupf("Unexpected %s, expected a value", kh_token_type_name(token.type)),
				err);

			return NULL;
		}

		KhValue *new_value = _parse_value(self, err);

		if (!new_value) return NULL;

		result = kh_list_append(result, new_value);

		_REQUIRE(_ignore_newlines(self, err));
		_REQUIRE(_peek(self, &token, err));

		if (token.type == terminator) break;

		EXPECT(T_IDENTIFIER);

		if (operator) {
			_consume(self);

			if (strcmp(((KhSymbol*) operator)->value, token.val) != 0) {
				_error(
					self,
					token,
					KH_SYNTAX_ERROR_MALFORMED,
					kh_strdupf("Non-matching operator %s in operator list", token.val),
					err);
				return NULL;
			}
		} else {
			operator = _parse_value(self, err);

			result = kh_list_prepend(result, operator);
		}

	}

	return result;
}

static KhValue* _parse_closed_list(KhParserContext *self, KhTokenType terminator, GError **err) {
	KhValue *result = kh_nil;

	KhToken token = KH_TOKEN_EMPTY;
	while (true) {

		if (terminator == ')') {
			_REQUIRE(_ignore_newlines(self, err));
			_REQUIRE(_peek(self, &token, err));

			if (!_token_type_is_value(token.type)) {
				EXPECT(')');
				break;
			}
		} else {
			_REQUIRE(_peek(self, &token, err));

			if (!_token_type_is_value(token.type)) {
				EXPECT(terminator, ',', '\n');
				break;
			}
		}

		KhValue *new_value = _parse_value(self, err);

		if (!new_value) return NULL;

		result = kh_list_append(result, new_value);
	}

	return result;
}

static KhValue* _parse_open_list(KhParserContext *self, KhTokenType terminator, GError **err) {
	KhValue *result = kh_nil;

	KhToken token = KH_TOKEN_EMPTY;
	while (true) {
		_REQUIRE(_peek(self, &token, err));

		if (_token_type_is_value(token.type)) {
			KhValue *new_value = _parse_closed_list(self, terminator, err);

			if (!new_value) return NULL;

			result = kh_list_append(result, new_value);

			_REQUIRE(_peek(self, &token, err));
		}

		EXPECT(',', '\n', terminator);

		if (token.type == terminator) {
			break;
		} else {
			_consume(self);
		}
	}

	return result;
}

static KhValue* _parse_value(KhParserContext *self, GError **err) {
	KhToken token = KH_TOKEN_EMPTY;
	KhValue *new_value = NULL;

	_REQUIRE(_peek(self, &token, err));

	bool quote_value = false;

	if (token.type == '\'') {
		_consume(self);
		quote_value = true;

		_REQUIRE(_peek(self, &token, err));
	}

	if (!_token_type_is_value(token.type) || token.type == '\'') {
		_error(
			self,
			token,
			KH_SYNTAX_ERROR_MALFORMED,
			kh_strdupf("Unexpected %s, expected a value", kh_token_type_name(token.type)),
			err);

		return NULL;
	}

	if (token.type == '(' || token.type == '[' || token.type == '{') {
		KhTokenType terminator;
		_consume(self);

		switch ((int) token.type) {
			case '(':
				terminator = ')';
				new_value = _parse_closed_list(self, terminator, err);
				break;
			case '[':
				terminator = ']';
				new_value = _parse_operator_list(self, terminator, err);
				break;
			case '{':
				terminator = '}';
				new_value = _parse_open_list(self, terminator, err);
				break;
			default: g_warn_if_reached();
		}

		if (new_value) {
			_REQUIRE(_read(self, &token, err));
			EXPECT(terminator);
		}
	} else if (token.type == T_NUMBER) {
		new_value = _parse_number(self, err);
	} else if (token.type == T_STRING) {
		new_value = _parse_string(self, err);
	} else if (token.type == T_IDENTIFIER) {
		new_value = _parse_identifier(self, err);
	}

	if (quote_value && new_value) {
		return kh_quoted_new(new_value);
	}

	return new_value;
}

static KhValue* _parse(KhParserContext *self, GError **err) {
	KhValue* result = _parse_open_list(self, T_EOF, err);

	if (result) {
		KhToken token = KH_TOKEN_EMPTY;
		_REQUIRE(_read(self, &token, err));
		EXPECT(T_EOF);
	}

	return result;
}

KhValue* kh_parse_string(const char *str, GError **err) {
	KhParserContext *self = GC_NEW(KhParserContext);
	self->tokenizer = kh_tokenizer_new_from_string(str, err);

	if (!self->tokenizer) {
		return NULL;
	}

	return _parse(self, err);
}

KhValue* kh_parse_file(const char *filename, GError **err) {
	KhParserContext *self = GC_NEW(KhParserContext);
	self->tokenizer = kh_tokenizer_new(filename, err);

	if (!self->tokenizer) {
		return NULL;
	}

	return _parse(self, err);
}

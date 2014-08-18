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
	KhTokenizer *tokenizer;
	KhToken *peek_token;
} KhParserContext;

#define EXPECT(...) if (!_expect(self, token, err, __VA_ARGS__, 0)) return NULL;
#define REQUIRE(expr) if (!expr) return NULL;

static bool _read(KhParserContext *self, KhToken **token, GError **err) {
	if (self->peek_token) {
		KhToken *result = self->peek_token;
		self->peek_token = NULL;
		*token = result;

		return true;
	} else {
		if (!kh_tokenizer_next(self->tokenizer, token, err)) {
			return false;
		} else {
			return true;
		}
	}
}

static bool _peek(KhParserContext *self, KhToken **token, GError **err) {
	if (self->peek_token) {
		*token = self->peek_token;

		return true;
	} else {
		if (!kh_tokenizer_next(self->tokenizer, &(self->peek_token), err)) {
			return false;
		} else {
			*token = self->peek_token;
			return true;
		}
	}
}

static void _consume(KhParserContext *self) {
	g_assert(self->peek_token != NULL);

	self->peek_token = NULL;
}

static void _error(KhParserContext *self, KhToken *token, KhSyntaxError err_type, char *msg, GError **err) {
	g_set_error(err,
		KH_SYNTAX_ERROR,
		err_type,
		"%s in %s, line %d, column %d",
		msg,
		kh_tokenizer_get_filename(self->tokenizer),
		token->line,
		token->col
	);
	kh_token_free(token);
}

static bool _expect(KhParserContext *self, KhToken *token, GError **err, ...) {
	va_list args;
	KhTokenType type;

	va_start(args, err);

	while (type = va_arg(args, KhTokenType), type != 0 && token->type != type);

	va_end(args);

	if (type == 0) {
		GString *err_string = g_string_new("");
		g_string_sprintf(err_string, "Unexpected %s, expected one of: ", kh_token_type_name(token->type));

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

//> Parser Functions
static bool _token_is_value(KhToken *token) {
	// The cast to int is largely to shut up the gcc enum niceties.
	switch ((int) token->type) {
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
	KhToken *token;
	REQUIRE(_read(self, &token, err));

	char *end;
	errno = 0;
	long value = strtol(token->val, &end, 10);

	if (errno) {
		_error(self, token, KH_SYNTAX_ERROR_BAD_LITERAL, "Long integer out of range", err);

		return NULL;
	}

	KhValue *result = kh_new_int(value);

	kh_token_free(token);

	return result;
}

static KhValue* _parse_string(KhParserContext *self, GError **err) {
	KhToken *token;
	REQUIRE(_read(self, &token, err));

	KhValue *result = kh_new_string(token->val);

	kh_token_free(token);

	return result;
}

static KhValue* _parse_identifier(KhParserContext *self, GError **err) {
	KhToken *token;
	REQUIRE(_read(self, &token, err));

	KhValue *result;

	if (strcmp(token->val, "nil") == 0) {
		result = kh_nil;
	} else {
		result = kh_new_symbol(token->val);
	}

	kh_token_free(token);

	return result;
}

// For the list parsers
static KhValue* _parse_value(KhParserContext *self, GError **err);

static KhValue* _parse_operator_list(KhParserContext *self, KhTokenType terminator, GError **err) {
	KhValue *result = kh_nil;
	KhValue *operator = NULL;
	KhToken *token;

	REQUIRE(_peek(self, &token, err));

	if (!_token_is_value(token)) {
		EXPECT('}');
		return result;
	}

	while (true) {
		REQUIRE(_peek(self, &token, err));

		if (!_token_is_value(token)) {
			_error(
				self,
				token,
				KH_SYNTAX_ERROR_MALFORMED,
				g_strdup_printf("Unexpected %s, expected a value", kh_token_type_name(token->type)),
				err);

			return NULL;
		}

		KhValue *new_value = _parse_value(self, err);

		if (!new_value) return NULL;

		result = kh_list_append(result, new_value);

		REQUIRE(_peek(self, &token, err));

		if (token->type == '}') break;

		EXPECT(T_IDENTIFIER);

		if (operator) {
			_consume(self);

			if (strcmp(operator->d_str, token->val) != 0) {
				_error(
					self,
					token,
					KH_SYNTAX_ERROR_MALFORMED,
					g_strdup_printf("Non-matching operator %s in operator list", token->val),
					err);
				return NULL;
			}

			kh_token_free(token);
		} else {
			operator = _parse_value(self, err);

			result = kh_list_prepend(result, operator);
		}

	}

	return result;
}

static KhValue* _parse_closed_list(KhParserContext *self, KhTokenType terminator, GError **err) {
	KhValue *result = kh_nil;

	KhToken *token;
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

		KhValue *new_value = _parse_value(self, err);

		if (!new_value) return NULL;

		result = kh_list_append(result, new_value);
	}

	return result;
}

static KhValue* _parse_open_list(KhParserContext *self, KhTokenType terminator, GError **err) {
	KhValue *result = kh_nil;

	KhToken *token;
	while (true) {
		REQUIRE(_peek(self, &token, err));

		if (_token_is_value(token)) {
			KhValue *new_value = _parse_closed_list(self, terminator, err);

			if (!new_value) return NULL;

			result = kh_list_append(result, new_value);

			REQUIRE(_peek(self, &token, err));
		}

		EXPECT(',', terminator);

		// This is retarded but arguably so is manual memory management
		if (token->type == terminator) {
			break;
		} else {
			_consume(self);
			kh_token_free(token);
		}
	}

	return result;
}

static KhValue* _parse_value(KhParserContext *self, GError **err) {
	KhToken *token;
	KhValue *new_value = NULL;

	REQUIRE(_peek(self, &token, err));

	bool quote_value = false;

	if (token->type == '\'') {
		_consume(self);
		kh_token_free(token);
		quote_value = true;

		REQUIRE(_peek(self, &token, err));
	}

	if (!_token_is_value(token) || token->type == '\'') {
		_error(
			self,
			token,
			KH_SYNTAX_ERROR_MALFORMED,
			g_strdup_printf("Unexpected %s, expected a value", kh_token_type_name(token->type)),
			err);

		return NULL;
	}

	if (token->type == '(' || token->type == '[' || token->type == '{') {
		KhTokenType terminator;
		_consume(self);

		switch ((int) token->type) {
			case '(':
				terminator = ')';
				new_value = _parse_closed_list(self, terminator, err);
				break;
			case '[':
				terminator = ']';
				new_value = _parse_open_list(self, terminator, err);
				break;
			case '{':
				terminator = '}';
				new_value = _parse_operator_list(self, terminator, err);
				break;
			default: g_warn_if_reached();
		}

		kh_token_free(token);

		if (new_value) {
			REQUIRE(_read(self, &token, err));
			EXPECT(terminator);
			kh_token_free(token);
		}
	} else if (token->type == T_NUMBER) {
		new_value = _parse_number(self, err);
	} else if (token->type == T_STRING) {
		new_value = _parse_string(self, err);
	} else if (token->type == T_IDENTIFIER) {
		new_value = _parse_identifier(self, err);
	}

	if (quote_value && new_value) {
		return kh_new_cell(kh_new_symbol("quote"), kh_new_cell(new_value, kh_nil));
	}

	return new_value;
}

static KhValue* _parse(KhParserContext *self, GError **err) {
	KhValue* result = _parse_open_list(self, T_EOF, err);

	if (result) {
		KhToken *token;
		REQUIRE(_read(self, &token, err));
		EXPECT(T_EOF);
		kh_token_free(token);
	}

	return result;
}

KhValue* kh_parse_string(const char *str, GError **err) {
	KhParserContext *self = g_slice_new0(KhParserContext);
	self->tokenizer = kh_tokenizer_new_from_string(str, err);

	if (!self->tokenizer) {
		return NULL;
	}

	return _parse(self, err);
}

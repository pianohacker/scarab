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
		case '{':
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

static ScarabValue* _parse_string(ScarabParserContext *self, GError **err) {
	ScarabToken *token;
	REQUIRE(_read(self, &token, err));

	ScarabValue *result = scarab_new_string(token->val);

	scarab_token_free(token);

	return result;
}

static ScarabValue* _parse_identifier(ScarabParserContext *self, GError **err) {
	ScarabToken *token;
	REQUIRE(_read(self, &token, err));

	ScarabValue *result = scarab_new_symbol(token->val);

	scarab_token_free(token);

	return result;
}

// For the list parsers
static ScarabValue* _parse_value(ScarabParserContext *self, GError **err);

static ScarabValue* _parse_operator_list(ScarabParserContext *self, ScarabTokenType terminator, GError **err) {
	ScarabValue *result = scarab_nil;
	ScarabValue *operator = NULL;
	ScarabToken *token;

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
				SCARAB_SYNTAX_ERROR_MALFORMED,
				g_strdup_printf("Unexpected %s, expected a value", scarab_token_type_name(token->type)),
				err);

			return NULL;
		}

		ScarabValue *new_value = _parse_value(self, err);

		if (!new_value) return NULL;

		result = scarab_list_append(result, new_value);

		REQUIRE(_peek(self, &token, err));

		if (token->type == '}') break;

		EXPECT(T_IDENTIFIER);

		if (operator) {
			_consume(self);

			if (strcmp(operator->d_str, token->val) != 0) {
				_error(
					self,
					token,
					SCARAB_SYNTAX_ERROR_MALFORMED,
					g_strdup_printf("Non-matching operator %s in operator list", token->val),
					err);
				return NULL;
			}

			scarab_token_free(token);
		} else {
			operator = _parse_value(self, err);

			result = scarab_list_prepend(result, operator);
		}

	}

	return result;
}

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

	if (token->type == '(' || token->type == '[' || token->type == '{') {
		ScarabTokenType terminator;
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

		scarab_token_free(token);

		if (new_value) {
			REQUIRE(_read(self, &token, err));
			EXPECT(terminator);
			scarab_token_free(token);
		}
	} else if (token->type == T_NUMBER) {
		new_value = _parse_number(self, err);
	} else if (token->type == T_STRING) {
		new_value = _parse_string(self, err);
	} else if (token->type == T_IDENTIFIER) {
		new_value = _parse_identifier(self, err);
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

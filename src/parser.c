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

#include "parser.h"
#include "syntax.h"
#include "tokenizer.h"
#include "types.h"

typedef struct {
	ScarabTokenizer *tokenizer;
	ScarabToken *peek_token;
} ScarabParserContext;

#define EXPECT(...) if (!_expect(self, token, __VA_ARGS__, 0)) return NULL;
#define REQUIRE(expr) if (!expr) return NULL;

static bool _read(ScarabParserContext *self, ScarabToken **token) {
	if (self->peek_token) {
		ScarabToken *result = self->peek_token;
		self->peek_token = NULL;
		*token = result;

		return true;
	} else {
		GError *error = NULL;

		if (!scarab_tokenizer_next(self->tokenizer, token, &error)) {
			MAYBE_CALLBACK(self->parser->error, self, error, self->user_data);
			return false;
		} else {
			return true;
		}
	}
}

static bool _peek(ScarabParserContext *self, ScarabToken **token) {
	if (self->peek_token) {
		*token = self->peek_token;

		return true;
	} else {
		GError *error = NULL;

		if (!scarab_tokenizer_next(self->tokenizer, &(self->peek_token), &error)) {
			MAYBE_CALLBACK(self->parser->error, self, error, self->user_data);
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

static void _error(ScarabParserContext *self, ScarabToken *token, ScarabSyntaxError err_type, char *msg) {
	GError *err = NULL;
	g_set_error(&err,
		SCARAB_SYNTAX_ERROR,
		err_type,
		"%s in %s, line %d, column %d",
		msg,
		scarab_tokenizer_get_filename(self->tokenizer),
		token->line,
		token->col
	);
	MAYBE_CALLBACK(self->parser->error, self, err, self->user_data);
}

static bool _expect(ScarabParserContext *self, ScarabToken *token, ...) {
	va_list args;
	ScarabTokenType type;

	va_start(args, token);

	while (type = va_arg(args, ScarabTokenType), type != 0 && token->type != type);

	va_end(args);

	if (type == 0) {
		GString *err_string = g_string_new("");
		g_string_sprintf(err_string, "Unexpected %s, expected one of: ", scarab_token_type_name(token->type));

		va_start(args, token);
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
			err_string->str
		);

		g_string_free(err_string, TRUE);

		return false;
	} else {
		return true;
	}
}

//> Parser Functions
static bool _token_is_value(ScarabToken *token) {
	switch ((int) token->type) {
		case '-':
		case T_NUMBER:
		case T_LONGINTEGER:
		case T_DAYS:
		case T_DATE_PART:
		case T_TIME_PART:
		case T_BOOLEAN:
		case T_NULL:
		case T_STRING:
		case T_CHAR:
		case T_BINARY:
			return true;
		default:
			return false;
	}
}

static bool _parse_number(ScarabParserContext *self, GValue *value, ScarabToken *token, int sign) {
	char *end;
	ScarabToken *next, *parts[1];

	if (token->type == T_LONGINTEGER) {
		g_value_init(value, G_TYPE_INT64);
		errno = 0;
		g_value_set_int64(value, sign * strtoll(token->val, &end, 10));

		if (errno) {
			_error(self, token, SCARAB_SYNTAX_ERROR_BAD_LITERAL, "Long integer out of range");

			return false;
		}

		scarab_token_free(token);
		return true;
	}

	REQUIRE(_peek(self, &next));

	if (next->type == '.') {
		_consume(self);
		scarab_token_free(next);
		parts[0] = token;

		REQUIRE(_read(self, &token));
		EXPECT(T_NUMBER, T_FLOAT_END, T_DOUBLE_END, T_DECIMAL_END);

		char *total = g_strdup_printf("%s%s.%s", sign <= 0 ? "-" : "", parts[0]->val, token->val);
		scarab_token_free(parts[0]);

		switch (token->type) {
			case T_NUMBER:
			case T_DOUBLE_END:
				g_value_init(value, G_TYPE_DOUBLE);

				g_value_set_double(value, strtod(total, &end));

				if (*end) {
					_error(self, token, SCARAB_SYNTAX_ERROR_BAD_LITERAL, "Double out of range");

					return false;
				}

				break;

			case T_FLOAT_END:
				g_value_init(value, G_TYPE_FLOAT);

				g_value_set_float(value, strtof(total, &end));

				if (*end) {
					_error(self, token, SCARAB_SYNTAX_ERROR_BAD_LITERAL, "Float out of range");

					return false;
				}

				break;
			case T_DECIMAL_END:
				g_value_init(value, SCARAB_TYPE_DECIMAL);

				scarab_gvalue_set_decimal(value, total);

				break;
			default:
				g_return_val_if_reached(false);
		}

		g_free(total);
	} else {
		g_value_init(value, G_TYPE_INT);
		g_value_set_int(value, sign * strtol(token->val, &end, 10));
	}

	scarab_token_free(token);
	return true;
}

static bool _parse_timezone(ScarabParserContext *self, GTimeZone **timezone, ScarabToken *first) {
	GString *identifier = g_string_new("");
	ScarabToken *token;

	if (strcmp(first->val, "GMT") == 0) {
		REQUIRE(_read(self, &token));
		EXPECT('+', '-');
		g_string_append_c(identifier, (gchar) token->type);
		scarab_token_free(token);

		REQUIRE(_read(self, &token));
		EXPECT(T_NUMBER, T_TIME_PART);

		if (token->type == T_NUMBER) {
			int val = atoi(token->val);
			g_string_append_printf(identifier, "%02d%02d", val / 100 % 100, val % 100);
			scarab_token_free(token);
		} else {
			g_string_append_printf(identifier, "%02d", atoi(token->val));
			scarab_token_free(token);

			REQUIRE(_read(self, &token));
			EXPECT(T_NUMBER);
			g_string_append_printf(identifier, "%02d", atoi(token->val));
			scarab_token_free(token);
		}
	} else {
		g_string_append(identifier, first->val);

		REQUIRE(_peek(self, &token));

		if (token->type == '/') {
			_consume(self);
			g_string_append_c(identifier, '/');
			scarab_token_free(token);

			REQUIRE(_read(self, &token));
			EXPECT(T_IDENTIFIER);
			g_string_append(identifier, token->val);
			scarab_token_free(token);
		}
	}

	*timezone = g_time_zone_new(identifier->str);

	if (!*timezone) {
		_error(self, first, SCARAB_SYNTAX_ERROR_BAD_LITERAL, g_strdup_printf("Unknown timezone in date/time: %s", identifier->str));
	}

	g_string_free(identifier, TRUE);
	scarab_token_free(first);

	return true;
}

static bool _parse_value(ScarabParserContext *self, GValue *value) {
	ScarabToken *token;
	REQUIRE(_read(self, &token));

	int sign = 1;
	
	retry:
	switch ((int) token->type) {
		case '-':
			sign = -1;
			scarab_token_free(token);

			REQUIRE(_read(self, &token));
			EXPECT(T_NUMBER, T_LONGINTEGER, T_DAYS, T_TIME_PART);

			goto retry;

		case T_NUMBER:
		case T_LONGINTEGER:
			return _parse_number(self, value, token, sign);

		case T_DATE_PART:
			return _parse_datetime(self, value, token);

		case T_DAYS:
		case T_TIME_PART:
			return _parse_timespan(self, value, token, sign);

		case T_BOOLEAN:
			g_value_init(value, G_TYPE_BOOLEAN);

			if (strcmp(token->val, "true") == 0 || strcmp(token->val, "on") == 0) {
				g_value_set_boolean(value, TRUE);
			} else if (strcmp(token->val, "false") == 0 || strcmp(token->val, "off") == 0) {
				g_value_set_boolean(value, FALSE);
			}

			break;

		case T_NULL:
			g_value_init(value, G_TYPE_POINTER);
			g_value_set_pointer(value, NULL);
			break;

		case T_STRING:
			g_value_init(value, G_TYPE_STRING);
			g_value_set_string(value, token->val);
			break;

		case T_CHAR:
			g_value_init(value, SCARAB_TYPE_UNICHAR);
			scarab_gvalue_set_unichar(value, g_utf8_get_char(token->val));
			break;

		case T_BINARY:
			g_value_init(value, SCARAB_TYPE_BINARY);

			gsize len;
			guchar *data = g_base64_decode(token->val, &len);
			scarab_gvalue_take_binary(value, g_byte_array_new_take(data, len));

			break;

		default:
			g_return_val_if_reached(false);
	}

	scarab_token_free(token);
	return true;
}

static void _str_ptr_unset(gchar **value) {
	g_free(*value);
}

static void _value_ptr_unset(GValue **value) {
	g_value_unset(*value);
}

static bool _parse_tag(ScarabParserContext *self) {
	ScarabToken *first, *token;
	char *name = g_strdup("content");

	GArray *values = g_array_new(TRUE, FALSE, sizeof(GValue*));
	GArray *attr_names = g_array_new(TRUE, FALSE, sizeof(gchar*));
	GArray *attr_values = g_array_new(TRUE, FALSE, sizeof(GValue*));

	g_array_set_clear_func(values, (GDestroyNotify) _value_ptr_unset);
	g_array_set_clear_func(attr_names, (GDestroyNotify) _str_ptr_unset);
	g_array_set_clear_func(attr_values, (GDestroyNotify) _value_ptr_unset);

	REQUIRE(_peek(self, &first));

	if (first->type == T_IDENTIFIER) {
		_consume(self);

		REQUIRE(_peek(self, &token));

		if (token->type == '=') {
			_error(
				self,
				first,
				SCARAB_SYNTAX_ERROR_MALFORMED,
				"At least one value required for an anonymous tag"
			);

			return false;
		}

		g_free(name);
		name = g_strdup(first->val);
		scarab_token_free(first);
	} else {
		token = first;

		EXPECT(T_IDENTIFIER, T_NUMBER, T_TIME_PART, T_DATE_PART, T_LONGINTEGER, T_DAYS, T_BOOLEAN, T_NULL, T_STRING, T_CHAR, T_BINARY);
	}

	bool peek_success = true;

	while ((_peek(self, &token) || (peek_success = false)) && _token_is_value(token)) {
		GValue *value = g_slice_new0(GValue);
		REQUIRE(_parse_value(self, value));
		g_array_append_val(values, value);
	}
	REQUIRE(peek_success);

	while ((_peek(self, &token) || (peek_success = false)) && token->type == T_IDENTIFIER) {
		_consume(self);
		char *contents = g_strdup(token->val);
		g_array_append_val(attr_names, contents);
		scarab_token_free(token);

		REQUIRE(_read(self, &token));
		EXPECT('=');
		scarab_token_free(token);

		GValue *value = g_slice_new0(GValue);
		REQUIRE(_parse_value(self, value));
		g_array_append_val(attr_values, value);
	}
	REQUIRE(peek_success);

	GError *err = NULL;
	MAYBE_CALLBACK(self->parser->start_tag,
		self,
		name,
		(GValue**) values->data,
		(gchar**) attr_names->data,
		(GValue**) attr_values->data,
		self->user_data,
		&err
	);
	if (err) {
		MAYBE_CALLBACK(self->parser->error, self, err, self->user_data);
		return false;
	}

	g_array_free(values, TRUE);
	g_array_free(attr_names, TRUE);
	g_array_free(attr_values, TRUE);

	REQUIRE(_peek(self, &token));

	if (token->type == '{') {
		_consume(self);
		scarab_token_free(token);

		while ((_peek(self, &token) || (peek_success = false)) && token->type != '}') {
			if (token->type == '\n') {
				_consume(self);
				continue;
			}

			REQUIRE(_parse_tag(self));
			REQUIRE(_peek(self, &token));
			EXPECT('\n', ';', '}');

			if (token->type != '}') {
				_consume(self);
				scarab_token_free(token);
			}
		}

		EXPECT('}');
		_consume(self);
		scarab_token_free(token);
	}

	err = NULL;
	MAYBE_CALLBACK(self->parser->end_tag,
		self,
		name,
		self->user_data,
		&err
	);
	if (err) {
		MAYBE_CALLBACK(self->parser->error, self, err, self->user_data);
		return false;
	}

	return true;
}

static ScarabValue* _parse_open_list(ScarabParserContext *self, ScarabTokenType terminator, GError **err) {
	ScarabValue *result = scarab_nil;

	ScarabToken *token;
	do {
		REQUIRE(_peek(self, &token));
		ScarabValue *new_value;

		if (token->type == T_NUMBER) {
			REQUIRE(m
		}
	} while(

	return result;
}

static ScarabValue* _parse(ScarabParserContext *self, GError **err) {
	return parse_open_list(self, T_EOF, err);
}

ScarabValue* scarab_parse_string(const char *str, GError **err) {
	ScarabParserContext *self = g_slice_new0(ScarabParserContext);
	self->tokenizer = scarab_tokenizer_new_from_string(str, err);

	if (!self->tokenizer) {
		return NULL;
	}

	return _parse(self, err);
}

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

#ifndef __TOKENIZER_H__
#define __TOKENIZER_H__

#include <glib.h>
#include <stdbool.h>
#include <stdio.h>

//> Types
/**
 * KhTokenType:
 * @T_EOF: A virtual token at the end of the input.*
 * @T_IDENTIFIER: An Unicode identifier.
 * @T_NUMBER: A sequence of ASCII digits.
 * @T_DECIMAL: A sequence of digits containing a period.
 * @T_STRING: A string.
 *
 * * These token types have no value, and their #KhToken.val field is undefined.
 */
typedef enum {
	T_EOF = EOF,
	T_IDENTIFIER = 256,
	T_NUMBER,
	T_DECIMAL,
	T_STRING,
} KhTokenType;

/**
 * KhToken:
 * @type: The type of the token, either one of %KhTokenType or an ASCII character in the range
 *        0-255.
 * @line: The line where the token occurred.
 * @col: The column where the token occurred.
 * @val: Any string contents of the token, as a %NULL-terminated string. This is undefined for any
 *       single-character token, and %T_EOF and %T_NULL.
 */
typedef struct {
	KhTokenType type;

	guint line;
	guint col;

	char *val;
} KhToken;

typedef struct _KhTokenizer KhTokenizer;

//> Exported Functions
extern KhTokenizer* kh_tokenizer_new(const char *filename, GError **err);
extern KhTokenizer* kh_tokenizer_new_from_string(const char *str, GError **err);

extern bool kh_tokenizer_next(KhTokenizer *self, KhToken **token, GError **err);
extern char* kh_tokenizer_get_filename(KhTokenizer *self);

extern void kh_tokenizer_free(KhTokenizer *self);

extern char* kh_token_type_name(KhTokenType token_type);
extern void kh_token_free(KhToken *token);

#endif

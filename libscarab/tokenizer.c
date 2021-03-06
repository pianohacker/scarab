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

// Tokenizer for Scarab code.
//
// This may at some point be replaced by an re2c implementation.

#include <ctype.h>
#include <gc.h>
#include <glib.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#include "error.h"
#include "strfuncs.h"
#include "tokenizer.h"
#include "util.h"

// Quick way of stopping and passing an error back up the stack:
#define FAIL_IF_ERR() if ((err != NULL) && (*err != NULL)) return false;

// This is used for reading in strings of unknown length.
// `str` and `alloc` are fairly self explanatory, but `i` is one beyond the largest index that the
// code might need to index within the string (so i+1 for ASCII, i+5 for UTF-8). This is subtle, and
// can get you in trouble if you're not careful.
#define GROW_IF_NEEDED(str, i, alloc) if (i >= alloc) { alloc = alloc ? (alloc * 2 + 1) : 7; str = GC_REALLOC(str, alloc); }

static char *TOKEN_NAMES[4] = {
	"identifier",
	"number",
	"decimal",
	"string",
};

// # Tokenizer
// Most of this is prosaic state information.
struct _KhTokenizer {
	char *filename;
	GIOChannel *channel;
	const char *str_base;
	const char *str;
	
	int line;
	int col;

	// This controls the peek mechanism.
	bool peek_avail;
	gunichar peeked;
};

// GC finalizer for a tokenizer.
void _tokenizer_finalize(KhTokenizer *self, void *data) {
	if (self->channel) {
		g_io_channel_shutdown(self->channel, FALSE, NULL);
		g_io_channel_unref(self->channel);
	}
}


// ## Public API

/**
 * kh_tokenizer_new:
 * @filename: Name of file to be parsed.
 * @err: Return location for a %GError to be set on failure, may be NULL.
 *
 * Creates a new tokenizer consuming the given file.
 *
 * Returns: A new %KhTokenizer, or NULL on failure.
 */
KhTokenizer* kh_tokenizer_new(const char *filename, GError **err) {
	KhTokenizer *self = GC_NEW(KhTokenizer);
	GC_REGISTER_FINALIZER(self, (GC_finalization_proc) _tokenizer_finalize, NULL, NULL, NULL);
	self->filename = GC_STRDUP(filename);
	self->channel = g_io_channel_new_file(filename, "r", err);

	if (!self->channel) return NULL;

	self->str_base = self->str = NULL;
	self->line = 1;
	self->col = 1;
	self->peek_avail = false;

	return self;
}

/**
 * kh_tokenizer_new_from_string:
 * @str: String to be parsed.
 * @err: Return location for a %GError to be set on failure, may be NULL.
 *
 * Creates a new tokenizer consuming the given string. The filename will be set to "&lt;string&gt;".
 *
 * Returns: A new %KhTokenizer, or NULL on failure.
 */
KhTokenizer* kh_tokenizer_new_from_string(const char *str, GError **err) {
	KhTokenizer *self = GC_NEW(KhTokenizer);
	GC_REGISTER_FINALIZER(self, (GC_finalization_proc) _tokenizer_finalize, NULL, NULL, NULL);
	self->filename = "<string>";
	self->str_base = self->str = str;

	if (!g_utf8_validate(self->str, -1, NULL)) {
		g_set_error(
			err,
			G_CONVERT_ERROR,
			G_CONVERT_ERROR_ILLEGAL_SEQUENCE,
			"Invalid UTF-8 sequence in string"
		);
		return NULL;
	}

	self->channel = NULL;
	self->line = 1;
	self->col = 1;
	self->peek_avail = false;

	return self;
}

/**
 * kh_tokenizer_get_filename:
 * @self: A valid %KhTokenizer.
 *
 * Returns: the name of the file being parsed by this %KhTokenizer. May be "&lt;string&gt;".
 */
char* kh_tokenizer_get_filename(KhTokenizer *self) {
	return self->filename;
}

/**
 * kh_token_type_name:
 * @token_type: A valid %KhTokenType.
 *
 * Returns: a string representing the token type. For simple, one-character tokens, this will be
 *          something like "'='". Longer tokens will have a simple phrase, like "date part".
 */
extern char* kh_token_type_name(KhTokenType token_type) {
	static char buffer[4] = "' '";

	if (token_type == '\n') {
		return "'\\n'";
	} else if (0 <= token_type && token_type < T_MIN_TOKEN) {
		buffer[1] = token_type;
		return buffer;
	} else {
		return token_type == EOF ? "EOF" : TOKEN_NAMES[token_type - T_MIN_TOKEN];
	}
}

//> Internal Functions
/*
 * _read:
 * @self: A valid %KhTokenizer.
 * @result: (out): The location to store the resulting %gunichar.
 * @err: (out) (allow-none): Location to store any %GError, or %NULL.
 *
 * Reads a single UTF-8 character from the input. Will return %EOF once at the end of the input.
 *
 * Returns: Whether the read succeeded.
 */
static bool _read(KhTokenizer *self, gunichar *result, GError **err) {
	if (self->peek_avail) {
		*result = self->peeked;
		self->peek_avail = false;
	} else if (self->str) {
		if (!*self->str) {
			self->str = NULL;
			*result = EOF;
		} else {
			*result = g_utf8_get_char(self->str);
			self->str = g_utf8_next_char(self->str);
		}

		return true;
	} else {
		if (G_UNLIKELY(!self->channel)) return false;

		switch (g_io_channel_read_unichar(self->channel, result, err)) {
			case G_IO_STATUS_ERROR:
				self->channel = NULL;
				self->peek_avail = false;
				return false;
			case G_IO_STATUS_EOF:
				self->peek_avail = false;
				*result = EOF;
				g_io_channel_shutdown(self->channel, FALSE, NULL);
				g_io_channel_unref(self->channel);
				self->channel = NULL;
				return true;
			case G_IO_STATUS_AGAIN:
			case G_IO_STATUS_NORMAL:
				break;
		}
	}

	if (*result == '\n') {
		self->line++;
		self->col = 1;
	} else {
		self->col++;
	}

	return true;
}

/*
 * _peek:
 * @self: A valid %KhTokenizer.
 * @result: (out): The location to store the resulting %gunichar.
 * @err: (out) (allow-none): Location to store any %GError, or %NULL.
 *
 * Looks at the next UTF-8 character from the input. Will return %EOF at the end of the input.
 *
 * Returns: Whether the peek succeeded. This should always be checked, as _peek() may or may not
 *          have to read from the input.
 */
static bool _peek(KhTokenizer *self, gunichar *result, GError **err) {
	if (!self->peek_avail) {
		if (self->str) {
			if (*self->str) {
				self->peeked = g_utf8_get_char(self->str);
				self->str = g_utf8_next_char(self->str);
			} else {
				self->str = NULL;
				self->peeked = EOF;
			}
			self->peek_avail = true;
		} else {
			if (self->channel == NULL) return false;

			switch (g_io_channel_read_unichar(self->channel, &(self->peeked), err)) {
				case G_IO_STATUS_ERROR:
					self->channel = NULL;
					self->peek_avail = false;
					return false;
				case G_IO_STATUS_EOF:
					self->peeked = EOF;
					g_io_channel_shutdown(self->channel, FALSE, NULL);
					g_io_channel_unref(self->channel);
					self->channel = NULL;
				case G_IO_STATUS_AGAIN:
				case G_IO_STATUS_NORMAL:
				default:
					self->peek_avail = true;
			}
		}
	}

	*result = self->peeked;
	return true;
}

/*
 * _consume:
 * @self: A valid %KhTokenizer.
 *
 * Throws away the current lookahead character. Useful after a _peek(), when the character is known,
 * but needs to be moved past.
 */
static void _consume(KhTokenizer *self) {
	g_assert(self->peek_avail);

	gunichar result;
	_read(self, &result, NULL);
}

/*
 * _maketoken:
 * @result: (out) A %KhToken to fill in.
 * @type: A valid %KhTokenType.
 * @line: Line where the token occurred.
 * @col: Column of the start of the token.
 *
 * Returns: A newly-allocated %KhToken with the given information.
 */
static void _maketoken(KhToken *result, KhTokenType type, int line, int col) {
	result->type = type;
	result->line = line;
	result->col = col;
}

/*
 * _set_error:
 * @err: (out) (allow-none): Output location of the %GError passed to the calling function.
 * @self: A valid %KhTokenizer.
 * @err_type: Kind of %KhSyntaxError to set.
 * @msg: Message to set on the %GError. Will be appended with the filename, line and column that the
 *       tokenizer is currently at.
 *
 * Sets a %GError in the %KH_SYNTAX_ERROR domain.
 */
static void _set_error(GError **err, KhTokenizer *self, KhSyntaxError err_type, char *msg) {
	g_set_error(err,
		KH_SYNTAX_ERROR,
		err_type,
		"%s in %s, line %d, column %d",
		msg,
		self->filename,
		self->line,
		self->col
	);
}

//> Sub-tokenizers
static bool _tokenize_number(KhTokenizer *self, KhToken *result, gunichar c, GError **err) {
	char *output = result->val;

	GROW_IF_NEEDED(output = result->val, 1, result->val_length);
	output[0] = c;
	int i = 1;

	while (_peek(self, &c, err) && c < 256 && isdigit(c)) {
		GROW_IF_NEEDED(output = result->val, i + 1, result->val_length);

		_consume(self);
		output[i++] = (gunichar) c;
	}

	FAIL_IF_ERR();

	char *suffix = output + i;

	while (_peek(self, &c, err) && c < 256 && (isalpha(c) || isdigit(c))) {
		GROW_IF_NEEDED(output = result->val, i + 1, result->val_length);

		_consume(self);
		output[i++] = (gunichar) c;
	}

	FAIL_IF_ERR();

	output[i] = '\0';

	*suffix = '\0';

	return true;
}

static bool _tokenize_identifier(KhTokenizer *self, KhToken *result, gunichar c, GError **err) {
	char *output = result->val;

	GROW_IF_NEEDED(output = result->val, 5, result->val_length);
	int i = g_unichar_to_utf8(c, output);

	while (_peek(self, &c, err) &&
			!(c < 256 && strchr(KH_TOKENIZER_SPECIAL_PUNCT, (char) c)) &&
			(c == '_' || c == '-' || g_unichar_isalpha(c) || g_unichar_isdigit(c) || g_unichar_ispunct(c))
		) {
		GROW_IF_NEEDED(output = result->val, i + 5, result->val_length);

		_consume(self);
		i += g_unichar_to_utf8(c, output + i);
	}

	FAIL_IF_ERR();
	output[i] = '\0';

	return true;
}

static bool _tokenize_string(KhTokenizer *self, KhToken *result, GError **err) {
	gunichar c;
	char *output = result->val;
	int i = 0;

	while (_peek(self, &c, err) && c != '"' && c != EOF) {
		GROW_IF_NEEDED(output = result->val, i + 5, result->val_length);

		_consume(self);

		if (c == '\\') {
			_read(self, &c, err);

			switch (c) {
				case 'n': output[i++] = '\n'; break;
				case 'r': output[i++] = '\r'; break;
				case 't': output[i++] = '\t'; break;
				case '"': output[i++] = '"'; break;
				case '\'': output[i++] = '\"'; break;
				case '\\': output[i++] = '\\'; break;
				case '\r':
					_read(self, &c, err);
				case '\n':
					output[i++] = '\n';
					while (_peek(self, &c, err) && (c == ' ' || c == '\t')) _consume(self);
					break;
				default:
					i += g_unichar_to_utf8(c, output + i);
			}
		} else {
			i += g_unichar_to_utf8(c, output + i);
		}
	}

	FAIL_IF_ERR();
	output[i] = '\0';

	return true;
}

static bool _tokenize_backquote_string(KhTokenizer *self, KhToken *result, GError **err) {
	gunichar c;
	char *output = result->val;
	int i = 0;

	while (_peek(self, &c, err) && c != '`' && c != EOF) {
		GROW_IF_NEEDED(output = result->val, i + 5, result->val_length);

		_consume(self);

		if (c == '\r') _read(self, &c, err);

		i += g_unichar_to_utf8(c, output + i);
	}

	FAIL_IF_ERR();
	output[i] = '\0';

	return true;
}

/**
 * kh_tokenizer_next:
 * @self: A valid %KhTokenizer.
 * @result: (out callee-allocates): A %KhToken to initialize and fill in.
 * @err: (out) (allow-none): Location to store any error, may be %NULL.
 *
 * Fetches the next token from the input. Depending on the source of input, may set an error in one
 * of the %KH_SYNTAX_ERROR, %G_IO_CHANNEL_ERROR, or %G_CONVERT_ERROR domains.
 *
 * Returns: Whether a token could be successfully read.
 */
bool kh_tokenizer_next(KhTokenizer *self, KhToken *result, GError **err) {
	gunichar c, nc;
	int line;
	int col;

	retry:
	line = self->line;
	col = self->col;
	if (!_read(self, &c, err)) return false;

	if (G_UNLIKELY(c == EOF)) {
		_maketoken(result, T_EOF, line, col);
		return true;
	} else if (c == '#') {
		_consume(self);
		while (_peek(self, &c, err) && !(c == '\n' || c == EOF)) _consume(self);

		_maketoken(result, '\n', line, col);
		return true;
	} else if (c == '-' && _peek(self, &nc, err) && nc < 256 && isdigit(nc)) {
		_maketoken(result, T_NUMBER, line, col);
		return _tokenize_number(self, result, c, err);
	} else if (c < 256 && strchr(KH_TOKENIZER_SPECIAL_PUNCT, (char) c)) {
		_maketoken(result, c, line, col);
		return true;
	} else if (c < 256 && isdigit((char) c)) {
		_maketoken(result, T_NUMBER, line, col);
		return _tokenize_number(self, result, c, err);
	} else if (c == '"') {
		_maketoken(result, T_STRING, line, col);
		if (!_tokenize_string(self, result, err)) return false;

		_REQUIRE(_read(self, &c, err));
		if (c == '"') {
			return true;
		} else {
			_set_error(err,
				self,
				KH_SYNTAX_ERROR_MISSING_DELIMITER,
				"Missing '\"'"
			);
			return false;
		}
	} else if (c == '`') {
		_maketoken(result, T_STRING, line, col);
		if (!_tokenize_backquote_string(self, result, err)) return false;

		_REQUIRE(_read(self, &c, err));
		if (c == '`') {
			return true;
		} else {
			_set_error(err,
				self,
				KH_SYNTAX_ERROR_MISSING_DELIMITER,
				"Missing '`'"
			);
			return false;
		}
	} else if (c == ' ' || c == '\t' || c == '\r') {
		// Do nothing
		goto retry;
	} else if (g_unichar_isalpha(c) || g_unichar_ispunct(c)) {
		_maketoken(result, T_IDENTIFIER, line, col);
		return _tokenize_identifier(self, result, c, err);
	} else {
		_set_error(err,
			self,
			KH_SYNTAX_ERROR_UNEXPECTED_CHAR,
		   	kh_strdupf("Invalid character '%s'(%d)", g_ucs4_to_utf8(&c, 1, NULL, NULL, NULL), c)
		);
		return false;
	}
}

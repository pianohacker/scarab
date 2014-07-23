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

#include <ctype.h>
#include <glib.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#include "error.h"
#include "tokenizer.h"

//> Macros
#define FAIL_IF_ERR() if ((err != NULL) && (*err != NULL)) return false;
#define GROW_IF_NEEDED(str, i, alloc) if (i >= alloc) { alloc = alloc * 2 + 1; str = g_realloc(str, alloc); }
#define REQUIRE(expr) if (!expr) return false;

//> Internal Types
struct _ScarabTokenizer {
	char *filename;
	GIOChannel *channel;
	gunichar *stringbuf;
	
	int line;
	int col;

	bool peek_avail;
	gunichar peeked;
};

//> Static Data
static char *TOKEN_NAMES[15] = {
	"EOF",
	"identifier",
	"number",
	"decimal",
	"string",
};

//> Public Functions

/**
 * scarab_tokenizer_new:
 * @filename: Name of file to be parsed.
 * @err: Return location for a %GError to be set on failure, may be NULL.
 *
 * Creates a new tokenizer consuming the given file.
 *
 * Returns: A new %ScarabTokenizer, or NULL on failure.
 */
ScarabTokenizer* scarab_tokenizer_new(const char *filename, GError **err) {
	ScarabTokenizer* self = g_slice_new0(ScarabTokenizer);
	self->filename = g_strdup(filename);
	self->channel = g_io_channel_new_file(filename, "r", err);

	if (!self->channel) return NULL;

	self->stringbuf = NULL;
	self->line = 1;
	self->col = 1;
	self->peek_avail = false;

	return self;
}

/**
 * scarab_tokenizer_new_from_string:
 * @str: String to be parsed.
 * @err: Return location for a %GError to be set on failure, may be NULL.
 *
 * Creates a new tokenizer consuming the given string. The filename will be set to "&lt;string&gt;".
 *
 * Returns: A new %ScarabTokenizer, or NULL on failure.
 */
ScarabTokenizer* scarab_tokenizer_new_from_string(const char *str, GError **err) {
	ScarabTokenizer* self = g_slice_new0(ScarabTokenizer);
	self->filename = "<string>";
	self->stringbuf = g_utf8_to_ucs4(str, -1, NULL, NULL, err);

	if (!self->stringbuf) return NULL;

	self->channel = NULL;
	self->line = 1;
	self->col = 1;
	self->peek_avail = false;

	return self;
}

/**
 * scarab_tokenizer_get_filename:
 * @self: A valid %ScarabTokenizer.
 *
 * Returns: the name of the file being parsed by this %ScarabTokenizer. May be "&lt;string&gt;".
 */
char* scarab_tokenizer_get_filename(ScarabTokenizer *self) {
	return self->filename;
}

/**
 * scarab_tokenizer_free:
 * @self: A valid %ScarabTokenizer.
 *
 * Frees this %ScarabTokenizer and all resources associated with it.
 */
void scarab_tokenizer_free(ScarabTokenizer *self) {
	g_free(self->filename);

	if (self->channel) {
		g_io_channel_shutdown(self->channel, FALSE, NULL);
		g_io_channel_unref(self->channel);
	}

	if (self->stringbuf) g_free(self->stringbuf);

	g_slice_free(ScarabTokenizer, self);
}

/**
 * scarab_token_type_name:
 * @token_type: A valid %ScarabTokenType.
 *
 * Returns: a string representing the token type. For simple, one-character tokens, this will be
 *          something like "'='". Longer tokens will have a simple phrase, like "date part".
 */
extern char* scarab_token_type_name(ScarabTokenType token_type) {
	static char buffer[4] = "' '";

	if (0 <= token_type && token_type < 256) {
		buffer[1] = token_type;
		return buffer;
	} else {
		return TOKEN_NAMES[token_type == EOF ? 0 : (token_type - 255)];
	}
}

/**
 * scarab_token_free:
 * @token: A valid %ScarabToken.
 *
 * Should be called to free a token and its contents once the parser is done with it.
 */
extern void scarab_token_free(ScarabToken *token) {
	if (token->val) g_free(token->val);

	g_slice_free(ScarabToken, token);
}

//> Internal Functions
/*
 * _read:
 * @self: A valid %ScarabTokenizer.
 * @result: (out): The location to store the resulting %gunichar.
 * @err: (out) (allow-none): Location to store any %GError, or %NULL.
 *
 * Reads a single UTF-8 character from the input. Will return %EOF once at the end of the input.
 *
 * Returns: Whether the read succeeded.
 */
static bool _read(ScarabTokenizer *self, gunichar *result, GError **err) {
	if (self->peek_avail) {
		*result = self->peeked;
		self->peek_avail = false;
	} else if (self->stringbuf) {
		if (!*self->stringbuf) {
			self->stringbuf = NULL;
			*result = EOF;
		} else {
			*result = *(self->stringbuf++);
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
 * @self: A valid %ScarabTokenizer.
 * @result: (out): The location to store the resulting %gunichar.
 * @err: (out) (allow-none): Location to store any %GError, or %NULL.
 *
 * Looks at the next UTF-8 character from the input. Will return %EOF at the end of the input.
 *
 * Returns: Whether the peek succeeded. This should always be checked, as _peek() may or may not
 *          have to read from the input.
 */
static bool _peek(ScarabTokenizer *self, gunichar *result, GError **err) {
	if (!self->peek_avail) {
		if (self->stringbuf) {
			if (*self->stringbuf) {
				self->peeked = *(self->stringbuf++);
			} else {
				self->stringbuf = NULL;
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
 * @self: A valid %ScarabTokenizer.
 *
 * Throws away the current lookahead character. Useful after a _peek(), when the character is known,
 * but needs to be moved past.
 */
static void _consume(ScarabTokenizer *self) {
	g_assert(self->peek_avail);

	gunichar result;
	_read(self, &result, NULL);
}

/*
 * _maketoken:
 * @type: A valid %ScarabTokenType.
 * @line: Line where the token occurred.
 * @col: Column of the start of the token.
 *
 * Returns: A newly-allocated %ScarabToken with the given information.
 */
static ScarabToken* _maketoken(ScarabTokenType type, int line, int col) {
	ScarabToken *result = g_slice_new0(ScarabToken);

	result->type = type;
	result->line = line;
	result->col = col;

	return result;
}

/*
 * _maketoken:
 * @err: (out) (allow-none): Output location of the %GError passed to the calling function.
 * @self: A valid %ScarabTokenizer.
 * @err_type: Kind of %ScarabSyntaxError to set.
 * @msg: Message to set on the %GError. Will be appended with the filename, line and column that the
 *       tokenizer is currently at.
 *
 * Sets a %GError in the %SCARAB_SYNTAX_ERROR domain.
 */
static void _set_error(GError **err, ScarabTokenizer *self, ScarabSyntaxError err_type, char *msg) {
	g_set_error(err,
		SCARAB_SYNTAX_ERROR,
		err_type,
		"%s in %s, line %d, column %d",
		msg,
		self->filename,
		self->line,
		self->col
	);
}

//> Sub-tokenizers
static bool _tokenize_number(ScarabTokenizer *self, ScarabToken *result, gunichar c, GError **err) {
	int length = 7;
	char *output = result->val = g_malloc(length);

	output[0] = c;
	int i = 1;

	while (_peek(self, &c, err) && c < 256 && isdigit(c)) {
		GROW_IF_NEEDED(output = result->val, i + 1, length);

		_consume(self);
		output[i++] = (gunichar) c;
	}

	FAIL_IF_ERR();

	char *suffix = output + i;

	while (_peek(self, &c, err) && c < 256 && (isalpha(c) || isdigit(c))) {
		GROW_IF_NEEDED(output = result->val, i + 1, length);

		_consume(self);
		output[i++] = (gunichar) c;
	}

	FAIL_IF_ERR();

	output[i] = '\0';

	*suffix = '\0';

	return true;
}

static bool _tokenize_identifier(ScarabTokenizer *self, ScarabToken *result, gunichar c, GError **err) {
	int length = 7;
	char *output = result->val = g_malloc(length);

	int i = g_unichar_to_utf8(c, output);

	while (_peek(self, &c, err) && (c == '_' || c == '-' || g_unichar_isalpha(c) || g_unichar_isdigit(c))) {
		GROW_IF_NEEDED(output = result->val, i + 5, length);

		_consume(self);
		i += g_unichar_to_utf8(c, output + i);
	}

	FAIL_IF_ERR();
	output[i] = '\0';

	return true;
}

static bool _tokenize_string(ScarabTokenizer *self, ScarabToken *result, GError **err) {
	int length = 7;
	gunichar c;
	char *output = result->val = g_malloc(length);
	int i = 0;

	while (_peek(self, &c, err) && c != '"' && c != EOF) {
		GROW_IF_NEEDED(output = result->val, i, length);

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

static bool _tokenize_backquote_string(ScarabTokenizer *self, ScarabToken *result, GError **err) {
	int length = 7;
	gunichar c;
	char *output = result->val = g_malloc(length);
	int i = 0;

	while (_peek(self, &c, err) && c != '`' && c != EOF) {
		GROW_IF_NEEDED(output = result->val, i, length);

		_consume(self);

		if (c == '\r') _read(self, &c, err);

		i += g_unichar_to_utf8(c, output + i);
	}

	FAIL_IF_ERR();
	output[i] = '\0';

	return true;
}

/**
 * scarab_tokenizer_next:
 * @self: A valid %ScarabTokenizer.
 * @result: (out callee-allocates): A %ScarabToken to initialize and fill in.
 * @err: (out) (allow-none): Location to store any error, may be %NULL.
 *
 * Fetches the next token from the input. Depending on the source of input, may set an error in one
 * of the %SCARAB_SYNTAX_ERROR, %G_IO_CHANNEL_ERROR, or %G_CONVERT_ERROR domains.
 *
 * Returns: Whether a token could be successfully read.
 */
bool scarab_tokenizer_next(ScarabTokenizer *self, ScarabToken **result, GError **err) {
	gunichar c, nc;
	int line;
	int col;

	retry:
	line = self->line;
	col = self->col;
	if (!_read(self, &c, err)) return false;

	if (G_UNLIKELY(c == EOF)) {
		*result = _maketoken(T_EOF, line, col);
		return true;
	} else if (c == '#') {
		_consume(self);
		while (_peek(self, &c, err) && !(c == '\n' || c == EOF)) _consume(self);

		goto retry;
	} else if (c == '-' && _peek(self, &nc, err) && nc < 256 && isdigit(nc)) {
		*result = _maketoken(T_NUMBER, line, col);
		return _tokenize_number(self, *result, c, err);
	} else if (c < 256 && strchr(",{}()[]", (char) c)) {
		*result = _maketoken(c, line, col);
		return true;
	} else if (c < 256 && isdigit((char) c)) {
		*result = _maketoken(T_NUMBER, line, col);
		return _tokenize_number(self, *result, c, err);
	} else if (c == '"') {
		*result = _maketoken(T_STRING, line, col);
		if (!_tokenize_string(self, *result, err)) return false;

		REQUIRE(_read(self, &c, err));
		if (c == '"') {
			return true;
		} else {
			_set_error(err,
				self,
				SCARAB_SYNTAX_ERROR_MISSING_DELIMITER,
				"Missing '\"'"
			);
			return false;
		}
	} else if (c == '`') {
		*result = _maketoken(T_STRING, line, col);
		if (!_tokenize_backquote_string(self, *result, err)) return false;

		REQUIRE(_read(self, &c, err));
		if (c == '`') {
			return true;
		} else {
			_set_error(err,
				self,
				SCARAB_SYNTAX_ERROR_MISSING_DELIMITER,
				"Missing '`'"
			);
			return false;
		}
	} else if (c == ' ' || c == '\t' || c == '\r' || c == '\n') {
		// Do nothing
		goto retry;
	} else if (g_unichar_isalpha(c) || g_unichar_ispunct(c)) {
		*result = _maketoken(T_IDENTIFIER, line, col);
		return _tokenize_identifier(self, *result, c, err);
	} else {
		_set_error(err,
			self,
			SCARAB_SYNTAX_ERROR_UNEXPECTED_CHAR,
		   	g_strdup_printf("Invalid character '%s'(%d)", g_ucs4_to_utf8(&c, 1, NULL, NULL, NULL), c)
		);
		return false;
	}
}

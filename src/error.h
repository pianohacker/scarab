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

#ifndef __ERROR_H__
#define __ERROR_H__

#include <glib.h>

#define SCARAB_SYNTAX_ERROR scarab_syntax_error_quark()

/**
 * GSDLSyntaxError:
 * @SCARAB_SYNTAX_ERROR_UNEXPECTED_CHAR: An unexpected character was found while reading the source file.
 * @SCARAB_SYNTAX_ERROR_MISSING_DELIMITER: Did not find the end of a string or binary literal before
 *                                       the end of the file.
 * @SCARAB_SYNTAX_ERROR_MALFORMED: Bad syntax; unexpected token in the input.
 * @SCARAB_SYNTAX_ERROR_BAD_LITERAL: Bad formatting inside a literal, or out of range value.
 * @SCARAB_SYNTAX_ERROR_UNEXPECTED_TAG: Parser handler found an unexpected tag.
 * @SCARAB_SYNTAX_ERROR_MISSING_VALUE: Parser handler was missing a required attribute or value.
 * @SCARAB_SYNTAX_ERROR_BAD_TYPE: Parser handler found a value that could not be converted to the
 *                              required type.
 * 
 * The last three errors are intended to be used by %GSDLParser parser callbacks.
 */
typedef enum {
	SCARAB_SYNTAX_ERROR_UNEXPECTED_CHAR,
	SCARAB_SYNTAX_ERROR_MISSING_DELIMITER,
	SCARAB_SYNTAX_ERROR_MALFORMED,
	SCARAB_SYNTAX_ERROR_BAD_LITERAL,
	SCARAB_SYNTAX_ERROR_UNEXPECTED_TAG,
	SCARAB_SYNTAX_ERROR_MISSING_VALUE,
	SCARAB_SYNTAX_ERROR_BAD_TYPE,
} ScarabSyntaxError;

extern GQuark scarab_syntax_error_quark();

#endif

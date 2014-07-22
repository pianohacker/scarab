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

#ifndef __PARSER_H__
#define __PARSER_H__

#include <glib.h>
#include <glib-object.h>
#include <stdbool.h>

/**
 * GSDLParserContext:
 *
 * All fields in GSDLParserContext are private.
 */
typedef struct _GSDLParserContext GSDLParserContext;

/**
 * GSDLParser:
 * @start_tag: Callback to invoke when a new element is entered. This is called for empty tags. 
 *             %values, %attr_names and %attr_values are %NULL-terminated arrays.
 * @end_tag: Callback to invoke at the end of an element.
 * @error: Callback to invoke when an error occurs. The error will be of type %G_CONVERT_ERROR,
 *         %G_IO_CHANNEL_ERROR or %GSDL_SYNTAX_ERROR.
 *
 * A set of parsing callbacks.
 *
 * Note: the %start_tag and %end_tag callbacks can optionally set an error, which will cause the
 * %error callback to be called with that error and parsing to immediately stop.
 */

typedef struct {
	void (*start_tag)(
		GSDLParserContext *context,
		const gchar *name,
		GValue* const *values,
		gchar* const *attr_names,
		GValue* const *attr_values,
		gpointer user_data,
		GError **err
	);

	void (*end_tag)(
		GSDLParserContext *context,
		const gchar *name,
		gpointer user_data,
		GError **err
	);

	void (*error)(
		GSDLParserContext *context,
		GError *err,
		gpointer user_data
	);

} GSDLParser;

#define GSDL_GTYPE_ANY 1L << (sizeof(GType) * 8 - 1)
#define GSDL_GTYPE_END 0L
#define GSDL_GTYPE_OPTIONAL 1L << (sizeof(GType) * 8 - 2)

extern GSDLParserContext* gsdl_parser_context_new(GSDLParser *parser, gpointer user_data);

extern void gsdl_parser_context_push(GSDLParserContext *self, GSDLParser *parser, gpointer user_data);
extern gpointer gsdl_parser_context_pop(GSDLParserContext *self);

extern bool gsdl_parser_context_parse_file(GSDLParserContext *self, const char *filename);
extern bool gsdl_parser_context_parse_string(GSDLParserContext *self, const char *str);

extern bool gsdl_parser_collect_values(const gchar *name, GValue* const *values, GError **err, GType first_type, GValue **first_value, ...);
extern bool gsdl_parser_collect_attributes(const gchar *name, gchar* const *attr_names, GValue* const *attr_values, GError **err, GType first_type, const gchar *first_name, GValue **first_value, ...);

#endif

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

#include <gc.h>
#include <stdarg.h>
#include <stdio.h>

#include "strfuncs.h"

// # String formatting

// This is merely the wrapper for the below (which takes an explicit `va_list`).
char* kh_strdupf(const char *format, ...) {
	va_list args;

	va_start(args, format);
	char *result = kh_vstrdupf(format, args);
	va_end(args);

	return result;
}

// Often, it's useful to return the newly-allocated result of an `sprintf` (without knowing the
// resulting length of the string) all in one step.
char* kh_vstrdupf(const char *format, va_list args) {
	// We need to call vsnprintf twice, so we need two args objects to give it.
	va_list args2;
	va_copy(args2, args);

	// First, call it with no target or size to calculate the length.
	size_t size = vsnprintf(NULL, 0, format, args) + 1;

	// Then allocate the result, actually do the sprintf and return.
	char *result = GC_MALLOC_ATOMIC(size);
	vsnprintf(result, size, format, args2);
	va_end(args2);

	return result;
}

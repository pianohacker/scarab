// # Headers

#include <gc.h>
#include <stdarg.h>
#include <stdio.h>

#include "strfuncs.h"

// # String formatting

char* kh_strdupf(const char *format, ...) {
	va_list args;

	va_start(args, format);
	char *result = kh_vstrdupf(format, args);
	va_end(args);

	return result;
}

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

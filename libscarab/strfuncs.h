#ifndef __STRFUNCS_H__
#define __STRFUNCS_H__

// # String formatting

#include <stdarg.h>

// Format a string, automatically allocating a string of the correct size.
char* kh_strdupf(const char *format, ...);
char* kh_vstrdupf(const char *format, va_list args);

#endif

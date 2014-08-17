#ifndef __UTIL_H__
#define __UTIL_H__

// Utility functions and macros used by internal code

// Simple function to pass failure back up the stack.
// Almost every usage of kh_eval should be checked with this macro.
#define _REQUIRE(x) if (!x) return x

#endif

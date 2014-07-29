#include <glib.h>
#include <stdbool.h>

#include "eval.h"
#include "value.h"

struct _KhContext {
};

KhContext* kh_context_new() {
	static bool core_init_done = false;
	
	if (!core_init_done) {
		kh_nil = kh_new(KH_NIL);

		core_init_done = true;
	}

	return g_slice_new0(KhContext);
}

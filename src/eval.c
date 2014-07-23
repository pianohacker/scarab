#include <glib.h>
#include <stdbool.h>

#include "eval.h"
#include "value.h"

struct _ScarabContext {
};

ScarabContext* scarab_context_new() {
	static bool core_init_done = false;
	
	if (!core_init_done) {
		scarab_nil = scarab_new(SCARAB_NIL);

		core_init_done = true;
	}

	return g_slice_new0(ScarabContext);
}

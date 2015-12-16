// # Headers
#include <stdbool.h>
#include <stdio.h>

#include "eval.c"
#include "list.h"
#include "parser.h"
#include "record.h"
#include "value.h"

// # Main program
int main(int argc, char **argv) {
	// Start up the execution context, where the root scope and other information specific to this
	// interpreter lives.
	KhContext *ctx = kh_context_new();

	// ## File execution
	// Check for a filename as the first argument

	// ## REPL
	while (true) {
		// Use a large input buffer; memory is cheap, laziness is rewarding.
		char buffer[65536];
		printf("> ");

		// Check for EOF.
		if (!fgets(buffer, sizeof(buffer), stdin)) break;

		// Parse our input string into a list of lists (assumed to be an open list).
		GError *err = NULL;
		KhValue *forms = kh_parse_string(buffer, &err);

		if (!forms) {
			printf("Parse error: %s\n", err->message);
			continue;
		}

		// Only print a number before each result if there is more than one result.
		bool print_number = true;
		if (forms->d_right == kh_nil) {
			print_number = false;
		}

		// Finally, run each form, checking for errors, and print out the result.
		int i = 1;
		KH_ITERATE(forms) {
			KhValue *value = kh_eval(ctx, elem->d_left);

			if (value == NULL) {
				printf("Error: %s\n", kh_inspect(kh_get_error(ctx)));
			} else {
				if (print_number) printf("%d. ", i++);
				// If we run only a single form and it returns nil, don't bother printing it.
				if (print_number || value != kh_nil) puts(kh_inspect(value));
			}
		}
	}
}

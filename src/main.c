#include <stdbool.h>
#include <stdio.h>

#include "eval.c"
#include "list.h"
#include "parser.h"
#include "value.h"

int main(int argc, char **argv) {
	KhContext *ctx = kh_context_new();

	while (true) {
		// Memory is free, laziness is rewarding
		char buffer[65536];
		printf("> ");

		if (!fgets(buffer, sizeof(buffer), stdin)) break;

		GError *err = NULL;
		KhValue *forms = kh_parse_string(buffer, &err);

		if (!forms) {
			printf("Parse error: %s\n", err->message);
			continue;
		}

		bool print_number = true;
		if (forms->d_right == kh_nil) {
			print_number = false;
		}

		int i = 1;
		KH_ITERATE(forms) {
			KhValue *value = kh_eval(ctx, elem->d_left);

			if (value == NULL) {
				printf("Error: %s\n", kh_inspect(kh_get_error(ctx)));
			} else {
				if (print_number) printf("%d. ", i++);
				if (print_number || value != kh_nil) puts(kh_inspect(value));
			}
		}
	}
}

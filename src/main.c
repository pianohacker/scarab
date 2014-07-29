#include <stdbool.h>
#include <stdio.h>

#include "eval.c"
#include "list.h"
#include "parser.h"
#include "value.h"

int main(int argc, char **argv) {
	ScarabContext *ctx = scarab_context_new();

	while (true) {
		char buffer[1024];
		printf("> ");

		if (!fgets(buffer, sizeof(buffer), stdin)) break;

		GError *err = NULL;
		ScarabValue *forms = scarab_parse_string(buffer, &err);

		if (!forms) {
			printf("Parse error: %s\n", err->message);
			continue;
		}

		bool print_number = true;
		if (forms->d_right == scarab_nil) {
			print_number = false;
		}

		int i = 1;
		SCARAB_ITERATE(forms) {
			ScarabValue *value = scarab_eval(ctx, forms->d_left);

			if (print_number) printf("%d. ", i++);
			puts(scarab_inspect(value));
		}
	}
}

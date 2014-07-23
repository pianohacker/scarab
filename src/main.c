#include <stdbool.h>
#include <stdio.h>

#include "eval.c"
#include "parser.h"
#include "value.h"

int main(int argc, char **argv) {
	ScarabContext *ctx = scarab_context_new();

	while (true) {
		char buffer[1024];
		printf("> ");

		if (!fgets(buffer, sizeof(buffer), stdin)) break;

		GError *err = NULL;
		ScarabValue *value = scarab_parse_string(buffer, &err);

		if (!value) {
			printf("Parse error: %s\n", err->message);
			continue;
		}

		puts(scarab_inspect(value));
	}
}

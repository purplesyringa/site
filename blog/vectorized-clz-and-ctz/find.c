#include <stdio.h>

int main() {
	for (unsigned factor = 1; factor != 0; factor++) {
		unsigned low_mask = 0;
		for (int bit = 0; bit < 16; bit++) {
			int index = ((factor << bit) - factor) >> 28;
			low_mask |= 1 << index;
		}
		if (low_mask != 0xffff) {
			continue;
		}

		unsigned high_mask = 0;
		for (int bit = 16; bit < 32; bit++) {
			int index = ((factor << bit) - factor) >> 28;
			high_mask |= 1 << index;
		}
		if (high_mask != 0xffff) {
			continue;
		}

		printf("%x\n", factor);
	}
}

#include <stdio.h>
#include <stdint.h>

void duumbi_print_i64(int64_t val) {
    printf("%lld\n", (long long)val);
}

void duumbi_print_f64(double val) {
    printf("%.15g\n", val);
}

void duumbi_print_bool(int8_t val) {
    printf("%s\n", val ? "true" : "false");
}

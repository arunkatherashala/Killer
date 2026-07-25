/*
 * KORE v2 Reader — C example (test file)
 * Compile: gcc -O2 -o kore_test kore_test.c
 */
#define KORE_IMPLEMENTATION
#include "kore_reader.h"
#include <stdio.h>

int main(int argc, char **argv) {
    const char *path = (argc > 1) ? argv[1] : "../../test/test_v2.kore";
    KoreFile kf;
    int rc = kore_open(&kf, path);
    if (rc != 0) { fprintf(stderr, "Failed to open KORE file: %d\n", rc); return 1; }

    printf("KORE v%d | %llu rows x %d cols | %u chunks\n",
           kf.version, (unsigned long long)kf.nrows, kf.ncols, kf.nchunks);
    printf("Columns:\n");
    for (int i = 0; i < kf.ncols; i++)
        printf("  [%d] %s : %s\n", i, kf.schema[i].name, kore_type_name(kf.schema[i].ktype));

    /* Read each column */
    for (int ci = 0; ci < kf.ncols; ci++) {
        KoreColumn col;
        rc = kore_read_column_idx(&kf, ci, &col);
        if (rc != 0) { printf("  FAILED to read col %d: %d\n", ci, rc); continue; }

        printf("\n%s (%s) — %llu values:\n", kf.schema[ci].name,
               kore_type_name(col.ktype), (unsigned long long)col.len);

        uint64_t show = col.len < 10 ? col.len : 10;
        for (uint64_t i = 0; i < show; i++) {
            printf("  [%llu] ", (unsigned long long)i);
            switch (col.ktype) {
                case KTYPE_INT:   printf("%lld\n", (long long)col.ints[i]); break;
                case KTYPE_FLOAT: printf("%.4f\n", col.floats[i]); break;
                case KTYPE_BOOL:  printf("%s\n", col.bools[i] ? "true" : "false"); break;
                case KTYPE_STR:   printf("\"%s\"\n", col.strings[i]); break;
                default:          printf("?\n"); break;
            }
        }
        kore_free_column(&col);
    }

    kore_close(&kf);
    printf("\nDONE — all columns read successfully.\n");
    return 0;
}

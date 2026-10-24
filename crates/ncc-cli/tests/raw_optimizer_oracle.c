#include <stdatomic.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int main(void) {
    uint8_t *source = malloc(128);
    uint8_t *target = malloc(128);
    if (source == NULL || target == NULL) {
        return 2;
    }

    memset(source, 0, 128);
    memset(target, 0, 128);
    for (int i = 0; i < 64; ++i) {
        source[i] = (uint8_t)(i * 2 + 1);
    }
    memcpy(target, source, 64);
    memmove(target + 8, target, 48);
    memset(target + 56, 85, 8);

    int32_t checksum = 0;
    for (int i = 0; i < 64; ++i) {
        checksum += target[i];
    }
    volatile int32_t observable = checksum;
    checksum = observable;

    _Atomic int32_t cell = 0;
    atomic_store_explicit(&cell, 41, memory_order_release);
    int32_t old = atomic_fetch_add_explicit(&cell, 1, memory_order_acq_rel);
    int32_t observed = atomic_load_explicit(&cell, memory_order_acquire);
    atomic_thread_fence(memory_order_seq_cst);
    atomic_signal_fence(memory_order_acq_rel);

    printf("%d\n%d\n", checksum, observed);
    free(target);
    free(source);
    return checksum == 3048 && old == 41 && observed == 42 ? 0 : 1;
}

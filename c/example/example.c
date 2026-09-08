/*
 * Example C client for the Enpose API: discover a device on the local
 * network, then stream and print live marker poses for a few seconds.
 *
 * Build with CMake (from this enpose-api/c/example directory):
 *   cmake -S . -B build
 *   cmake --build build
 *   ./build/enpose_example
 *
 * Or directly (from the enpose-api directory):
 *   cargo build --release --manifest-path rust/Cargo.toml
 *   cc c/example/example.c -I c/include -L rust/target/release -lenpose_api \
 *      -o /tmp/enpose_example
 *   LD_LIBRARY_PATH=rust/target/release /tmp/enpose_example
 */
#include <stdio.h>

#include "enpose_api.h"

/* How many batches of poses to print before exiting. */
#define POSE_BATCHES 200

int main(void) {
    EnposeDeviceInfo *devices = NULL;
    size_t device_count = 0;

    if (enpose_discover(&devices, &device_count) != ENPOSE_OK) {
        fprintf(stderr, "discovery failed\n");
        return 1;
    }

    if (device_count == 0) {
        printf("No Enpose devices found on the local network.\n");
        enpose_device_info_array_free(devices, device_count);
        return 0;
    }

    printf("Found %zu device(s):\n", device_count);
    const EnposeDeviceInfo *chosen = NULL;
    for (size_t i = 0; i < device_count; i++) {
        printf("  - %s (serial %u): %s\n", devices[i].ip, devices[i].serial,
               devices[i].compatible ? "compatible" : "INCOMPATIBLE");
        if (chosen == NULL && devices[i].compatible) {
            chosen = &devices[i];
        }
    }

    if (chosen == NULL) {
        printf("No compatible device to stream poses from.\n");
        enpose_device_info_array_free(devices, device_count);
        return 0;
    }

    printf("\nStreaming poses from %s...\n", chosen->ip);

    /* Threaded mode: a background thread buffers poses between polls. */
    EnposePoseStream *stream = enpose_pose_stream_connect(chosen->ip, true);
    enpose_device_info_array_free(devices, device_count);
    if (stream == NULL) {
        fprintf(stderr, "failed to connect pose stream\n");
        return 1;
    }

    for (int batch = 0; batch < POSE_BATCHES; batch++) {
        EnposeMarkerPose *poses = NULL;
        size_t count = 0;
        /* Blocking receive: waits up to 3 s for a pose update, so no manual
         * polling delay is needed. */
        if (enpose_pose_stream_receive(stream, true, &poses, &count) != ENPOSE_OK) {
            fprintf(stderr, "receive failed\n");
            break;
        }
        for (size_t i = 0; i < count; i++) {
            printf("  t=%10lu us  marker %3u: pos=(%+.4f, %+.4f, %+.4f) sensors=%u emitters=%u\n",
                   (unsigned long)poses[i].timestamp, poses[i].marker_id,
                   poses[i].x, poses[i].y, poses[i].z, poses[i].sensors,
                   poses[i].observed_emitters);
        }
        enpose_marker_pose_array_free(poses, count);
    }

    enpose_pose_stream_free(stream);
    return 0;
}

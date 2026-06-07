// Example C++ client for the Enpose API: discover a device on the local
// network, then stream and print live marker poses for a few seconds.
//
// Build with CMake (from this enpose-api/cpp/example directory):
//   cmake -S . -B build
//   cmake --build build
//   ./build/enpose_example_cpp
//
// Or directly (from the enpose-api directory):
//   cargo build --release --manifest-path rust/Cargo.toml
//   c++ cpp/example/example.cpp -I cpp/include -I c/include \
//       -L rust/target/release -lenpose_api -o /tmp/enpose_example_cpp
//   LD_LIBRARY_PATH=rust/target/release /tmp/enpose_example_cpp

#include "enpose_api.hpp"

#include <cstdio>

// How many batches of poses to print before exiting.
constexpr int kPoseBatches = 200;

int main() {
    try {
        auto devices = enpose::discover();
        if (devices.empty()) {
            std::printf("No Enpose devices found on the local network.\n");
            return 0;
        }

        std::printf("Found %zu device(s):\n", devices.size());
        const enpose::DeviceInfo* chosen = nullptr;
        for (const auto& device : devices) {
            std::printf("  - %s (serial %u): %s\n", device.ip.c_str(), device.serial,
                        device.compatible ? "compatible" : "INCOMPATIBLE");
            if (chosen == nullptr && device.compatible) {
                chosen = &device;
            }
        }

        if (chosen == nullptr) {
            std::printf("No compatible device to stream poses from.\n");
            return 0;
        }

        std::printf("\nStreaming poses from %s...\n", chosen->ip.c_str());

        // Threaded mode: a background thread buffers poses between polls.
        enpose::PoseStream stream(*chosen, /*create_thread=*/true);

        for (int batch = 0; batch < kPoseBatches; ++batch) {
            // Blocking receive: waits up to 3 s for a pose update, so no
            // manual polling delay is needed.
            for (const auto& pose : stream.receive(/*block=*/true)) {
                std::printf(
                    "  t=%10llu us  marker %3u: pos=(%+.4f, %+.4f, %+.4f) sensors=%u\n",
                    static_cast<unsigned long long>(pose.timestamp), pose.marker_id,
                    pose.x, pose.y, pose.z, pose.sensors);
            }
        }
    } catch (const enpose::Error& e) {
        std::fprintf(stderr, "error: %s\n", e.what());
        return 1;
    }

    return 0;
}

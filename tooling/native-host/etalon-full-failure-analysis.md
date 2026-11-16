# Etalon full-run failure analysis

Run: `cargo xtask test`, revision `785ab9c1b7d729db9afcd9b0f3fd70344b37be1e`, Rust 1.96.1, nextest 0.9.152, two build/test workers. Full log: `etalon-full-resume-20261116-161454.log`. This run was explicitly interrupted after 273 completed cases (271 harness passes, one failure, one abort); 5,234 scheduled cases did not complete. Candidate source edits made after these binaries were built do not change this run's attribution.

## Intel OpenGL buffer-clear regression

Exact test: `[Executed] [Gl/Intel(R) RaptorLake-S Mobile Graphics Controller/5] etalon_gpu::regression::issue_4122::clear_buffer_range_respected`.

Failed after 39.304 seconds: `Result::unwrap()` on `Err(Timeout)` at `tests/tests/etalon-gpu/regression/issue_4122.rs:47`. The other five adapter/backend variants passed. The failure occurs before the final range comparison; no incorrect cleared range was reported.

Call path: the test requests `PollType::wait_indefinitely()`. `tests/src/poll.rs` forwards native calls to `device.poll`; `etalon-core/src/device/resource.rs` preserves `timeout: None`. `etalon-hal/src/gles/device.rs:1844` maps this to `u32::MAX` nanoseconds. `gles/fence.rs:159` clamps it to `i32::MAX` nanoseconds and issues a single `client_wait_sync`. `TIMEOUT_EXPIRED` and `WAIT_FAILED` both become `false`. Consequently, the promised indefinite wait is bounded to roughly 2.147 seconds per call. Resource contention may expose this mismatch, but this run does not prove contention or a driver hang. The separate nextest process timeout is 90 seconds and did not cause this failure.

## Intel Vulkan compute-pass benchmark

Exact benchmark: `Computepass Encoding`, adapter `Intel(R) RaptorLake-S Mobile Graphics Controller`, Vulkan driver 101.7092. Aborted after 40.488 seconds with status `0xc0000409` following a panic in a non-unwinding Vulkan debug callback.

Validation message: `VUID-VkSubmitInfo-pSignalSemaphores-03242`: submit attempted to signal timeline value 4 while the semaphore's current value was `18446744073709551615`. `etalon-hal/src/vulkan/instance.rs:195` panicked on that validation report. The separate `SymInitialize` diagnostic is not the cause established by the traceback.

Read-only source inspection: `vulkan/mod.rs` creates the submitted signal value from the core submission index, while `device.rs` creates the timeline semaphore with the Vulkan default initial value of zero. Native `get_semaphore_counter_value` and `wait_semaphores` errors already pass through `map_host_device_oom_and_lost_err`; a `VK_ERROR_DEVICE_LOST` return is not intentionally discarded. No inspected Etalon path writes UINT64_MAX into this semaphore. That value alone is not sufficient evidence to label the cause device loss or a driver defect. Focused reproduction remains necessary.

Official Vulkan contracts: [counter-value query](https://docs.vulkan.org/refpages/latest/refpages/source/vkGetSemaphoreCounterValue.html), [timeline wait](https://docs.vulkan.org/refpages/latest/refpages/source/vkWaitSemaphores.html). Both expose device-loss errors; a normal counter value does not itself encode a defined device-loss sentinel.

## Focused reproduction after the full run

Run from `C:/github/etalon`, using the already-built binaries to preserve this revision. Set the same Agility environment values from the full-run configuration, retained separately. Do not execute these concurrently with the full suite or GPU profiling.

```powershell
& C:/github/etalon/target-webizen-tests/debug/deps/etalon_gpu-4405d1164bc21cd6.exe --exact '[Executed] [Gl/Intel(R) RaptorLake-S Mobile Graphics Controller/5] etalon_gpu::regression::issue_4122::clear_buffer_range_respected' --nocapture

$env:ETALON_BACKEND = 'vulkan'
$env:ETALON_ADAPTER_NAME = 'Intel(R) RaptorLake-S Mobile Graphics Controller'
& C:/github/etalon/target-webizen-tests/debug/deps/etalon_benchmark-7dadce70023a7b1c.exe --exact 'Computepass Encoding'
```

The second command uses the benchmark harness's test mode (one iteration per sub-benchmark), not a new performance baseline. Restore the process environment after each reproduction. Do not relax validation, skip the failing cases, or reinterpret unsupported/expected-failure harness passes as successfully executed GPU capabilities.

## Isolated reproduction and correction

Both original binaries were retained under `original-785/`, with their hashes in `etalon-full-reproduction-binary-hashes.json`. The isolated GL test passed all 1,000 range checks in 33.67 seconds. The finite clamp remains an independently established contract defect, but its runtime failure did not reproduce in this isolated check.

The isolated original Intel Vulkan benchmark reproduced the same semaphore validation abort in 16.17 seconds. Its stack trace identified the empty submission immediately after the bindless workload. Inspection then established invalid inputs in that workload: the bindless shader launched 16 invocations, addressed descriptor indices 0..31, but test mode declared 16 entries and supplied only eight. The ordinary shader also launched 16 invocations that wrote the same fixed storage locations.

The corrected candidate uses one invocation per dispatch, supplies every declared bindless descriptor (two resources per dispatch), and reads the second sampled texture from its correct index. Dispatch-count and CPU encoding/submission measurements are preserved. Corrected execution labels contain `single invocation v2` to avoid matching historical result keys with different shader semantics. These smoke-run timings are not published benchmark results.

Three native Naga contract checks passed, including rejection of the previous 16-invocation workloads. A separate candidate benchmark was compiled against the unchanged original provider RLIBs, with native dependencies pinned in `compute-workload-candidate/original-dependencies.json`. The same Intel Vulkan adapter then completed all seven sub-benchmarks, including bindless and empty submission, with exit zero in 7.73 seconds. No Vulkan provider code or validation rule was changed. This demonstrates that correcting the workload removes this reproducing failure; it does not qualify the entire GPU suite.

Exact receipts: `etalon-focused-original-gl.json`, `etalon-focused-original-vulkan.json`, `compute-workload-candidate/intel-vulkan.json` and corresponding logs. The full suite remains required after combining and reviewing the GL wait fix and benchmark correction.

# Benchmark Report

Date: 2026-04-28

This report summarizes the current parity and benchmark suite for `proj-rust`
against bundled C PROJ. It captures both the current Rust-versus-C performance
shape and the current transform-construction cost for the `0.4.0` release
state.

## System Under Test

- Machine: Apple M1
- CPU topology: 8 logical CPUs
- Memory: 16 GiB
- OS: macOS 13.0
- Architecture: `arm64`
- Rust toolchain: `rustc 1.92.0`

These measurements reflect this machine and should not be read as universal
throughput claims.

## Scope

- Live parity against bundled C PROJ using the checked-in 161-value reference corpus
- Transform-construction timing for:
  - `EPSG:4326 -> 3857`
  - `EPSG:4267 -> 4326`
- Single-point comparisons for:
  - `EPSG:4326 -> 3857`
  - `EPSG:4326 -> 32618`
  - `EPSG:4326 -> 3413`
  - `EPSG:4267 -> 4326`
- Single-point 3D comparisons for:
  - `EPSG:4326 -> 3857`
  - `EPSG:4267 -> 4326`
- Batch comparison for 10,000 points in `EPSG:4326 -> 3857`
- Batch 3D timing for 10,000 points in `EPSG:4326 -> 3857`

## Methodology

Commands used for this report:

```sh
cargo test -p proj-core --features c-proj-compat
./scripts/run-reference-benchmarks.sh
```

Notes:

- The parity run passed both live C PROJ tests.
- The 3D parity run passed the live C PROJ 3D cases.
- The parity corpus currently contains 161 reference values.
- Criterion is used for all timing.
- The batch benchmark reports element throughput for 10,000 coordinate pairs.
- The current 3D API preserves the third ordinate unchanged because the CRS model remains horizontal-only.

## Current Results

### Parity

- `live_c_proj_parity`: 2 tests passed
- `live_c_proj_parity_3d`: 1 test passed
- The 161-value corpus remained in sync with live bundled C PROJ
- `proj-core` matched live bundled C PROJ for all supported corpus cases
- `proj-core` matched live bundled C PROJ for all covered 3D cases

### Construction Summary

| workload | proj-rust |
| --- | ---: |
| `construct 4326 -> 3857` | 842.26 ns |
| `construct 4267 -> 4326` | 34.21 us |

### Single-Point Summary

| workload | proj-rust | C PROJ | result |
| --- | ---: | ---: | --- |
| `4326 -> 3857` | 64.91 ns | 75.89 ns | `proj-rust` 1.17x faster |
| `4326 -> 32618` | 71.48 ns | 142.25 ns | `proj-rust` 1.99x faster |
| `4326 -> 3413` | 217.21 ns | 111.59 ns | C PROJ 1.95x faster |
| `4267 -> 4326` | 277.17 ns | 281.69 ns | `proj-rust` 1.02x faster |

### Single-Point 3D Summary

| workload | proj-rust | C PROJ | result |
| --- | ---: | ---: | --- |
| `3D 4326 -> 3857` | 63.07 ns | 80.06 ns | `proj-rust` 1.27x faster |
| `3D 4267 -> 4326` | 247.88 ns | 295.44 ns | `proj-rust` 1.19x faster |

### Batch Summary

| workload | proj-rust | C PROJ | result |
| --- | ---: | ---: | --- |
| `10K 4326 -> 3857` sequential | 711.61 us | 877.78 us | `proj-rust` 1.23x faster |
| `10K 4326 -> 3857` throughput | 14.1 Melem/s | 11.4 Melem/s | `proj-rust` 1.23x higher throughput |
| `10K 4326 -> 3857` parallel | 722.06 us | 877.78 us | `proj-rust` 1.22x faster |
| `10K 4326 -> 3857` parallel throughput | 13.8 Melem/s | 11.4 Melem/s | `proj-rust` 1.22x higher throughput |

### Batch 3D Summary

| workload | proj-rust | result |
| --- | ---: | --- |
| `10K 3D 4326 -> 3857` sequential | 607.03 us | 16.5 Melem/s |
| `10K 3D 4326 -> 3857` parallel | 645.65 us | 15.5 Melem/s |

## Interpretation

- `proj-rust` remains faster than bundled C PROJ in most measured Rust-versus-C cases in this suite.
- Construction is now sub-microsecond for simple registry-backed projected transforms and roughly 34 microseconds for the covered datum-shifted pair.
- UTM single-point transforms show the largest relative win, while the covered Polar Stereographic case is currently faster in C PROJ on this host.
- On this host and at 10K elements, the adaptive parallel path is essentially flat with the sequential path for the covered workloads, which is the intended crossover behavior.
- The current 3D path stays close to the 2D fast path because the third ordinate is preserved unchanged.
- The live parity suite remains the strongest correctness signal because it checks both corpus drift and current Rust-versus-C behavior.

## Limits

- This report reflects one machine.
- The benchmark suite is representative, not exhaustive across the full CRS registry.
- The batch comparison uses one 10K Web Mercator workload; different sizes or
  thread topologies may shift the parallel-versus-sequential crossover point.

# [`egui`](https://github.com/emilk/egui) demo library

[![Latest version](https://img.shields.io/crates/v/egui_demo_lib.svg)](https://crates.io/crates/egui_demo_lib)
[![Documentation](https://docs.rs/egui_demo_lib/badge.svg)](https://docs.rs/egui_demo_lib)
[![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/)
![MIT](https://img.shields.io/badge/license-MIT-blue.svg)
![Apache](https://img.shields.io/badge/license-Apache-blue.svg)

This crate contains example code for [`egui`](https://github.com/emilk/egui).

The demo library is a separate crate for three reasons:

* To ensure it only uses the public `egui` api.
* To remove the amount of code in `egui` proper.
* To make it easy for 3rd party egui integrations to use it for tests.
  - See for instance https://github.com/not-fl3/egui-miniquad/blob/master/examples/demo.rs

This crate also contains benchmarks for egui. 
Run them with 
```bash
# Run all benchmarks
cargo bench -p egui_demo_lib 

# Run a single benchmark
cargo bench -p egui_demo_lib "benchmark name"

# Profile benchmarks with cargo-flamegraph (--root flag is necessary for MacOS)
CARGO_PROFILE_BENCH_DEBUG=true cargo flamegraph --bench benchmark --root -p egui_demo_lib  -- --bench "benchmark name"

# Profile with cargo-instruments
CARGO_PROFILE_BENCH_DEBUG=true cargo instruments --profile bench --bench benchmark -p egui_demo_lib -t time -- --bench "benchmark name" 
```

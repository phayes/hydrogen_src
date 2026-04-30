# Rust HydrogenAudio Sample Rate Conversion testing

This crate tests rust sample-rate converters for HydrogenAudio Sample Rate Conversion testing.

See: https://src.hydrogenaudio.org/

## Quick start

```bash
cargo run -p rubato_src_test -- --workdir=./workspace
cargo run -p rubato_src_test -- --workdir=./workspace --f64 --chunk-size=1024 --sub-chunk=2
cargo run -p rubato_src_test -- --workdir=./workspace --local # Run analysis locally, requires "octave" commandline
cargo run -p rubato_src_test -- --workdir=./workspace --local --json # Run analysis locally and report output in json for automation
```

## Params

- `--workdir`: input/output workspace directory
- `--f64`: use `f64` pipeline (`f32` is default)

## Rubato Params
- `--chunk-size`: rubato chunk-size
- `--sub-chunk`:  rubato sub-chunks

## Add your own converter

1. Add a new workspace binary crate (for example `my_src_test`) and include `hydrogen_src` plus your SRC dependency in `Cargo.toml`.
2. Copy structure from `rubato_src_test` or `ardftsrc_src_test` and adapt as needed
4. Run it with `cargo run -p my_src_test -- --workdir=./workspace`.
5. Open a PR against `https://github.com/phayes/hydrogen_src` to get your converter officially added.

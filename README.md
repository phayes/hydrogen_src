# Rust HydrogenAudio Sample Rate Conversion testing

This crate tests rust sample-rate converters for HydrogenAudio Sample Rate Conversion testing.

See: https://src.hydrogenaudio.org/

## Quick start

```bash
cargo run -p rubato_src_test -- --workdir=./workspace
cargo run -p rubato_src_test -- --workdir=./workspace --f64 --chunk-size=1024 --sub-chunk=2
```

## Params

- `--workdir`: input/output workspace directory
- `--f64`: use `f64` pipeline (`f32` is default)

## Rubato Params
- `--chunk-size`: rubato chunk-size
- `--sub-chunk`:  rubato sub-chunks

use std::num::NonZero;
use std::path::PathBuf;

use clap::{ArgGroup, Parser};
use hydrogen_src::{
    FloatVariant, HydrogenError, HydrogenSrc, LocalHarness, ResampleRequestF32, ResampleRequestF64,
    default_workspace_dir,
};
use rodio::conversions::SampleRateConverter;
use rodio::{ChannelCount, Sample, SampleRate};

#[derive(Debug, Parser)]
#[command(
    group(ArgGroup::new("float-variant").args(["f32", "f64"]))
)]
struct Args {
    // General options
    #[arg(long)]
    workdir: Option<PathBuf>,
    #[arg(long)]
    f32: bool,
    #[arg(long)]
    f64: bool,
    #[arg(long, default_value_t = false)]
    local: bool,
    #[arg(long, default_value_t = false)]
    json: bool,
    #[arg(long, default_value_t = false)]
    score_only: bool,
}

fn main() -> Result<(), HydrogenError> {
    let cli = Args::parse();

    let float_variant = if cli.f64 {
        FloatVariant::F64
    } else {
        FloatVariant::F32
    };

    let workdir = match cli.workdir {
        Some(path) => path,
        None => default_workspace_dir()?,
    };

    // Local mode
    if cli.local {
        let mut local = LocalHarness::new(Some(workdir))?;
        match float_variant {
            FloatVariant::F32 => {
                local.set_callback_f32(run_rodio_f32);
            }
            FloatVariant::F64 => {
                local.set_callback_f64(run_rodio_f64);
            }
        }
        let results = local.run()?;
        if cli.score_only {
            println!("{}", results.balanced_score);
        } else if cli.json {
            println!(
                "{}",
                serde_json::to_string(&results).expect("failed to serialize local results")
            );
        } else {
            println!("{results}");
        }
        return Ok(());
    // Remote upload mode (default)
    } else {
        let mut hydrogen = HydrogenSrc::new(workdir, float_variant, "output-rodio");

        hydrogen.set_callback_f32(run_rodio_f32);
        hydrogen.set_callback_f64(run_rodio_f64);

        let _ = hydrogen.run()?;
        Ok(())
    }
}

// `Sample` is f32 or f64 depending on the `64bit` feature, so the casts are only sometimes no-ops
#[allow(clippy::unnecessary_cast)]
fn run_rodio_f32(request: ResampleRequestF32) -> Vec<f32> {
    let input = request.samples.iter().map(|&s| s as Sample);
    resample(input, request.sample_rate, request.target_sample_rate, request.channels)
        .map(|s| s as f32)
        .collect()
}

#[allow(clippy::unnecessary_cast)]
fn run_rodio_f64(request: ResampleRequestF64) -> Vec<f64> {
    let input = request.samples.iter().map(|&s| s as Sample);
    resample(input, request.sample_rate, request.target_sample_rate, request.channels)
        .map(|s| s as f64)
        .collect()
}

/// rodio's default resampler: the `SampleRateConverter` used by `UniformSourceIterator` (and so by
/// the mixer / player) when a source's sample rate differs from the output's.
fn resample<I: Iterator<Item = Sample>>(
    input: I,
    sample_rate: usize,
    target_sample_rate: usize,
    channels: usize,
) -> SampleRateConverter<I> {
    let from: SampleRate = u32::try_from(sample_rate)
        .ok()
        .and_then(NonZero::new)
        .expect("sample rate must be > 0 and fit in u32");
    let to: SampleRate = u32::try_from(target_sample_rate)
        .ok()
        .and_then(NonZero::new)
        .expect("target sample rate must be > 0 and fit in u32");
    let channels: ChannelCount = u16::try_from(channels)
        .ok()
        .and_then(NonZero::new)
        .expect("channel count must be > 0 and fit in u16");

    SampleRateConverter::new(input, from, to, channels)
}

use std::path::PathBuf;

use clap::{ArgGroup, Parser};
use hydrogen_src::{FloatVariant, HydrogenError, HydrogenSrc, ResampleRequestF32, ResampleRequestF64};
use ardftsrc::{Ardftsrc, Config};

#[derive(Debug, Parser)]
#[command(
    group(ArgGroup::new("float-variant").args(["f32", "f64"]))
)]
struct Args {
    #[arg(long)]
    workdir: PathBuf,
    #[arg(long)]
    f32: bool,
    #[arg(long)]
    f64: bool,
    #[arg(long, default_value_t = 2048)]
    quality: usize,
    #[arg(long, default_value_t = 0.95)]
    bandwidth: f32,
}

fn main() -> Result<(), HydrogenError> {
    let cli = Args::parse();

    let float_variant = if cli.f64 {
        FloatVariant::F64
    } else {
        FloatVariant::F32
    };

    if cli.quality == 0 {
        eprintln!("--quality must be > 0");
        std::process::exit(2);
    }

    if !(0.0..=1.0).contains(&cli.bandwidth) || !cli.bandwidth.is_finite() {
        eprintln!("--bandwidth must be finite and in 0.0..=1.0");
        std::process::exit(2);
    }

    let quality = cli.quality;
    let bandwidth = cli.bandwidth;

    let mut hydrogen = HydrogenSrc::new(
        cli.workdir,
        float_variant,
        &format!("output-ardftsrc-q{}-bw{bandwidth:.2}", quality),
    );

    hydrogen.set_callback_f32(move |request: ResampleRequestF32| -> Vec<f32> {
        run_ardftsrc_f32(request, quality, bandwidth)
    });
    hydrogen.set_callback_f64(move |request: ResampleRequestF64| -> Vec<f64> {
        run_ardftsrc_f64(request, quality, bandwidth)
    });

    let _ = hydrogen.run()?;
    Ok(())
}

fn run_ardftsrc_f32(request: ResampleRequestF32, quality: usize, bandwidth: f32) -> Vec<f32> {
    let config = Config {
        input_sample_rate: request.sample_rate,
        output_sample_rate: request.target_sample_rate,
        channels: request.channels,
        quality,
        bandwidth,
    };

    let mut resampler = Ardftsrc::<f32>::new(config).expect("failed to create ardftsrc f32 resampler");
    resampler
        .process_all(&request.samples)
        .expect("failed during ardftsrc f32 processing")
}

fn run_ardftsrc_f64(request: ResampleRequestF64, quality: usize, bandwidth: f32) -> Vec<f64> {
    let config = Config {
        input_sample_rate: request.sample_rate,
        output_sample_rate: request.target_sample_rate,
        channels: request.channels,
        quality,
        bandwidth,
    };

    let mut resampler = Ardftsrc::<f64>::new(config).expect("failed to create ardftsrc f64 resampler");
    resampler
        .process_all(&request.samples)
        .expect("failed during ardftsrc f64 processing")
}

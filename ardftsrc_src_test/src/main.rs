use std::path::PathBuf;

use ardftsrc::{
    Config, PlanarResampler, TaperType, PRESET_EXTREME, PRESET_FAST, PRESET_GOOD, PRESET_HIGH,
};
use clap::{ArgGroup, Parser, ValueEnum};
use hydrogen_src::{
    FloatVariant, HydrogenError, HydrogenSrc, LocalHarness, ResampleRequestF32, ResampleRequestF64,
};

const LOCAL_SCRIPT_DIR: &str = "scripts/TestScripts";

#[derive(Debug, Parser)]
#[command(
    group(ArgGroup::new("float-variant").args(["f32", "f64"]))
)]
struct Args {
    // General options
    #[arg(long)]
    workdir: PathBuf,
    /// Use f32 internal processing (default is f64).
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

    // Ardftsrc options
    /// Baseline quality/bandwidth from an ardftsrc preset; optional `--quality` / `--bandwidth` override preset values.
    #[arg(long, value_enum)]
    preset: Option<PresetArg>,
    /// Resampler quality (tap count / FFT size parameter). With `--preset`, defaults to the preset; otherwise defaults to 2048.
    #[arg(long)]
    quality: Option<usize>,
    /// Normalized low-pass bandwidth in [0.0, 1.0]. With `--preset`, defaults to the preset; otherwise defaults to 0.95.
    #[arg(long)]
    bandwidth: Option<f32>,
    #[arg(long, value_enum, default_value_t = CliTaperType::Cosine)]
    taper_type: CliTaperType,
    #[arg(long)]
    alpha: Option<f32>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum PresetArg {
    /// Low-latency (quality = 512, bandwidth ≈ 0.8323).
    Fast,
    /// Balanced (quality = 1878, bandwidth ≈ 0.911).
    Good,
    /// High quality (quality = 73622, bandwidth ≈ 0.987).
    High,
    /// Maximum quality (quality = 524514, bandwidth ≈ 0.995).
    Extreme,
}

impl PresetArg {
    fn slug(self) -> &'static str {
        match self {
            Self::Fast => "fast",
            Self::Good => "good",
            Self::High => "high",
            Self::Extreme => "extreme",
        }
    }

    fn base_config(self) -> Config {
        match self {
            Self::Fast => PRESET_FAST,
            Self::Good => PRESET_GOOD,
            Self::High => PRESET_HIGH,
            Self::Extreme => PRESET_EXTREME,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CliTaperType {
    Planck,
    Cosine,
}

impl CliTaperType {
    fn slug(self) -> &'static str {
        match self {
            Self::Planck => "planck",
            Self::Cosine => "cosine",
        }
    }
}

fn build_taper_type(taper_type: CliTaperType, alpha: Option<f32>) -> TaperType {
    match taper_type {
        CliTaperType::Planck => {
            if alpha.is_some() {
                eprintln!("--alpha can only be used with --taper-type=cosine");
                std::process::exit(2);
            }
            TaperType::Planck
        }
        CliTaperType::Cosine => {
            let alpha = alpha.unwrap_or(3.4375);
            if alpha <= 0.0 || !alpha.is_finite() {
                eprintln!("--alpha must be finite and > 0 when --taper-type=cosine");
                std::process::exit(2);
            }
            TaperType::Cosine(alpha)
        }
    }
}

fn main() -> Result<(), HydrogenError> {
    let cli = Args::parse();

    let float_variant = if cli.f32 {
        FloatVariant::F32
    } else {
        FloatVariant::F64
    };

    let (quality, bandwidth) = match cli.preset {
        Some(preset) => {
            let base = preset.base_config();
            (
                cli.quality.unwrap_or(base.quality),
                cli.bandwidth.unwrap_or(base.bandwidth),
            )
        }
        None => (
            cli.quality.unwrap_or(2048),
            cli.bandwidth.unwrap_or(0.95),
        ),
    };

    if quality == 0 {
        eprintln!("--quality must be > 0");
        std::process::exit(2);
    }

    if !(0.0..=1.0).contains(&bandwidth) || !bandwidth.is_finite() {
        eprintln!("--bandwidth must be finite and in 0.0..=1.0");
        std::process::exit(2);
    }

    let taper_type = build_taper_type(cli.taper_type, cli.alpha);
    let taper_slug = cli.taper_type.slug();
    let alpha_slug = match taper_type {
        TaperType::Cosine(alpha) => format!("-a{alpha:.2}"),
        TaperType::Planck => String::new(),
    };

    let output_label = match cli.preset {
        Some(preset) => format!(
            "output-ardftsrc-preset-{}-q{quality}-bw{bandwidth:.4}-t{taper_slug}{alpha_slug}",
            preset.slug()
        ),
        None => format!(
            "output-ardftsrc-q{quality}-bw{bandwidth:.4}-t{taper_slug}{alpha_slug}"
        ),
    };

    if cli.local {
        let mut local = LocalHarness::new(cli.workdir, LOCAL_SCRIPT_DIR);
        match float_variant {
            FloatVariant::F32 => {
                local.set_callback_f32(move |request: ResampleRequestF32| -> Vec<f32> {
                    run_ardftsrc_f32(request, quality, bandwidth, taper_type)
                });
            }
            FloatVariant::F64 => {
                local.set_callback_f64(move |request: ResampleRequestF64| -> Vec<f64> {
                    run_ardftsrc_f64(request, quality, bandwidth, taper_type)
                });
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
    }

    let mut hydrogen = HydrogenSrc::new(cli.workdir, float_variant, &output_label);

    hydrogen.set_callback_f32(move |request: ResampleRequestF32| -> Vec<f32> {
        run_ardftsrc_f32(request, quality, bandwidth, taper_type)
    });
    hydrogen.set_callback_f64(move |request: ResampleRequestF64| -> Vec<f64> {
        run_ardftsrc_f64(request, quality, bandwidth, taper_type)
    });

    let _ = hydrogen.run()?;
    Ok(())
}

fn run_ardftsrc_f32(
    request: ResampleRequestF32,
    quality: usize,
    bandwidth: f32,
    taper_type: TaperType,
) -> Vec<f32> {
    let config = Config {
        input_sample_rate: request.sample_rate,
        output_sample_rate: request.target_sample_rate,
        channels: request.channels,
        quality,
        bandwidth,
        taper_type,
    };

    let mut resampler = PlanarResampler::<f32>::new(config)
        .expect("failed to create ardftsrc f32 resampler");

    let input_samples = vec![request.samples.as_slice()];
    let mut output_samples = resampler
        .process_all(&input_samples)
        .expect("failed during ardftsrc f32 processing");

    output_samples.pop_channel().expect("failed to get output samples")
}

fn run_ardftsrc_f64(
    request: ResampleRequestF64,
    quality: usize,
    bandwidth: f32,
    taper_type: TaperType,
) -> Vec<f64> {
    let config = Config {
        input_sample_rate: request.sample_rate,
        output_sample_rate: request.target_sample_rate,
        channels: request.channels,
        quality,
        bandwidth,
        taper_type,
    };

    let mut resampler = PlanarResampler::<f64>::new(config)
        .expect("failed to create ardftsrc f64 resampler");

    let input_samples = vec![request.samples.as_slice()];
    let mut output_samples = resampler
        .process_all(&input_samples)
        .expect("failed during ardftsrc f64 processing");

    output_samples.pop_channel().expect("failed to get output samples")
}

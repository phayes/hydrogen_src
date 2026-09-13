use std::path::PathBuf;

use ardftsrc::{
    Config, PRESET_EXTREME, PRESET_FAST, PRESET_GOOD, PRESET_HIGH, PlanarResampler, TaperType,
};
use clap::{ArgGroup, Parser, ValueEnum};
use hydrogen_src::{
    FloatVariant, HydrogenError, HydrogenSrc, LocalHarness, ResampleRequestF32, ResampleRequestF64,
    default_workspace_dir,
};

#[derive(Debug, Parser)]
#[command(
    group(ArgGroup::new("float-variant").args(["f32", "f64"]))
)]
struct Args {
    // General options
    #[arg(long)]
    workdir: Option<PathBuf>,
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
    /// Taper alpha parameter. Used by `bessel`, `cosine`, and `beta_cdf`.
    #[arg(long)]
    alpha: Option<f32>,
    /// Taper beta parameter. Used by `beta_cdf`.
    #[arg(long)]
    beta: Option<f32>,
    /// Frequency-dependent phase rotation in [-1.0, 1.0]. Defaults to 0.0.
    #[arg(long)]
    phase: Option<f32>,
    /// Phase rotation intensity in [0.0, 100.0]. Defaults to 40.0.
    #[arg(long)]
    phase_intensity: Option<f32>,
    /// Enable 2:1 pre-decimation stages ahead of the FFT resampler for large downsampling ratios.
    /// With `--preset`, defaults to the preset; otherwise defaults to false.
    #[arg(long)]
    decimate: Option<bool>,
    /// Use the double-double-precision FFT backend. Much slower; intended for extreme quality.
    /// Only compatible with f64 processing.
    #[arg(long = "dd-fft", alias = "dd_fft")]
    dd_fft: bool,
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
    Bessel,
    Cosine,
    #[value(name = "beta_cdf", alias = "beta-cdf")]
    BetaCdf,
}

impl CliTaperType {
    fn slug(self) -> &'static str {
        match self {
            Self::Planck => "planck",
            Self::Bessel => "bessel",
            Self::Cosine => "cosine",
            Self::BetaCdf => "beta_cdf",
        }
    }
}

fn build_taper_type(taper_type: CliTaperType, alpha: Option<f32>, beta: Option<f32>) -> TaperType {
    match taper_type {
        CliTaperType::Planck => {
            if alpha.is_some() || beta.is_some() {
                eprintln!(
                    "--alpha and --beta can only be used with --taper-type=bessel, --taper-type=cosine, or --taper-type=beta_cdf"
                );
                std::process::exit(2);
            }
            TaperType::Planck
        }
        CliTaperType::Bessel => {
            if beta.is_some() {
                eprintln!("--beta can only be used with --taper-type=beta_cdf");
                std::process::exit(2);
            }
            let alpha = alpha.unwrap_or(6.0);
            if alpha <= 0.0 || !alpha.is_finite() {
                eprintln!("--alpha must be finite and > 0 when --taper-type=bessel");
                std::process::exit(2);
            }
            TaperType::Bessel(alpha)
        }
        CliTaperType::Cosine => {
            if beta.is_some() {
                eprintln!("--beta can only be used with --taper-type=beta_cdf");
                std::process::exit(2);
            }
            let alpha = alpha.unwrap_or(3.4375);
            if alpha <= 0.0 || !alpha.is_finite() {
                eprintln!("--alpha must be finite and > 0 when --taper-type=cosine");
                std::process::exit(2);
            }
            TaperType::Cosine(alpha)
        }
        CliTaperType::BetaCdf => {
            let alpha = alpha.unwrap_or(24.0);
            if alpha <= 0.0 || !alpha.is_finite() {
                eprintln!("--alpha must be finite and > 0 when --taper-type=beta_cdf");
                std::process::exit(2);
            }

            let beta = beta.unwrap_or(24.0);
            if beta <= 0.0 || !beta.is_finite() {
                eprintln!("--beta must be finite and > 0 when --taper-type=beta_cdf");
                std::process::exit(2);
            }

            TaperType::BetaCdf { alpha, beta }
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

    let (quality, bandwidth, phase, phase_intensity, decimate) = match cli.preset {
        Some(preset) => {
            let base = preset.base_config();
            (
                cli.quality.unwrap_or(base.quality),
                cli.bandwidth.unwrap_or(base.bandwidth),
                cli.phase.unwrap_or(base.phase),
                cli.phase_intensity.unwrap_or(base.phase_intensity),
                cli.decimate.unwrap_or(base.decimate),
            )
        }
        None => (
            cli.quality.unwrap_or(2048),
            cli.bandwidth.unwrap_or(0.95),
            cli.phase.unwrap_or(Config::DEFAULT.phase),
            cli.phase_intensity
                .unwrap_or(Config::DEFAULT.phase_intensity),
            cli.decimate.unwrap_or(Config::DEFAULT.decimate),
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

    if !(-1.0..=1.0).contains(&phase) || !phase.is_finite() {
        eprintln!("--phase must be finite and in -1.0..=1.0");
        std::process::exit(2);
    }

    if !(0.0..=100.0).contains(&phase_intensity) || !phase_intensity.is_finite() {
        eprintln!("--phase-intensity must be finite and in 0.0..=100.0");
        std::process::exit(2);
    }

    let dd_fft = cli.dd_fft;
    if dd_fft && matches!(float_variant, FloatVariant::F32) {
        eprintln!("--dd-fft is not compatible with --f32; use --f64 instead");
        std::process::exit(2);
    }

    let taper_type = build_taper_type(cli.taper_type, cli.alpha, cli.beta);
    let taper_slug = cli.taper_type.slug();
    let taper_param_slug = match taper_type {
        TaperType::Bessel(alpha) => format!("-a{alpha:.2}"),
        TaperType::Cosine(alpha) => format!("-a{alpha:.2}"),
        TaperType::BetaCdf { alpha, beta } => format!("-a{alpha:.2}-b{beta:.2}"),
        TaperType::Planck => String::new(),
    };
    let phase_slug = format!("-p{phase:.3}-pi{phase_intensity:.1}");
    let decimate_slug = if decimate { "-dec1" } else { "-dec0" };
    let dd_fft_slug = if dd_fft { "-dd1" } else { "-dd0" };

    let output_label = match cli.preset {
        Some(preset) => format!(
            "output-ardftsrc-preset-{}-q{quality}-bw{bandwidth:.4}-t{taper_slug}{taper_param_slug}{phase_slug}{decimate_slug}{dd_fft_slug}",
            preset.slug()
        ),
        None => format!(
            "output-ardftsrc-q{quality}-bw{bandwidth:.4}-t{taper_slug}{taper_param_slug}{phase_slug}{decimate_slug}{dd_fft_slug}"
        ),
    };

    let workdir = match cli.workdir {
        Some(path) => path,
        None => default_workspace_dir()?,
    };

    if cli.local {
        let mut local = LocalHarness::new(Some(workdir))?;
        match float_variant {
            FloatVariant::F32 => {
                local.set_callback_f32(move |request: ResampleRequestF32| -> Vec<f32> {
                    run_ardftsrc_f32(
                        request,
                        quality,
                        bandwidth,
                        taper_type,
                        phase,
                        phase_intensity,
                        decimate,
                    )
                });
            }
            FloatVariant::F64 => {
                local.set_callback_f64(move |request: ResampleRequestF64| -> Vec<f64> {
                    run_ardftsrc_f64(
                        request,
                        quality,
                        bandwidth,
                        taper_type,
                        phase,
                        phase_intensity,
                        decimate,
                        dd_fft,
                    )
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

    let mut hydrogen = HydrogenSrc::new(workdir, float_variant, &output_label);

    hydrogen.set_callback_f32(move |request: ResampleRequestF32| -> Vec<f32> {
        run_ardftsrc_f32(
            request,
            quality,
            bandwidth,
            taper_type,
            phase,
            phase_intensity,
            decimate,
        )
    });
    hydrogen.set_callback_f64(move |request: ResampleRequestF64| -> Vec<f64> {
        run_ardftsrc_f64(
            request,
            quality,
            bandwidth,
            taper_type,
            phase,
            phase_intensity,
            decimate,
            dd_fft,
        )
    });

    let _ = hydrogen.run()?;
    Ok(())
}

fn run_ardftsrc_f32(
    request: ResampleRequestF32,
    quality: usize,
    bandwidth: f32,
    taper_type: TaperType,
    phase: f32,
    phase_intensity: f32,
    decimate: bool,
) -> Vec<f32> {
    let config = Config {
        input_sample_rate: request.sample_rate,
        output_sample_rate: request.target_sample_rate,
        channels: request.channels,
        quality,
        bandwidth,
        taper_type,
        phase,
        phase_intensity,
        decimate,
        ..Config::default()
    };

    let mut resampler =
        PlanarResampler::<f32>::new(config).expect("failed to create ardftsrc f32 resampler");

    let input_samples = vec![request.samples.as_slice()];
    let mut output_samples = resampler
        .process_all(&input_samples)
        .expect("failed during ardftsrc f32 processing");

    output_samples
        .pop_channel()
        .expect("failed to get output samples")
}

fn run_ardftsrc_f64(
    request: ResampleRequestF64,
    quality: usize,
    bandwidth: f32,
    taper_type: TaperType,
    phase: f32,
    phase_intensity: f32,
    decimate: bool,
    dd_fft: bool,
) -> Vec<f64> {
    let config = Config {
        input_sample_rate: request.sample_rate,
        output_sample_rate: request.target_sample_rate,
        channels: request.channels,
        quality,
        bandwidth,
        taper_type,
        phase,
        phase_intensity,
        decimate,
        dd_fft,
        ..Config::default()
    };

    let mut resampler =
        PlanarResampler::<f64>::new(config).expect("failed to create ardftsrc f64 resampler");

    let input_samples = vec![request.samples.as_slice()];
    let mut output_samples = resampler
        .process_all(&input_samples)
        .expect("failed during ardftsrc f64 processing");

    output_samples
        .pop_channel()
        .expect("failed to get output samples")
}

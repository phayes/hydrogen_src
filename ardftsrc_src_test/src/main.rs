use std::path::PathBuf;

use ardftsrc::{
    AliasFloor, Config, HighPrecision, PlanarResampler, TaperType, PRESET_EXTREME, PRESET_FAST,
    PRESET_GOOD, PRESET_HIGH,
};
use clap::{ArgGroup, Parser, ValueEnum};
use hydrogen_src::{
    default_workspace_dir, FloatVariant, HydrogenError, HydrogenSrc, LocalHarness,
    ResampleRequestF32, ResampleRequestF64,
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
    /// Baseline settings from an ardftsrc preset; `--quality`, `--bandwidth`, `--taper-type`,
    /// `--alpha`, `--beta`, etc. override the preset's values.
    #[arg(long, value_enum)]
    preset: Option<PresetArg>,
    /// Resampler quality (tap count / FFT size parameter). With `--preset`, defaults to the preset; otherwise defaults to 2048.
    #[arg(long)]
    quality: Option<usize>,
    /// Normalized low-pass bandwidth in [0.0, 1.0]. With `--preset`, defaults to the preset; otherwise defaults to 0.95.
    #[arg(long)]
    bandwidth: Option<f32>,
    /// Cutoff transition taper. With `--preset`, defaults to the preset's taper; otherwise defaults to `cosine`.
    #[arg(long, value_enum)]
    taper_type: Option<CliTaperType>,
    /// Taper alpha parameter. Used by all tapers except `planck`.
    /// With `--preset` and no `--taper-type`, overrides the preset taper's alpha.
    /// Otherwise defaults: kaiser_cdf/kbd 6, tanh 3, cosine 3.4375, beta_cdf 24.
    #[arg(long)]
    alpha: Option<f32>,
    /// Taper beta parameter. Used by `beta_cdf`.
    /// With `--preset` and no `--taper-type`, overrides the preset taper's beta.
    #[arg(long)]
    beta: Option<f32>,
    /// Frequency-dependent phase rotation in [-1.0, 1.0]. Defaults to 0.0.
    #[arg(long)]
    phase: Option<f32>,
    /// Phase rotation intensity in [0.0, 100.0]. Defaults to 40.0.
    #[arg(long)]
    phase_intensity: Option<f32>,
    /// Permit aliasing/imaging inside the low-pass transition band (similar to SoX `rate -a`).
    /// Widens the transition to reduce ringing at the cost of alias rejection; the passband is
    /// unchanged. Equivalent to `--alias-floor-db -3`.
    #[arg(short = 'a', long = "allow-aliasing", conflicts_with_all = ["alias_floor", "alias_floor_db"])]
    allow_aliasing: bool,
    /// Lowest frequency that may receive folded/imaged energy, as a fraction of the lower Nyquist
    /// in [bandwidth, 1.0]. 1.0 (default) disables aliasing.
    #[arg(long = "alias-floor", conflicts_with = "alias_floor_db")]
    alias_floor: Option<f32>,
    /// Fold/image only down to the frequency where the filter response is this many dB (<= 0).
    #[arg(long = "alias-floor-db", allow_negative_numbers = true)]
    alias_floor_db: Option<f32>,
    /// Enable 2:1 pre-decimation stages ahead of the FFT resampler for large downsampling ratios.
    /// With `--preset`, defaults to the preset; otherwise defaults to false.
    #[arg(long)]
    decimate: Option<bool>,
    /// Use a high-precision FFT backend. Much slower; intended for extreme quality.
    /// `doubledouble` (~106-bit) is the fastest; `f256` (~237-bit) is the most precise.
    /// Only compatible with f64 processing.
    #[arg(long = "high-precision", value_enum, conflicts_with = "f32")]
    high_precision: Option<HighPrecisionArg>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum PresetArg {
    /// `ardftsrc::PRESET_FAST`: low latency.
    Fast,
    /// `ardftsrc::PRESET_GOOD`: balanced.
    Good,
    /// `ardftsrc::PRESET_HIGH`: high quality.
    High,
    /// `ardftsrc::PRESET_EXTREME`: maximum quality.
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
    /// Cumulative distribution of a Kaiser window (amplitude-complementary).
    #[value(name = "kaiser_cdf", aliases = ["kaiser-cdf", "bessel"])]
    KaiserCdf,
    /// Descending half of a Kaiser-Bessel-derived window (power-complementary).
    #[value(name = "kbd", aliases = ["kaiser_bessel_derived", "kaiser-bessel-derived"])]
    KaiserBesselDerived,
    /// Endpoint-normalized hyperbolic tangent transition.
    Tanh,
    Cosine,
    #[value(name = "beta_cdf", alias = "beta-cdf")]
    BetaCdf,
}

impl CliTaperType {
    fn slug(self) -> &'static str {
        match self {
            Self::Planck => "planck",
            Self::KaiserCdf => "kaiser_cdf",
            Self::KaiserBesselDerived => "kbd",
            Self::Tanh => "tanh",
            Self::Cosine => "cosine",
            Self::BetaCdf => "beta_cdf",
        }
    }
}

const DEFAULT_ALLOW_ALIASING_DB: f32 = -3.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum HighPrecisionArg {
    /// Double-double (~106-bit mantissa).
    #[value(name = "doubledouble", alias = "double-double")]
    DoubleDouble,
    /// IEEE binary128 (113-bit mantissa).
    F128,
    /// IEEE binary256 (237-bit mantissa).
    F256,
}

impl HighPrecisionArg {
    fn slug(self) -> &'static str {
        match self {
            Self::DoubleDouble => "doubledouble",
            Self::F128 => "f128",
            Self::F256 => "f256",
        }
    }
}

impl From<HighPrecisionArg> for HighPrecision {
    fn from(arg: HighPrecisionArg) -> Self {
        match arg {
            HighPrecisionArg::DoubleDouble => Self::DoubleDouble,
            HighPrecisionArg::F128 => Self::F128,
            HighPrecisionArg::F256 => Self::F256,
        }
    }
}

fn build_alias_floor(
    allow_aliasing: bool,
    alias_floor: Option<f32>,
    alias_floor_db: Option<f32>,
    default: AliasFloor,
) -> AliasFloor {
    if allow_aliasing {
        AliasFloor::Decibels(DEFAULT_ALLOW_ALIASING_DB)
    } else if let Some(fraction) = alias_floor {
        AliasFloor::Fraction(fraction)
    } else if let Some(db) = alias_floor_db {
        AliasFloor::Decibels(db)
    } else {
        default
    }
}

fn validate_alias_floor(alias_floor: AliasFloor, bandwidth: f32) {
    match alias_floor {
        AliasFloor::Fraction(fraction) => {
            if !fraction.is_finite() || fraction < bandwidth || fraction > 1.0 {
                eprintln!("--alias-floor must be finite and in [bandwidth, 1.0]");
                std::process::exit(2);
            }
        }
        AliasFloor::Decibels(db) => {
            if !db.is_finite() || db > 0.0 {
                eprintln!("--alias-floor-db must be finite and <= 0.0");
                std::process::exit(2);
            }
        }
    }
}

fn alias_floor_slug(alias_floor: AliasFloor) -> String {
    match alias_floor {
        AliasFloor::Fraction(fraction) => format!("-af{fraction:.4}"),
        AliasFloor::Decibels(db) => format!("-afdb{db:.1}"),
    }
}

fn build_taper_type(taper_type: CliTaperType, alpha: Option<f32>, beta: Option<f32>) -> TaperType {
    match taper_type {
        CliTaperType::Planck => {
            if alpha.is_some() || beta.is_some() {
                eprintln!(
                    "--alpha is not supported by --taper-type=planck; --beta requires --taper-type=beta_cdf"
                );
                std::process::exit(2);
            }
            TaperType::Planck
        }
        CliTaperType::KaiserCdf | CliTaperType::KaiserBesselDerived | CliTaperType::Tanh => {
            if beta.is_some() {
                eprintln!("--beta can only be used with --taper-type=beta_cdf");
                std::process::exit(2);
            }
            let alpha = alpha.unwrap_or(if matches!(taper_type, CliTaperType::Tanh) {
                3.0
            } else {
                6.0
            });
            if alpha <= 0.0 || !alpha.is_finite() {
                eprintln!(
                    "--alpha must be finite and > 0 when --taper-type={}",
                    taper_type.slug()
                );
                std::process::exit(2);
            }
            match taper_type {
                CliTaperType::KaiserCdf => TaperType::KaiserCdf(alpha),
                CliTaperType::KaiserBesselDerived => TaperType::KaiserBesselDerived(alpha),
                CliTaperType::Tanh => TaperType::Tanh(alpha),
                _ => unreachable!(),
            }
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

/// Splits a taper into its CLI kind and parameters, so CLI overrides can be applied on top of it.
fn taper_parts(taper_type: TaperType) -> (CliTaperType, Option<f32>, Option<f32>) {
    match taper_type {
        TaperType::Planck => (CliTaperType::Planck, None, None),
        TaperType::KaiserCdf(alpha) => (CliTaperType::KaiserCdf, Some(alpha), None),
        TaperType::KaiserBesselDerived(alpha) => {
            (CliTaperType::KaiserBesselDerived, Some(alpha), None)
        }
        TaperType::Tanh(alpha) => (CliTaperType::Tanh, Some(alpha), None),
        TaperType::Cosine(alpha) => (CliTaperType::Cosine, Some(alpha), None),
        TaperType::BetaCdf { alpha, beta } => (CliTaperType::BetaCdf, Some(alpha), Some(beta)),
    }
}

/// An explicit `--taper-type` wins; otherwise the preset's taper is used with `--alpha` /
/// `--beta` overriding its parameters; with neither, the taper is `cosine`.
fn resolve_taper_type(
    taper_type: Option<CliTaperType>,
    alpha: Option<f32>,
    beta: Option<f32>,
    preset_taper: Option<TaperType>,
) -> TaperType {
    match (taper_type, preset_taper) {
        (Some(taper_type), _) => build_taper_type(taper_type, alpha, beta),
        (None, Some(preset_taper)) => {
            let (kind, preset_alpha, preset_beta) = taper_parts(preset_taper);
            build_taper_type(kind, alpha.or(preset_alpha), beta.or(preset_beta))
        }
        (None, None) => build_taper_type(CliTaperType::Cosine, alpha, beta),
    }
}

fn main() -> Result<(), HydrogenError> {
    let cli = Args::parse();

    let float_variant = if cli.f32 {
        FloatVariant::F32
    } else {
        FloatVariant::F64
    };

    let (quality, bandwidth, phase, phase_intensity, alias_floor, decimate) = match cli.preset {
        Some(preset) => {
            let base = preset.base_config();
            (
                cli.quality.unwrap_or(base.quality),
                cli.bandwidth.unwrap_or(base.bandwidth),
                cli.phase.unwrap_or(base.phase),
                cli.phase_intensity.unwrap_or(base.phase_intensity),
                build_alias_floor(
                    cli.allow_aliasing,
                    cli.alias_floor,
                    cli.alias_floor_db,
                    base.alias_floor,
                ),
                cli.decimate.unwrap_or(base.decimate),
            )
        }
        None => (
            cli.quality.unwrap_or(2048),
            cli.bandwidth.unwrap_or(0.95),
            cli.phase.unwrap_or(Config::DEFAULT.phase),
            cli.phase_intensity
                .unwrap_or(Config::DEFAULT.phase_intensity),
            build_alias_floor(
                cli.allow_aliasing,
                cli.alias_floor,
                cli.alias_floor_db,
                Config::DEFAULT.alias_floor,
            ),
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

    validate_alias_floor(alias_floor, bandwidth);

    let high_precision = cli.high_precision;
    if high_precision.is_some() && matches!(float_variant, FloatVariant::F32) {
        eprintln!("--high-precision is not compatible with --f32; use --f64 instead");
        std::process::exit(2);
    }

    let taper_type = resolve_taper_type(
        cli.taper_type,
        cli.alpha,
        cli.beta,
        cli.preset.map(|preset| preset.base_config().taper_type),
    );
    let taper_slug = taper_parts(taper_type).0.slug();
    let taper_param_slug = match taper_type {
        TaperType::KaiserCdf(alpha)
        | TaperType::KaiserBesselDerived(alpha)
        | TaperType::Tanh(alpha) => format!("-a{alpha:.2}"),
        TaperType::Cosine(alpha) => format!("-a{alpha:.2}"),
        TaperType::BetaCdf { alpha, beta } => format!("-a{alpha:.2}-b{beta:.2}"),
        TaperType::Planck => String::new(),
    };
    let phase_slug = format!("-p{phase:.3}-pi{phase_intensity:.1}");
    let alias_floor_slug = alias_floor_slug(alias_floor);
    let decimate_slug = if decimate { "-dec1" } else { "-dec0" };
    let high_precision_slug = match high_precision {
        Some(precision) => format!("-hp-{}", precision.slug()),
        None => "-hp-off".to_string(),
    };

    let output_label = match cli.preset {
        Some(preset) => format!(
            "output-ardftsrc-preset-{}-q{quality}-bw{bandwidth:.4}-t{taper_slug}{taper_param_slug}{phase_slug}{alias_floor_slug}{decimate_slug}{high_precision_slug}",
            preset.slug()
        ),
        None => format!(
            "output-ardftsrc-q{quality}-bw{bandwidth:.4}-t{taper_slug}{taper_param_slug}{phase_slug}{alias_floor_slug}{decimate_slug}{high_precision_slug}"
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
                        alias_floor,
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
                        alias_floor,
                        decimate,
                        high_precision,
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
            alias_floor,
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
            alias_floor,
            decimate,
            high_precision,
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
    alias_floor: AliasFloor,
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
        alias_floor,
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
    alias_floor: AliasFloor,
    decimate: bool,
    high_precision: Option<HighPrecisionArg>,
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
        alias_floor,
        decimate,
        ..Config::default()
    }
    .with_high_precision(high_precision.map(HighPrecisionArg::into));

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

#[cfg(test)]
mod taper_tests {
    use super::*;

    #[test]
    fn new_tapers_parse_and_use_expected_parameters() {
        for (name, slug, default, custom) in [
            (
                "kaiser_cdf",
                "kaiser_cdf",
                TaperType::KaiserCdf(6.0),
                TaperType::KaiserCdf(2.5),
            ),
            (
                "bessel",
                "kaiser_cdf",
                TaperType::KaiserCdf(6.0),
                TaperType::KaiserCdf(2.5),
            ),
            (
                "kbd",
                "kbd",
                TaperType::KaiserBesselDerived(6.0),
                TaperType::KaiserBesselDerived(2.5),
            ),
            (
                "kaiser_bessel_derived",
                "kbd",
                TaperType::KaiserBesselDerived(6.0),
                TaperType::KaiserBesselDerived(2.5),
            ),
            ("tanh", "tanh", TaperType::Tanh(3.0), TaperType::Tanh(2.5)),
        ] {
            let args = Args::try_parse_from(["test", "--taper-type", name]).unwrap();
            assert_eq!(args.taper_type.unwrap().slug(), slug);
            assert_eq!(
                resolve_taper_type(args.taper_type, args.alpha, args.beta, None),
                default
            );
            let args =
                Args::try_parse_from(["test", "--taper-type", name, "--alpha", "2.5"]).unwrap();
            assert_eq!(
                resolve_taper_type(args.taper_type, args.alpha, args.beta, None),
                custom
            );
        }
    }

    fn resolve_from(argv: &[&str]) -> TaperType {
        let args = Args::try_parse_from(argv).unwrap();
        resolve_taper_type(
            args.taper_type,
            args.alpha,
            args.beta,
            args.preset.map(|preset| preset.base_config().taper_type),
        )
    }

    #[test]
    fn preset_taper_is_used_when_taper_type_is_omitted() {
        for preset in [PresetArg::Fast, PresetArg::Good, PresetArg::High, PresetArg::Extreme] {
            assert_eq!(
                resolve_from(&["test", "--preset", preset.slug()]),
                preset.base_config().taper_type
            );
        }
    }

    #[test]
    fn alpha_and_beta_override_preset_taper_parameters() {
        let TaperType::BetaCdf { alpha, beta } = PRESET_GOOD.taper_type else {
            panic!("test assumes PRESET_GOOD uses a beta_cdf taper");
        };
        assert_eq!(
            resolve_from(&["test", "--preset", "good", "--alpha", "21"]),
            TaperType::BetaCdf { alpha: 21.0, beta }
        );
        assert_eq!(
            resolve_from(&["test", "--preset", "good", "--beta", "22"]),
            TaperType::BetaCdf { alpha, beta: 22.0 }
        );
    }

    #[test]
    fn explicit_taper_type_overrides_preset_taper() {
        assert_eq!(
            resolve_from(&["test", "--preset", "high", "--taper-type", "cosine"]),
            TaperType::Cosine(3.4375)
        );
    }

    #[test]
    fn taper_defaults_to_cosine_without_preset() {
        assert_eq!(resolve_from(&["test"]), TaperType::Cosine(3.4375));
    }

    #[test]
    fn high_precision_parses_expected_values() {
        for (name, expected) in [
            ("doubledouble", HighPrecisionArg::DoubleDouble),
            ("double-double", HighPrecisionArg::DoubleDouble),
            ("f128", HighPrecisionArg::F128),
            ("f256", HighPrecisionArg::F256),
        ] {
            let args = Args::try_parse_from(["test", "--high-precision", name]).unwrap();
            assert_eq!(args.high_precision, Some(expected));
            assert_eq!(
                expected.slug(),
                match expected {
                    HighPrecisionArg::DoubleDouble => "doubledouble",
                    HighPrecisionArg::F128 => "f128",
                    HighPrecisionArg::F256 => "f256",
                }
            );
        }

        let args = Args::try_parse_from(["test"]).unwrap();
        assert_eq!(args.high_precision, None);
    }

    #[test]
    fn high_precision_rejects_unknown_backend() {
        assert!(Args::try_parse_from(["test", "--high-precision", "f64"]).is_err());
    }

    #[test]
    fn high_precision_conflicts_with_f32() {
        assert!(Args::try_parse_from(["test", "--f32", "--high-precision", "f128"]).is_err());
    }
}

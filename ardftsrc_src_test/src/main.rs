use std::path::PathBuf;

use ardftsrc::{Config, InterleavedResampler, TaperType};
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
    #[arg(long, default_value_t = 2048)]
    quality: usize,
    #[arg(long, default_value_t = 0.95)]
    bandwidth: f32,
    #[arg(long, value_enum, default_value_t = CliTaperType::Cosine)]
    taper_type: CliTaperType,
    #[arg(long)]
    alpha: Option<f32>,
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
    let taper_type = build_taper_type(cli.taper_type, cli.alpha);
    let taper_slug = cli.taper_type.slug();
    let alpha_slug = match taper_type {
        TaperType::Cosine(alpha) => format!("-a{alpha:.2}"),
        TaperType::Planck => String::new(),
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

    let mut hydrogen = HydrogenSrc::new(
        cli.workdir,
        float_variant,
        &format!("output-ardftsrc-q{quality}-bw{bandwidth:.4}-t{taper_slug}{alpha_slug}"),
    );

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

    let mut resampler = InterleavedResampler::<f32>::new(config)
        .expect("failed to create ardftsrc f32 resampler");
    resampler
        .process_all(&request.samples)
        .expect("failed during ardftsrc f32 processing")
        .interleave()
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

    let mut resampler = InterleavedResampler::<f64>::new(config)
        .expect("failed to create ardftsrc f64 resampler");
    resampler
        .process_all(&request.samples)
        .expect("failed during ardftsrc f64 processing")
        .interleave()
}

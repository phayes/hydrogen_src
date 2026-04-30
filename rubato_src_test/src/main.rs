use std::path::PathBuf;

use clap::{ArgGroup, Parser};
use hydrogen_src::{
    FloatVariant, HydrogenError, HydrogenSrc, LocalHarness, ResampleRequestF32, ResampleRequestF64,
};
use rubato_dsp::audioadapter_buffers::direct::InterleavedSlice;
use rubato_dsp::{Fft, FixedSync, Resampler};

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

    // Rubato options
    #[arg(long, default_value_t = 512)]
    chunk_size: usize,
    #[arg(long, default_value_t = 1)]
    sub_chunk: usize,
}

fn main() -> Result<(), HydrogenError> {
    let cli = Args::parse();

    let float_variant = if cli.f64 {
        FloatVariant::F64
    } else {
        FloatVariant::F32
    };

    if cli.chunk_size == 0 || cli.sub_chunk == 0 {
        eprintln!("--chunk-size and --sub-chunk must be > 0");
        std::process::exit(2);
    }
    let chunk_size = cli.chunk_size;
    let sub_chunk = cli.sub_chunk;

    // Local mode
    if cli.local {
        let mut local = LocalHarness::new(cli.workdir, LOCAL_SCRIPT_DIR);
        match float_variant {
            FloatVariant::F32 => {
                local.set_callback_f32(move |request: ResampleRequestF32| -> Vec<f32> {
                    run_rubato_f32(request, chunk_size, sub_chunk)
                });
            }
            FloatVariant::F64 => {
                local.set_callback_f64(move |request: ResampleRequestF64| -> Vec<f64> {
                    run_rubato_f64(request, chunk_size, sub_chunk)
                });
            }
        }
        let results = local.run()?;
        if cli.json {
            println!(
                "{}",
                serde_json::to_string_pretty(&results).expect("failed to serialize local results")
            );
        } else {
            println!("{results}");
        }
        return Ok(());
    // Remote upload mode (default)
    } else {
        let mut hydrogen = HydrogenSrc::new(
            cli.workdir,
            float_variant,
            &format!("output-rubato-{}-{}", chunk_size, sub_chunk),
        );

        hydrogen.set_callback_f32(move |request: ResampleRequestF32| -> Vec<f32> {
            run_rubato_f32(request, chunk_size, sub_chunk)
        });
        hydrogen.set_callback_f64(move |request: ResampleRequestF64| -> Vec<f64> {
            run_rubato_f64(request, chunk_size, sub_chunk)
        });

        let _ = hydrogen.run()?;
        Ok(())
    }
}

fn run_rubato_f32(request: ResampleRequestF32, chunk_size: usize, sub_chunk: usize) -> Vec<f32> {
    if request.channels == 0 {
        eprintln!("channel count must be > 0");
        std::process::exit(2);
    }
    let mut resampler = Fft::<f32>::new(
        request.sample_rate as usize,
        request.target_sample_rate as usize,
        chunk_size,
        sub_chunk,
        request.channels,
        FixedSync::Input,
    )
    .expect("failed to create rubato f32 resampler");

    // Set up input adapter
    let input_frames = request.samples.len() / request.channels;
    let input_adapter = InterleavedSlice::new(&request.samples, request.channels, input_frames)
        .expect("failed to build f32 input adapter");

    // Set up output adapter
    let output_frames = resampler.process_all_needed_output_len(input_frames);
    let mut output = Vec::with_capacity(output_frames * request.channels);
    output.resize(output_frames * request.channels, 0.0f32);
    let mut output_adapter =
        InterleavedSlice::new_mut(&mut output, request.channels, output_frames)
            .expect("failed to build f32 output adapter");

    // Process
    let (_, written_frames) = resampler
        .process_all_into_buffer(&input_adapter, &mut output_adapter, input_frames, None)
        .expect("failed during rubato f32 processing");

    // Truncate output to the actual number of frames written (shouldn't actually do anything, but just in case)
    output.truncate(written_frames * request.channels);

    // Return output
    output
}

fn run_rubato_f64(request: ResampleRequestF64, chunk_size: usize, sub_chunk: usize) -> Vec<f64> {
    if request.channels == 0 {
        eprintln!("channel count must be > 0");
        std::process::exit(2);
    }
    let mut resampler = Fft::<f64>::new(
        request.sample_rate as usize,
        request.target_sample_rate as usize,
        chunk_size,
        sub_chunk,
        request.channels,
        FixedSync::Input,
    )
    .expect("failed to create rubato f64 resampler");

    // Set up input adapter
    let input_frames = request.samples.len() / request.channels;
    let input_adapter = InterleavedSlice::new(&request.samples, request.channels, input_frames)
        .expect("failed to build f64 input adapter");

    // Set up output adapter
    let output_frames = resampler.process_all_needed_output_len(input_frames);
    let mut output = Vec::with_capacity(output_frames * request.channels);
    output.resize(output_frames * request.channels, 0.0f64);
    let mut output_adapter =
        InterleavedSlice::new_mut(&mut output, request.channels, output_frames)
            .expect("failed to build f64 output adapter");

    // Process
    let (_, written_frames) = resampler
        .process_all_into_buffer(&input_adapter, &mut output_adapter, input_frames, None)
        .expect("failed during rubato f64 processing");

    // Truncate output to the actual number of frames written (shouldn't actually do anything, but just in case)
    output.truncate(written_frames * request.channels);

    // Return output
    output
}

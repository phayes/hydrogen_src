use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use reqwest::blocking::Client;
use thiserror::Error;
use wavers::{Wav, write};
use zip::write::SimpleFileOptions;

pub mod local;
pub use local::LocalHarness;
pub use local::LocalTestResults;

const TARGET_OUTPUT_SAMPLE_RATE: usize = 44_100;

#[derive(Debug, Clone, Copy)]
pub enum FloatVariant {
    F32,
    F64,
}

impl FloatVariant {
    fn sample_pack_filename(self) -> &'static str {
        match self {
            Self::F32 => "src-test-32bitfloat.zip",
            Self::F64 => "src-test-64bitfloat.zip",
        }
    }

    fn sample_pack_url(self) -> &'static str {
        match self {
            Self::F32 => "https://src.hydrogenaudio.org/testsamples/src-test-32bitfloat.zip",
            Self::F64 => "https://src.hydrogenaudio.org/testsamples/src-test-64bitfloat.zip",
        }
    }

    fn extracted_sample_pack_dir(self) -> &'static str {
        match self {
            Self::F32 => "32bitfloat",
            Self::F64 => "64bitfloat",
        }
    }

    fn slug(self) -> &'static str {
        match self {
            Self::F32 => "f32",
            Self::F64 => "f64",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResampleRequestF32 {
    pub sample_rate: usize,
    pub channels: usize,
    pub samples: Vec<f32>,
    pub target_sample_rate: usize,
}

#[derive(Debug, Clone)]
pub struct ResampleRequestF64 {
    pub sample_rate: usize,
    pub channels: usize,
    pub samples: Vec<f64>,
    pub target_sample_rate: usize,
}

pub type ResamplerCallbackF32 = dyn Fn(ResampleRequestF32) -> Vec<f32>;
pub type ResamplerCallbackF64 = dyn Fn(ResampleRequestF64) -> Vec<f64>;

pub struct HydrogenSrc {
    work_dir: PathBuf,
    float_variant: FloatVariant,
    output_zip_basename: String,
    callback_f32: Option<Box<ResamplerCallbackF32>>,
    callback_f64: Option<Box<ResamplerCallbackF64>>,
}

impl HydrogenSrc {
    pub fn new(
        work_dir: impl Into<PathBuf>,
        float_variant: FloatVariant,
        output_prefix: &str,
    ) -> Self {
        Self {
            work_dir: work_dir.into(),
            float_variant,
            output_zip_basename: output_prefix.to_string(),
            callback_f32: None,
            callback_f64: None,
        }
    }

    pub fn set_callback_f32<F>(&mut self, callback: F)
    where
        F: Fn(ResampleRequestF32) -> Vec<f32> + 'static,
    {
        self.callback_f32 = Some(Box::new(callback));
    }

    pub fn set_callback_f64<F>(&mut self, callback: F)
    where
        F: Fn(ResampleRequestF64) -> Vec<f64> + 'static,
    {
        self.callback_f64 = Some(Box::new(callback));
    }

    pub fn run(&mut self) -> Result<PathBuf, HydrogenError> {
        fs::create_dir_all(self.absolute_work_dir()?)?;
        self.ensure_sample_pack_downloaded()?;
        self.ensure_sample_pack_extracted()?;
        self.prepare_output_dir()?;

        match self.float_variant {
            FloatVariant::F32 => {
                let callback = self
                    .callback_f32
                    .take()
                    .ok_or(HydrogenError::MissingCallbackF32)?;
                self.run_f32(callback.as_ref())?;
                self.callback_f32 = Some(callback);
            }
            FloatVariant::F64 => {
                let callback = self
                    .callback_f64
                    .take()
                    .ok_or(HydrogenError::MissingCallbackF64)?;
                self.run_f64(callback.as_ref())?;
                self.callback_f64 = Some(callback);
            }
        }

        self.package_output_directory()?;
        let output_zip = self.reported_output_zip_path();
        println!("Output zip: {}", output_zip.display());
        println!("Upload for analysis: https://src.hydrogenaudio.org/upload");
        Ok(output_zip)
    }

    fn run_f32(&self, callback: &ResamplerCallbackF32) -> Result<(), HydrogenError> {
        for input_file in list_wavs(&self.sample_pack_dir()?)? {
            let mut wav = Wav::<f32>::from_path(&input_file)?;
            let request = ResampleRequestF32 {
                sample_rate: wav.sample_rate() as usize,
                channels: wav.n_channels() as usize,
                samples: wav.read()?.to_vec(),
                target_sample_rate: TARGET_OUTPUT_SAMPLE_RATE,
            };
            let output_sample_rate = request.target_sample_rate as i32;
            let output_channels = request.channels as u16;
            let output_samples = callback(request);
            write(
                &self.output_dir()?.join(file_name_from_path(&input_file)?),
                &output_samples,
                output_sample_rate,
                output_channels,
            )?;
        }
        Ok(())
    }

    fn run_f64(&self, callback: &ResamplerCallbackF64) -> Result<(), HydrogenError> {
        for input_file in list_wavs(&self.sample_pack_dir()?)? {
            let mut wav = Wav::<f64>::from_path(&input_file)?;
            let request = ResampleRequestF64 {
                sample_rate: wav.sample_rate() as usize,
                channels: wav.n_channels() as usize,
                samples: wav.read()?.to_vec(),
                target_sample_rate: TARGET_OUTPUT_SAMPLE_RATE,
            };
            let output_sample_rate = request.target_sample_rate as i32;
            let output_channels = request.channels as u16;
            let output_samples = callback(request);
            write(
                &self.output_dir()?.join(file_name_from_path(&input_file)?),
                &output_samples,
                output_sample_rate,
                output_channels,
            )?;
        }
        Ok(())
    }

    fn ensure_sample_pack_downloaded(&self) -> Result<(), HydrogenError> {
        if self.sample_pack_path()?.exists() {
            return Ok(());
        }

        let client = Client::new();
        let bytes = client
            .get(self.float_variant.sample_pack_url())
            .send()?
            .error_for_status()?
            .bytes()?;

        let mut file = File::create(self.sample_pack_path()?)?;
        file.write_all(&bytes)?;
        Ok(())
    }

    fn ensure_sample_pack_extracted(&self) -> Result<(), HydrogenError> {
        if self.sample_pack_dir()?.exists() {
            return Ok(());
        }

        let zip_file = File::open(self.sample_pack_path()?)?;
        let mut sample_pack_archive = zip::ZipArchive::new(zip_file)?;
        sample_pack_archive.extract(self.absolute_work_dir()?)?;

        Ok(())
    }

    fn prepare_output_dir(&self) -> Result<(), HydrogenError> {
        let output_dir = self.output_dir()?;
        if output_dir.exists() {
            fs::remove_dir_all(&output_dir)?;
        }
        fs::create_dir_all(&output_dir)?;
        Ok(())
    }

    fn package_output_directory(&self) -> Result<(), HydrogenError> {
        let output_dir = self.output_dir()?;
        let output_zip = self.output_zip_path()?;
        if output_zip.exists() {
            fs::remove_file(&output_zip)?;
        }

        let file = File::create(&output_zip)?;
        let mut writer = zip::ZipWriter::new(file);
        let options = SimpleFileOptions::default();

        for output_file in list_wavs(&output_dir)? {
            let rel_path =
                output_file
                    .strip_prefix(&output_dir)
                    .map_err(|_| HydrogenError::PathPrefix {
                        input: output_file.clone(),
                        base: output_dir.to_path_buf(),
                    })?;

            writer.start_file_from_path(rel_path, options)?;
            let mut input = File::open(output_file)?;
            io::copy(&mut input, &mut writer)?;
        }

        writer.finish()?;
        Ok(())
    }

    fn absolute_work_dir(&self) -> Result<PathBuf, HydrogenError> {
        if self.work_dir.is_absolute() {
            Ok(self.work_dir.clone())
        } else {
            Ok(std::env::current_dir()?.join(&self.work_dir))
        }
    }

    fn sample_pack_path(&self) -> Result<PathBuf, HydrogenError> {
        Ok(self
            .absolute_work_dir()?
            .join(self.float_variant.sample_pack_filename()))
    }

    fn sample_pack_dir(&self) -> Result<PathBuf, HydrogenError> {
        Ok(self
            .absolute_work_dir()?
            .join(self.float_variant.extracted_sample_pack_dir()))
    }

    fn output_dir(&self) -> Result<PathBuf, HydrogenError> {
        Ok(self.absolute_work_dir()?.join("output"))
    }

    fn output_zip_path(&self) -> Result<PathBuf, HydrogenError> {
        Ok(self.absolute_work_dir()?.join(format!(
            "{}-{}.zip",
            self.output_zip_basename,
            self.float_variant.slug()
        )))
    }

    fn reported_output_zip_path(&self) -> PathBuf {
        self.work_dir.join(format!(
            "{}-{}.zip",
            self.output_zip_basename,
            self.float_variant.slug()
        ))
    }
}

pub(crate) fn list_wavs(dir: &Path) -> Result<Vec<PathBuf>, HydrogenError> {
    let mut wavs = Vec::new();
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_file()
            && path
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("wav"))
        {
            wavs.push(path);
        }
    }

    wavs.sort();
    Ok(wavs)
}

fn file_name_from_path(path: &Path) -> Result<PathBuf, HydrogenError> {
    path.file_name()
        .map(PathBuf::from)
        .ok_or_else(|| HydrogenError::MissingFileName(path.to_path_buf()))
}

#[derive(Debug, Error)]
pub enum HydrogenError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("ZIP error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("WAV error: {0}")]
    Wav(#[from] wavers::WaversError),
    #[error("failed to strip prefix '{base}' from path '{input}'")]
    PathPrefix { input: PathBuf, base: PathBuf },
    #[error("missing callback for f32 variant")]
    MissingCallbackF32,
    #[error("missing callback for f64 variant")]
    MissingCallbackF64,
    #[error("path has no file name: '{0}'")]
    MissingFileName(PathBuf),
    #[error("missing callback for local harness")]
    MissingLocalCallback,
    #[error("only one local callback can be set at a time")]
    ConflictingLocalCallbacks,
    #[error("octave command was not found on PATH")]
    MissingOctaveCommand,
    #[error("required octave packages are missing: {0:?}")]
    MissingOctavePackages(Vec<String>),
    #[error("octave command failed during {context}: {stderr}")]
    OctaveCommandFailed { context: String, stderr: String },
    #[error("invalid script location: '{0}'")]
    InvalidScriptLocation(PathBuf),
    #[error("missing generated sample files in '{sample_dir}': {missing_files:?}")]
    MissingGeneratedSampleFiles {
        sample_dir: PathBuf,
        missing_files: Vec<String>,
    },
    #[error("quality metric file has no non-empty lines: '{path}'")]
    EmptyQualityMetric { path: PathBuf },
    #[error("failed to parse quality metric in '{path}' from '{value}'")]
    InvalidQualityMetric { path: PathBuf, value: String },
}

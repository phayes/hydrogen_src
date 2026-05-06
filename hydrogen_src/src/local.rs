use std::ffi::OsStr;
use std::fmt;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Serialize};
use wavers::Wav;

use crate::{
    FloatVariant, HydrogenError, ResampleRequestF32, ResampleRequestF64, ResamplerCallbackF32,
    ResamplerCallbackF64, list_wavs, write_f32_with_wav_encoding, write_f64_with_wav_encoding,
};

const TARGET_OUTPUT_SAMPLE_RATE: usize = 44_100;
const DEFAULT_SAMPLE_SUBDIR: &str = "local_test_generated_samples";
const DEFAULT_OUTPUT_SUBDIR: &str = "local_test_output";
const DEFAULT_ANALYSIS_OUTPUT_SUBDIR: &str = "analysis_output";
const CALCULATED_DELAY_FILENAME: &str = "calculateddelay.txt";
const REFERENCE_SPECTROGRAM_PNG: &str = "sweep-1-to-44KHz-1to11secHighRES-REF.png";
const GENERATOR_SUBDIR: &str = "TestSignals96KHzto44KHz";
const INTERNAL_IMPULSE_REFERENCE_WAV: &str = "impulse-64bitfloat-InternalUse.wav";
const INTERNAL_IMPULSE_REFERENCE_ZIP: &str = "impulse-64bitfloat-InternalUse.zip";
const GENERATION_SCRIPTS: [&str; 6] = [
    "GEN_aliasing150db.m",
    "GEN_bitdepthtest.m",
    "GEN_gaplesstest.m",
    "GEN_impulse_96KHz.m",
    "GEN_intermodulation_60Hz_and_7kHz.m",
    "GEN_sweep_1_to_48KHz_96KHz.m",
];
const ANALYSIS_SCRIPTS: [&str; 6] = [
    "aliasing150db.m",
    "bit_depth_test.m",
    "gaplesstest.m",
    "impulse.m",
    "intermodulation_harmonic_distortion.m",
    "spectogram_sweep_1_to_44kHz_96kHz_srcto_44kHz.m",
];

static PRECHECK_AND_GENERATION_DONE: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalTestResults {
    pub average_score: f64,
    #[serde(default)]
    pub balanced_score: f64,
    #[serde(default = "nan_f64")]
    pub spectrogram_score: f64,
    #[serde(default = "nan_f64")]
    pub bandwidth_score: f64,
    #[serde(default = "nan_f64")]
    pub impulse_freq_score: f64,
    #[serde(default = "nan_f64")]
    pub average_impulse_freq: f64,
    #[serde(default = "nan_f64")]
    pub alias_score: f64,
    #[serde(default = "nan_f64")]
    pub preringing_score: f64,
    #[serde(default = "nan_f64")]
    pub gapless_score: f64,
    #[serde(default = "nan_f64")]
    pub intermoddiff_score: f64,
    #[serde(default = "nan_f64")]
    pub delay_score: f64,
    #[serde(default = "nan_f64")]
    pub delay_samples: f64,
    pub figures: Vec<PathBuf>,
}

impl fmt::Display for LocalTestResults {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Average score: {}", self.average_score)?;
        writeln!(f, "Balanced score: {}", self.balanced_score)?;
        writeln!(
            f,
            "Average impulse freq: {} db",
            self.average_impulse_freq
        )?;
        writeln!(f, "Delay: {} samples", self.delay_samples)?;
        writeln!(f, "Scores:")?;
        writeln!(f, "- spectrogram: {}", self.spectrogram_score)?;
        writeln!(f, "- bandwidth: {}", self.bandwidth_score)?;
        writeln!(f, "- impulse_freq: {}", self.impulse_freq_score)?;
        writeln!(f, "- alias: {}", self.alias_score)?;
        writeln!(f, "- preringing: {}", self.preringing_score)?;
        writeln!(f, "- gapless: {}", self.gapless_score)?;
        writeln!(f, "- intermoddiff: {}", self.intermoddiff_score)?;
        writeln!(f, "- delay: {}", self.delay_score)?;
        writeln!(f, "Figures:")?;
        for figure in &self.figures {
            writeln!(f, "- {}", figure.display())?;
        }
        Ok(())
    }
}

impl LocalTestResults {
    pub fn from_analysis_dir(analysis_dir: impl Into<PathBuf>) -> Result<Self, HydrogenError> {
        let analysis_dir = absolutize_path(analysis_dir.into())?;
        if !analysis_dir.is_dir() {
            return Err(HydrogenError::InvalidScriptLocation(analysis_dir));
        }

        let mut spectrogram_score = f64::NAN;
        let mut bandwidth_score = f64::NAN;
        let mut impulse_freq_score = f64::NAN;
        let mut average_impulse_freq = f64::NAN;
        let mut alias_score = f64::NAN;
        let mut preringing_score = f64::NAN;
        let mut gapless_score = f64::NAN;
        let mut intermoddiff_score = f64::NAN;
        let mut delay_score = f64::NAN;
        let mut delay_samples = f64::NAN;
        let mut figures = Vec::new();

        for entry in fs::read_dir(&analysis_dir)? {
            let path = entry?.path();
            if !path.is_file() {
                continue;
            }

            let file_name = path
                .file_name()
                .and_then(OsStr::to_str)
                .ok_or_else(|| HydrogenError::MissingFileName(path.clone()))?;
            match file_name {
                "quality-spectrogram.txt" => {
                    spectrogram_score = parse_file(&path, 0)?;
                }
                "quality-bandwidth.txt" => {
                    bandwidth_score = parse_file(&path, 0)?;
                }
                "quality-impulse_freq.txt" => {
                    average_impulse_freq = parse_file(&path, 0)?;
                    impulse_freq_score = parse_file(&path, 1)?;
                }
                "quality-alias.txt" => {
                    alias_score = parse_file(&path, 0)?;
                }
                "quality-preringing.txt" => {
                    preringing_score = parse_file(&path, 0)?;
                }
                "quality-gapless.txt" => {
                    gapless_score = parse_file(&path, 0)?;
                }
                "quality-intermoddiff.txt" => {
                    intermoddiff_score = parse_file(&path, 0)?;
                }
                CALCULATED_DELAY_FILENAME => {
                    let raw_delay = parse_file(&path, 0)?;
                    delay_samples = raw_delay;
                    delay_score = delay_score_from_normalized(raw_delay);
                }
                _ => {}
            }

            if has_extension(&path, "png") {
                figures.push(absolutize_path(path)?);
            }
        }

        figures.sort();
        let quality_scores = [
            spectrogram_score,
            bandwidth_score,
            impulse_freq_score,
            average_impulse_freq,
            alias_score,
            preringing_score,
            gapless_score,
            intermoddiff_score,
        ];
        let average_score = average_quality_score(&quality_scores);
        let balanced_score = balanced_quality_score(
            spectrogram_score,
            bandwidth_score,
            impulse_freq_score,
            alias_score,
            preringing_score,
            gapless_score,
            intermoddiff_score,
            delay_score,
        );
        Ok(Self {
            figures,
            average_score,
            balanced_score,
            spectrogram_score,
            bandwidth_score,
            impulse_freq_score,
            average_impulse_freq,
            alias_score,
            preringing_score,
            gapless_score,
            intermoddiff_score,
            delay_score,
            delay_samples,
        })
    }

    pub fn from_workspace_dir(workspace_dir: impl Into<PathBuf>) -> Result<Self, HydrogenError> {
        let workspace_dir = absolutize_path(workspace_dir.into())?;
        let analysis_dir = workspace_dir.join(DEFAULT_ANALYSIS_OUTPUT_SUBDIR);
        Self::from_analysis_dir(analysis_dir)
    }
}

pub struct LocalHarness {
    workspace: PathBuf,
    script_dir: PathBuf,
    callback_f32: Option<Box<ResamplerCallbackF32>>,
    callback_f64: Option<Box<ResamplerCallbackF64>>,
}

impl LocalHarness {
    pub fn new(workspace: impl Into<PathBuf>, script_dir: impl Into<PathBuf>) -> Self {
        Self {
            workspace: workspace.into(),
            script_dir: script_dir.into(),
            callback_f32: None,
            callback_f64: None,
        }
    }

    pub fn set_callback_f32<F>(&mut self, callback: F)
    where
        F: Fn(ResampleRequestF32) -> Vec<f32> + Send + Sync + 'static,
    {
        self.callback_f32 = Some(Box::new(callback));
    }

    pub fn set_callback_f64<F>(&mut self, callback: F)
    where
        F: Fn(ResampleRequestF64) -> Vec<f64> + Send + Sync + 'static,
    {
        self.callback_f64 = Some(Box::new(callback));
    }

    pub fn run(&mut self) -> Result<LocalTestResults, HydrogenError> {
        fs::create_dir_all(self.workspace_dir()?)?;

        let has_f32 = self.callback_f32.is_some();
        let has_f64 = self.callback_f64.is_some();
        if has_f32 && has_f64 {
            return Err(HydrogenError::ConflictingLocalCallbacks);
        }
        if !has_f32 && !has_f64 {
            return Err(HydrogenError::MissingLocalCallback);
        }

        self.ensure_precheck()?;

        if has_f32 {
            let callback = self
                .callback_f32
                .take()
                .ok_or(HydrogenError::MissingLocalCallback)?;
            self.run_f32(callback.as_ref())?;
            self.copy_outputs_without_suffixes()?;
            self.copy_references()?;
            self.callback_f32 = Some(callback);
            self.run_analysis()?;
            return LocalTestResults::from_analysis_dir(self.analysis_output_dir()?);
        }

        if has_f64 {
            let callback = self
                .callback_f64
                .take()
                .ok_or(HydrogenError::MissingLocalCallback)?;
            self.run_f64(callback.as_ref())?;
            self.copy_outputs_without_suffixes()?;
            self.copy_references()?;
            self.callback_f64 = Some(callback);
            self.run_analysis()?;
            return LocalTestResults::from_analysis_dir(self.analysis_output_dir()?);
        }

        Err(HydrogenError::MissingLocalCallback)
    }

    fn ensure_precheck(&self) -> Result<(), HydrogenError> {
        if PRECHECK_AND_GENERATION_DONE
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Ok(());
        }

        self.prepare_output_dir()?;
        self.prepare_analysis_output_dir()?;
        self.check_octave_command()?;
        self.check_octave_packages()?;
        self.ensure_internal_impulse_reference_wav()?;
        self.ensure_sample_wavs_generated()?;
        Ok(())
    }

    fn run_f32(&self, callback: &ResamplerCallbackF32) -> Result<(), HydrogenError> {
        for input_file in list_wavs(&self.sample_input_dir(FloatVariant::F32)?)? {
            let mut wav = Wav::<f32>::from_path(&input_file)?;
            let input_encoding = wav.encoding();
            let request = ResampleRequestF32 {
                sample_rate: wav.sample_rate() as usize,
                channels: wav.n_channels() as usize,
                samples: wav.read()?.to_vec(),
                target_sample_rate: TARGET_OUTPUT_SAMPLE_RATE,
            };
            let output_sample_rate = request.target_sample_rate as i32;
            let output_channels = request.channels as u16;
            let output_samples = callback(request);
            write_f32_with_wav_encoding(
                &self.output_dir()?.join(file_name_from_path(&input_file)?),
                &output_samples,
                output_sample_rate,
                output_channels,
                input_encoding,
            )?;
        }
        Ok(())
    }

    fn run_f64(&self, callback: &ResamplerCallbackF64) -> Result<(), HydrogenError> {
        for input_file in list_wavs(&self.sample_input_dir(FloatVariant::F64)?)? {
            let mut wav = Wav::<f64>::from_path(&input_file)?;
            let input_encoding = wav.encoding();
            let request = ResampleRequestF64 {
                sample_rate: wav.sample_rate() as usize,
                channels: wav.n_channels() as usize,
                samples: wav.read()?.to_vec(),
                target_sample_rate: TARGET_OUTPUT_SAMPLE_RATE,
            };
            let output_sample_rate = request.target_sample_rate as i32;
            let output_channels = request.channels as u16;
            let output_samples = callback(request);
            write_f64_with_wav_encoding(
                &self.output_dir()?.join(file_name_from_path(&input_file)?),
                &output_samples,
                output_sample_rate,
                output_channels,
                input_encoding,
            )?;
        }
        Ok(())
    }

    fn check_octave_command(&self) -> Result<(), HydrogenError> {
        let output = Command::new("octave").arg("--version").output();
        let output = match output {
            Ok(output) => output,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(HydrogenError::MissingOctaveCommand);
            }
            Err(error) => return Err(HydrogenError::Io(error)),
        };

        if !output.status.success() {
            return Err(HydrogenError::OctaveCommandFailed {
                context: "octave --version".to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }

        Ok(())
    }

    fn check_octave_packages(&self) -> Result<(), HydrogenError> {
        let output = Command::new("octave")
            .args(["--silent", "--no-gui", "--quiet", "--eval"])
            .arg("pkgs = pkg('list'); for i = 1:numel(pkgs), disp(pkgs{i}.name); end")
            .output()?;

        if !output.status.success() {
            return Err(HydrogenError::OctaveCommandFailed {
                context: "octave package check".to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }

        let listed_packages = String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();

        let mut missing_packages = Vec::new();
        for required in ["signal", "image"] {
            if !listed_packages.iter().any(|pkg| pkg == required) {
                missing_packages.push(required.to_string());
            }
        }

        if !missing_packages.is_empty() {
            return Err(HydrogenError::MissingOctavePackages(missing_packages));
        }

        Ok(())
    }

    fn ensure_sample_wavs_generated(&self) -> Result<(), HydrogenError> {
        let sample_dir = self.sample_dir()?;
        if sample_dir.is_dir() && dir_has_entries(&sample_dir)? {
            return Ok(());
        }

        fs::create_dir_all(&sample_dir)?;
        self.run_sample_generation()?;
        if !dir_has_entries(&sample_dir)? {
            return Err(HydrogenError::MissingGeneratedSampleFiles {
                sample_dir,
                missing_files: vec!["no generated files found".to_string()],
            });
        }

        Ok(())
    }

    fn ensure_internal_impulse_reference_wav(&self) -> Result<(), HydrogenError> {
        let generator_dir = self.script_dir()?.join(GENERATOR_SUBDIR);
        let reference_wav = generator_dir.join(INTERNAL_IMPULSE_REFERENCE_WAV);
        if reference_wav.is_file() {
            return Ok(());
        }

        let reference_zip = generator_dir.join(INTERNAL_IMPULSE_REFERENCE_ZIP);
        if !reference_zip.is_file() {
            return Err(HydrogenError::InvalidScriptLocation(reference_zip));
        }

        let zip_file = File::open(&reference_zip)?;
        let mut archive = zip::ZipArchive::new(zip_file)?;
        archive.extract(&generator_dir)?;

        if !reference_wav.is_file() {
            return Err(HydrogenError::InvalidScriptLocation(reference_wav));
        }

        Ok(())
    }

    fn run_sample_generation(&self) -> Result<(), HydrogenError> {
        let sample_dir = self.sample_dir()?;

        let scripts_root = self.script_dir()?;
        let generator_dir = scripts_root.join(GENERATOR_SUBDIR);

        for script_path in self.generation_script_paths()? {
            let output = Command::new("octave")
                .args(["--silent", "--no-gui", "--quiet"])
                .current_dir(&generator_dir)
                .arg(&script_path)
                .arg(&sample_dir)
                .output()?;

            if !output.status.success() {
                let script_name = script_path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("unknown script");
                return Err(HydrogenError::OctaveCommandFailed {
                    context: format!("octave sample generation ({script_name})"),
                    stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                });
            }
        }

        Ok(())
    }

    fn run_analysis(&self) -> Result<(), HydrogenError> {
        let output_dir = self.output_dir()?;
        let analysis_output_dir = self.analysis_output_dir()?;

        for analysis_script in self.analysis_script_paths()? {
            let output = Command::new("octave")
                .args(["--silent", "--no-gui", "--quiet"])
                .current_dir(self.script_dir()?)
                .arg(&analysis_script)
                .arg(&output_dir)
                .arg(&analysis_output_dir)
                .output()?;

            if !output.status.success() {
                let script_name = analysis_script
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("unknown script");
                return Err(HydrogenError::OctaveCommandFailed {
                    context: format!("octave analysis run ({script_name})"),
                    stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                });
            }
        }

        Ok(())
    }

    fn copy_outputs_without_suffixes(&self) -> Result<(), HydrogenError> {
        const SUFFIXES: [&str; 2] = ["-64bitfloat", "-32bitfloat"];

        let output_dir = self.output_dir()?;
        for input_file in list_wavs(&output_dir)? {
            let file_stem = input_file
                .file_stem()
                .and_then(|stem| stem.to_str())
                .ok_or_else(|| HydrogenError::MissingFileName(input_file.clone()))?;

            let suffix = SUFFIXES
                .iter()
                .find(|suffix| file_stem.ends_with(*suffix))
                .copied();

            let Some(suffix) = suffix else {
                continue;
            };

            let normalized_stem = &file_stem[..file_stem.len() - suffix.len()];
            let normalized_file = output_dir.join(format!("{normalized_stem}.wav"));
            fs::copy(&input_file, normalized_file)?;
        }

        Ok(())
    }

    fn copy_references(&self) -> Result<(), HydrogenError> {
        // Copy wav references
        let sample_dir = self.sample_dir()?;
        let output_dir = self.output_dir()?;
        for reference_wav in list_wavs(&sample_dir)? {
            let file_name = file_name_from_path(&reference_wav)?;
            fs::copy(&reference_wav, output_dir.join(file_name))?;
        }

        // Copy reference spectrogram PNG
        let reference_png = self.script_dir()?.join(REFERENCE_SPECTROGRAM_PNG);
        if !reference_png.is_file() {
            return Err(HydrogenError::InvalidScriptLocation(reference_png));
        }
        fs::copy(&reference_png, output_dir.join(REFERENCE_SPECTROGRAM_PNG))?;

        Ok(())
    }

    fn prepare_output_dir(&self) -> Result<(), HydrogenError> {
        let output_dir = self.output_dir()?;
        if output_dir.exists() {
            fs::remove_dir_all(&output_dir)?;
        }
        fs::create_dir_all(output_dir)?;
        Ok(())
    }

    fn prepare_analysis_output_dir(&self) -> Result<(), HydrogenError> {
        let analysis_output_dir = self.analysis_output_dir()?;
        if analysis_output_dir.exists() {
            fs::remove_dir_all(&analysis_output_dir)?;
        }
        fs::create_dir_all(analysis_output_dir)?;
        Ok(())
    }

    fn workspace_dir(&self) -> Result<PathBuf, HydrogenError> {
        if self.workspace.is_absolute() {
            Ok(self.workspace.clone())
        } else {
            Ok(std::env::current_dir()?.join(&self.workspace))
        }
    }

    fn script_dir(&self) -> Result<PathBuf, HydrogenError> {
        let script_dir = if self.script_dir.is_absolute() {
            self.script_dir.clone()
        } else {
            std::env::current_dir()?.join(&self.script_dir)
        };

        if !script_dir.is_dir() {
            return Err(HydrogenError::InvalidScriptLocation(script_dir));
        }

        Ok(script_dir)
    }

    fn sample_dir(&self) -> Result<PathBuf, HydrogenError> {
        Ok(self.workspace_dir()?.join(DEFAULT_SAMPLE_SUBDIR))
    }

    fn sample_input_dir(&self, float_variant: FloatVariant) -> Result<PathBuf, HydrogenError> {
        let subdir = match float_variant {
            FloatVariant::F32 => "32bitfloat",
            FloatVariant::F64 => "64bitfloat",
        };
        Ok(self.sample_dir()?.join(subdir))
    }

    fn output_dir(&self) -> Result<PathBuf, HydrogenError> {
        Ok(self.workspace_dir()?.join(DEFAULT_OUTPUT_SUBDIR))
    }

    fn analysis_output_dir(&self) -> Result<PathBuf, HydrogenError> {
        Ok(self.workspace_dir()?.join(DEFAULT_ANALYSIS_OUTPUT_SUBDIR))
    }

    fn analysis_script_paths(&self) -> Result<Vec<PathBuf>, HydrogenError> {
        let script_dir = self.script_dir()?;
        let mut script_paths = Vec::with_capacity(ANALYSIS_SCRIPTS.len());
        for script_name in ANALYSIS_SCRIPTS {
            let script_path = script_dir.join(script_name);
            if !script_path.is_file() {
                return Err(HydrogenError::InvalidScriptLocation(script_path));
            }
            script_paths.push(script_path);
        }
        Ok(script_paths)
    }

    fn generation_script_paths(&self) -> Result<Vec<PathBuf>, HydrogenError> {
        let scripts_root = self.script_dir()?;
        let generator_dir = scripts_root.join(GENERATOR_SUBDIR);
        if !generator_dir.is_dir() {
            return Err(HydrogenError::InvalidScriptLocation(generator_dir));
        }

        let mut script_paths = Vec::with_capacity(GENERATION_SCRIPTS.len());
        for script_name in GENERATION_SCRIPTS {
            let script_path = generator_dir.join(script_name);
            if !script_path.is_file() {
                return Err(HydrogenError::InvalidScriptLocation(script_path));
            }
            script_paths.push(script_path);
        }
        Ok(script_paths)
    }
}

fn file_name_from_path(path: &Path) -> Result<PathBuf, HydrogenError> {
    path.file_name()
        .map(PathBuf::from)
        .ok_or_else(|| HydrogenError::MissingFileName(path.to_path_buf()))
}

fn dir_has_entries(path: &Path) -> Result<bool, HydrogenError> {
    let mut entries = fs::read_dir(path)?;
    Ok(entries.next().transpose()?.is_some())
}

fn has_extension(path: &Path, extension: &str) -> bool {
    path.extension()
        .and_then(OsStr::to_str)
        .is_some_and(|ext| ext.eq_ignore_ascii_case(extension))
}

fn parse_file(path: &Path, line_num: usize) -> Result<f64, HydrogenError> {
    let contents = fs::read_to_string(path)?;
    let truncated = contents.trim_end();
    let line = truncated
        .lines()
        .nth(line_num)
        .map(str::trim)
        .ok_or_else(|| HydrogenError::EmptyQualityMetric {
            path: path.to_path_buf(),
        })?;

    line.parse::<f64>()
        .map_err(|_| HydrogenError::InvalidQualityMetric {
            path: path.to_path_buf(),
            value: line.to_string(),
        })
}

fn absolutize_path(path: PathBuf) -> Result<PathBuf, HydrogenError> {
    if path.is_absolute() {
        Ok(path)
    } else {
        Ok(std::env::current_dir()?.join(path))
    }
}

fn average_quality_score(quality_scores: &[f64]) -> f64 {
    if quality_scores.is_empty() {
        return 0.0;
    }

    let sum: f64 = quality_scores.iter().copied().sum();
    sum / (quality_scores.len() as f64)
}

fn balanced_quality_score(
    spectrogram_score: f64,
    bandwidth_score: f64,
    impulse_freq_score: f64,
    alias_score: f64,
    preringing_score: f64,
    gapless_score: f64,
    intermoddiff_score: f64,
    delay_score: f64,
) -> f64 {
    const SPECTROGRAM_WEIGHT: f64 = 100.0;
    const BANDWIDTH_WEIGHT: f64 = 100.0;
    const IMPULSE_FREQUENCY_WEIGHT: f64 = 60.0;
    const ALIASING_WEIGHT: f64 = 75.0;
    const PRE_RINGING_WEIGHT: f64 = 0.0;
    const DELAY_WEIGHT: f64 = 20.0;
    const GAPLESS_WEIGHT: f64 = 20.0;
    const INTERMODULATION_WEIGHT: f64 = 60.0;

    let weighted_sum = spectrogram_score * SPECTROGRAM_WEIGHT
        + bandwidth_score * BANDWIDTH_WEIGHT
        + impulse_freq_score * IMPULSE_FREQUENCY_WEIGHT
        + alias_score * ALIASING_WEIGHT
        + preringing_score * PRE_RINGING_WEIGHT
        + delay_score * DELAY_WEIGHT
        + gapless_score * GAPLESS_WEIGHT
        + intermoddiff_score * INTERMODULATION_WEIGHT;

    let total_weight = SPECTROGRAM_WEIGHT
        + BANDWIDTH_WEIGHT
        + IMPULSE_FREQUENCY_WEIGHT
        + ALIASING_WEIGHT
        + PRE_RINGING_WEIGHT
        + DELAY_WEIGHT
        + GAPLESS_WEIGHT
        + INTERMODULATION_WEIGHT;

    weighted_sum / total_weight
}

fn nan_f64() -> f64 {
    f64::NAN
}

fn delay_score_from_normalized(value: f64) -> f64 {
    let clamped = value.clamp(0.0, 1.0);
    ((1.0 - clamped) * 100.0).max(0.0)
}

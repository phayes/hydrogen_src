use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};

use wavers::{Wav, write};

use crate::{FloatVariant, HydrogenError, ResampleRequestF32, ResampleRequestF64, ResamplerCallbackF32, ResamplerCallbackF64, list_wavs};

const TARGET_OUTPUT_SAMPLE_RATE: usize = 44_100;
const DEFAULT_SAMPLE_SUBDIR: &str = "local_test_generated_samples";
const DEFAULT_OUTPUT_SUBDIR: &str = "local_test_output";
const DEFAULT_ANALYSIS_OUTPUT_SUBDIR: &str = "analysis_output";
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

    pub fn run(&mut self) -> Result<(), HydrogenError> {
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
            return Ok(());
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
            return Ok(());
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
        for input_file in list_wavs(&self.sample_input_dir(FloatVariant::F64)?)? {
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
        fs::copy(
            &reference_png,
            output_dir.join(REFERENCE_SPECTROGRAM_PNG),
        )?;

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

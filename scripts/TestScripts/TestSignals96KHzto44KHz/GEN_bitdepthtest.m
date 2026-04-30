if ~exist('output_root', 'var')
  args = argv();
  if numel(args) >= 1
    output_root = args{1};
  else
    output_root = '.';
  endif
endif

pkg load signal

dir24i = fullfile(output_root, '24bitint');
dir32i = fullfile(output_root, '32bitint');
dir32f = fullfile(output_root, '32bitfloat');
dir64 = fullfile(output_root, '64bitfloat');
dirBitdepth = fullfile(output_root, 'bitdepthtest');

mkdir(dir24i);
mkdir(dir32i);
mkdir(dir32f);
mkdir(dir64);
mkdir(dirBitdepth);

Fs = 96000;
duration = 4;
t = 0:1/Fs:duration;


% Normalize the noise to a range of -1 to 1
noise = randn(1, duration * Fs);
noise = noise / max(abs(noise));
scaling_factor_16bit = 10^(-7/20);
y_16bit = noise * scaling_factor_16bit;


noise = randn(1, duration * Fs);
noise = noise / max(abs(noise));
scaling_factor_24bit = 10^(-135/20);
y_24bit = noise * scaling_factor_24bit;


noise = randn(1, duration * Fs);
noise = noise / max(abs(noise));
scaling_factor_32bit = 10^(-180/20);
y_32bit = noise * scaling_factor_32bit;

noise = randn(1, duration * Fs);
noise = noise / max(abs(noise));
scaling_factor_64bit = 10^(-225/20);
y_64bit = noise * scaling_factor_64bit;

y32bitint = int32(y_32bit * 2147483647);


audiowrite(fullfile(dir24i, 'bitdepth_16bitint.wav'), y_16bit, Fs, 'BitsPerSample', 16);
audiowrite(fullfile(dir24i, 'bitdepth_24bitint.wav'), y_24bit, Fs, 'BitsPerSample', 24);


audiowrite(fullfile(dir32i, 'bitdepth_16bitint.wav'), y_16bit, Fs, 'BitsPerSample', 16);
audiowrite(fullfile(dir32i, 'bitdepth_24bitint.wav'), y_24bit, Fs, 'BitsPerSample', 24);
audiowrite(fullfile(dir32i, 'bitdepth_32bitint.wav'), y32bitint, Fs, 'BitsPerSample', 32);
audiowrite(fullfile(dir32i, 'bitdepth_32bitfloat.wav'), y_32bit, Fs, 'BitsPerSample', 32);


audiowrite(fullfile(dir32f, 'bitdepth_16bitint.wav'), y_16bit, Fs, 'BitsPerSample', 16);
audiowrite(fullfile(dir32f, 'bitdepth_24bitint.wav'), y_24bit, Fs, 'BitsPerSample', 24);
audiowrite(fullfile(dir32f, 'bitdepth_32bitint.wav'), y32bitint, Fs, 'BitsPerSample', 32);
audiowrite(fullfile(dir32f, 'bitdepth_32bitfloat.wav'), y_32bit, Fs, 'BitsPerSample', 32);


audiowrite(fullfile(dir64, 'bitdepth_16bitint.wav'), y_16bit, Fs, 'BitsPerSample', 16);
audiowrite(fullfile(dir64, 'bitdepth_24bitint.wav'), y_24bit, Fs, 'BitsPerSample', 24);
audiowrite(fullfile(dir64, 'bitdepth_32bitint.wav'), y32bitint, Fs, 'BitsPerSample', 32);
audiowrite(fullfile(dir64, 'bitdepth_32bitfloat.wav'), y_32bit, Fs, 'BitsPerSample', 32);
audiowrite(fullfile(dir64, 'bitdepth_64bitfloat.wav'), y_64bit, Fs, 'BitsPerSample', 64);

% generate a 1 KHz full sine wave
f_1 = 10;            % Sine wave frequency (Hz)
Fs_1 = 96000;         % Sampling frequency (samples per second)
duration_1 = 1;       % Duration of the sound (seconds)
amplitude_1 = 0.9;    % Amplitude (between 0 and 1)

t_1 = 0:1/Fs_1:duration_1; % Time vector
t_1(end) = [];%remove one from end

y_1 = amplitude_1 * sin(2 * pi * f_1 * t_1); % Sine wave signal

y32bitint_1 = int32(y_1 * 2147483647);
audiowrite(fullfile(dirBitdepth, 'bitdepthtest_16bitint.wav'), y_1, Fs_1, 'BitsPerSample', 16);
audiowrite(fullfile(dirBitdepth, 'bitdepthtest_24bitint.wav'), y_1, Fs_1, 'BitsPerSample', 24);
audiowrite(fullfile(dirBitdepth, 'bitdepthtest_32bitint.wav'), y32bitint_1, Fs_1, 'BitsPerSample', 32);
audiowrite(fullfile(dirBitdepth, 'bitdepthtest_32bitfloat.wav'), y_1, Fs_1, 'BitsPerSample', 32);
audiowrite(fullfile(dirBitdepth, 'bitdepthtest_64bitfloat.wav'), y_1, Fs_1, 'BitsPerSample', 64);

if ~exist('output_root', 'var')
  args = argv();
  if numel(args) >= 1
    output_root = args{1};
  else
    output_root = '.';
  endif
endif

pkg load signal

dir64 = fullfile(output_root, '64bitfloat');
dir32f = fullfile(output_root, '32bitfloat');
dir32i = fullfile(output_root, '32bitint');
dir24i = fullfile(output_root, '24bitint');
dir44ideal = fullfile(output_root, '44kHzIdealResponse');

mkdir(dir64);
mkdir(dir32f);
mkdir(dir32i);
mkdir(dir24i);
mkdir(dir44ideal);

% Parameters
duration = 10; % seconds
Fs = 96000; % Sampling frequency (Hz)

% Generate 10 seconds of white noise
noise = randn(1, duration * Fs);

% Normalize the noise to a range of -1 to 1
noise = noise / max(abs(noise));

% Apply a dB reduction (scale by a very small number)
% dB = 20 * log10(amplitude) => amplitude = 10^(dB/20)
scaling_factor = 10^(-85/20);
reduced_noise = scaling_factor * noise;

% Ensure the data type is suitable for audio (e.g., double)
%reduced_noise = double(reduced_noise);

f = 23000;            % Sine wave frequency (Hz)
amplitudedB = -4;   % in dB
amplitude = 10^(amplitudedB/20);    % Amplitude 

t = 0:1/Fs:duration; % Time vector
t(end) = [];%remove one from end
sinedata = amplitude * sin(2 * pi * f * t); % Sine wave signal

y = sinedata + reduced_noise;

y32bitint = int32(y * 2147483647);
audiowrite(fullfile(dir64, 'aliasing150db-64bitfloat.wav'), y, Fs, 'BitsPerSample', 64);
audiowrite(fullfile(dir32f, 'aliasing150db-32bitfloat.wav'), y, Fs, 'BitsPerSample', 32);
audiowrite(fullfile(dir32i, 'aliasing150db-32bitint.wav'), y32bitint, Fs, 'BitsPerSample', 32);
audiowrite(fullfile(dir24i, 'aliasing150db-24bitint.wav'), y, Fs, 'BitsPerSample', 24);

% and the ideal plot
Fs_ideal = 44100; % Sampling frequency (Hz)

% Generate 10 seconds of white noise
noise_ideal = randn(1, duration * Fs_ideal);

% Normalize the noise to a range of -1 to 1
noise_ideal = noise_ideal / max(abs(noise_ideal));

% Apply a dB reduction (scale by a very small number)
% dB = 20 * log10(amplitude) => amplitude = 10^(dB/20)
scaling_factor_ideal = 10^(-89/20);
reduced_noise_ideal = scaling_factor_ideal * noise_ideal;

% Ensure the data type is suitable for audio (e.g., double)
%reduced_noise = double(reduced_noise);

%t_ideal = 0:1/Fs_ideal:duration; % Time vector
%t_ideal(end) = [];%remove one from end
%sinedata_ideal = amplitude * sin(2 * pi * f * t_ideal); % Sine wave signal

y_ideal = reduced_noise_ideal;
audiowrite(fullfile(dir44ideal, 'aliasing150db-64bitfloat.wav'), y_ideal, Fs_ideal, 'BitsPerSample', 64);

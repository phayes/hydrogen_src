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

Fs = 96000;         % Sampling frequency (samples per second)
duration = 10;       % Duration of the sound (seconds)

t = 0:1/Fs:duration; % Time vector
t(end) = [];%remove one from end

f_S1 = 64.599609375;           % Sine wave frequency (Hz)
amplitudedB_S1 = -6;   % in dB
amplitude_S1 = 10^(amplitudedB_S1/20);    % Amplitude 

f_S2 = 6998.291015625;            % Sine wave frequency (Hz)
amplitudedB_S2 = -18.0412;   % in dB
amplitude_S2 = 10^(amplitudedB_S2/20);    % Amplitude 

y_S1 = amplitude_S1 * sin(2 * pi * f_S1 * t); % Sine wave signal
y_S2 = amplitude_S2 * sin(2 * pi * f_S2 * t); % Sine wave signal
y = y_S1 + y_S2;

y32bitint = int32(y * 2147483647);
audiowrite(fullfile(dir64, 'intermodulation_sine-64bitfloat.wav'), y, Fs, 'BitsPerSample', 64);
audiowrite(fullfile(dir32f, 'intermodulation_sine-32bitfloat.wav'), y, Fs, 'BitsPerSample', 32);
audiowrite(fullfile(dir32i, 'intermodulation_sine-32bitint.wav'), y32bitint, Fs, 'BitsPerSample', 32);
audiowrite(fullfile(dir24i, 'intermodulation_sine-24bitint.wav'), y, Fs, 'BitsPerSample', 24);

% create at 44KHz
Fs44 = 44100;         % Sampling frequency (samples per second)
t44 = 0:1/Fs44:duration; % Time vector
t44(end) = [];%remove one from end

y_S144 = amplitude_S1 * sin(2 * pi * f_S1 * t44); % Sine wave signal
y_S244 = amplitude_S2 * sin(2 * pi * f_S2 * t44); % Sine wave signal
y44 = y_S144 + y_S244;
audiowrite(fullfile(output_root, 'intermodulation_sine-InternalUseAt44kHz.wav'), y44, Fs44, 'BitsPerSample', 64);
audiowrite(fullfile(dir44ideal, 'intermodulation_sine-64bitfloat.wav'), y44, Fs44, 'BitsPerSample', 64);



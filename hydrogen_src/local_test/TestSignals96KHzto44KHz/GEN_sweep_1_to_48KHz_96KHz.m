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

freq1 = 1;  % start frequency
freq2 = 44000; % end frequency (so can use subtraction)
fs = 96000;
dur = 20;     % duration of signal in seconds

t = 0:1/fs:dur;
freqt = linspace(freq1,freq2,numel(t));
ifreqt = cumsum(freqt)/fs;
y = sin(2*pi*ifreqt);

y32bitint = int32(y * 2147483647);
audiowrite(fullfile(dir64, 'sweep_1_to_44KHz-64bitfloat.wav'), y, fs, 'BitsPerSample', 64);
audiowrite(fullfile(dir32f, 'sweep_1_to_44KHz-32bitfloat.wav'), y, fs, 'BitsPerSample', 32);
audiowrite(fullfile(dir32i, 'sweep_1_to_44KHz-32bitint.wav'), y32bitint, fs, 'BitsPerSample', 32);
audiowrite(fullfile(dir24i, 'sweep_1_to_44KHz-24bitint.wav'), y, fs, 'BitsPerSample', 24);

% ideal as though resampled at 44.1kHz
fs_ideal = 44100;
freq1_ideal = freq1;
freq2_ideal = fs_ideal/2;
dur_ideal = dur * freq2_ideal / freq2;     % duration of signal in seconds
t_ideal = 0:1/fs_ideal:dur_ideal;
freqt_ideal = linspace(freq1_ideal,freq2_ideal,numel(t_ideal));
ifreqt_ideal = cumsum(freqt_ideal)/fs_ideal;
y_ideal = sin(2*pi*ifreqt_ideal);

% note fade was to remove HF line on spectrogram, this is less important
% now as manually edit reference spectrogram image

%fade 0.1 seconds at end
%fadeDuration = 0.08; % seconds
%fadeSamples = round(fadeDuration * fs_ideal);
%fadeOutEnvelope = cos(pi * (0:fadeSamples-1) / (2 * fadeSamples))';
%lenFade = length(fadeOutEnvelope);
%y_end_portion = y_ideal(end - lenFade + 1 : end);
%result_portion = y_end_portion .* fadeOutEnvelope';
%result_portion = result_portion .* fadeOutEnvelope';
%y_ideal(end - lenFade + 1 : end) = result_portion;

fadeDuration = 0.15; % seconds
fade_samples = round(fs_ideal * fadeDuration);
total_samples = length(y_ideal);
fade_start_index = total_samples - fade_samples + 1;
fade_end_index = total_samples;
%fade_window = hann(2 * fade_samples, 'periodic');
%fade_window = fade_window(fade_samples + 1 : end);
fade_window = hann(2 * fade_samples, 'periodic');
fade_window = flipud(fade_window(1:fade_samples)); % Use first half of hann and flip for 1 to 0
%t_fade = linspace(0, 1, fade_samples)';
%fade_window = 0.5 * (1 + cos(pi * t_fade));
y_ideal(fade_start_index:fade_end_index) = y_ideal(fade_start_index:fade_end_index) .* fade_window';


t_idealZEROs = 0:1/fs_ideal:dur_ideal; % Time vector
t_idealZEROs(end) = [];%remove one from end
y_idealZEROs = 0 * sin(2 * pi * 1000 * t_idealZEROs); % Sine wave signal

y_idealFinal = [y_ideal,y_idealZEROs];
audiowrite(fullfile(dir44ideal, 'sweep_1_to_44KHz-64bitfloat.wav'), y_idealFinal, fs_ideal, 'BitsPerSample', 64);

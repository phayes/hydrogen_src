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

mkdir(dir64);
mkdir(dir32f);
mkdir(dir32i);
mkdir(dir24i);

Fs = 96000;         % Sampling frequency (samples per second)
duration = 0.2;       % Duration of the sound (seconds)

t = 0:1/Fs:duration; % Time vector
t(end) = [];%remove one from end

f_S1 = 239;            % Sine wave frequency (Hz)
amplitudedB_S1 = -5;   % in dB
amplitude_S1 = 10^(amplitudedB_S1/20);    % Amplitude 

f_S2 = 1000;            % Sine wave frequency (Hz)
amplitudedB_S2 = -7.5;   % in dB
amplitude_S2 = 10^(amplitudedB_S2/20);    % Amplitude 

y_S1 = amplitude_S1 * sin(2 * pi * f_S1 * t); % Sine wave signal
y_S2 = amplitude_S2 * sin(2 * pi * f_S2 * t); % Sine wave signal

y = y_S1 + y_S2;

firstcut=3288;                      %8562;
lastcut=firstcut + 2409;            %12347;
section1 = y(1:firstcut-1);
section2 = y(firstcut:lastcut-1);

% create same as 44KHz
FsAtResampleRate = 44100;
firstcutAtResampleRate=floor((firstcut * FsAtResampleRate) / Fs);
lastcutAtResampleRate=floor((lastcut * FsAtResampleRate) / Fs);

tAtResampleRate = 0:1/FsAtResampleRate:duration; % Time vector
y_S1AtResampleRate = amplitude_S1 * sin(2 * pi * f_S1 * tAtResampleRate); % Sine wave signal
y_S2AtResampleRate = amplitude_S2 * sin(2 * pi * f_S2 * tAtResampleRate); % Sine wave signal
yAtResampleRate = y_S1AtResampleRate + y_S2AtResampleRate;
section1AtResampleRate = yAtResampleRate(1:firstcutAtResampleRate-1);
section2AtResampleRate = yAtResampleRate(firstcutAtResampleRate:lastcutAtResampleRate-1);
completeAtResampleRate = yAtResampleRate(1:lastcutAtResampleRate);


section132bitint = int32(section1 * 2147483647);
audiowrite(fullfile(dir64, 'gaplesstest_s-64bitfloat.wav'), section1, Fs, 'BitsPerSample', 64);
audiowrite(fullfile(dir32f, 'gaplesstest_s-32bitfloat.wav'), section1, Fs, 'BitsPerSample', 32);
audiowrite(fullfile(dir32i, 'gaplesstest_s-32bitint.wav'), section132bitint, Fs, 'BitsPerSample', 32);
audiowrite(fullfile(dir24i, 'gaplesstest_s-24bitint.wav'), section1, Fs, 'BitsPerSample', 24);

section232bitint = int32(section2 * 2147483647);
audiowrite(fullfile(dir64, 'gaplesstest_m-64bitfloat.wav'), section2, Fs, 'BitsPerSample', 64);
audiowrite(fullfile(dir32f, 'gaplesstest_m-32bitfloat.wav'), section2, Fs, 'BitsPerSample', 32);
audiowrite(fullfile(dir32i, 'gaplesstest_m-32bitint.wav'), section232bitint, Fs, 'BitsPerSample', 32);
audiowrite(fullfile(dir24i, 'gaplesstest_m-24bitint.wav'), section2, Fs, 'BitsPerSample', 24);

audiowrite(fullfile(output_root, 'gaplesstest_c-64bitfloat-InternalUseAt44kHz.wav'), completeAtResampleRate, FsAtResampleRate, 'BitsPerSample', 64);
%audiowrite('gaplesstest_c-64bitfloat-InternalUse-s.wav', section1AtResampleRate, FsAtResampleRate, 'BitsPerSample', 64);
%audiowrite('gaplesstest_c-64bitfloat-InternalUse-m.wav', section2AtResampleRate, FsAtResampleRate, 'BitsPerSample', 64);



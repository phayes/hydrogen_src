pkg load signal

f = 1000;            % Sine wave frequency (Hz)
Fs = 96000;         % Sampling frequency (samples per second)
duration = 4;       % Duration of the sound (seconds)
amplitude = 0.9;    % Amplitude (between 0 and 1)

t = 0:1/Fs:duration; % Time vector
t(end) = [];%remove one from end

y = amplitude * sin(2 * pi * f * t); % Sine wave signal

y32bitint = int32(y * 2147483647);
audiowrite('64bitfloat/sine1khz-64bitfloat.wav', y, Fs, 'BitsPerSample', 64);
audiowrite('32bitfloat/sine1khz-32bitfloat.wav', y, Fs, 'BitsPerSample', 32);
audiowrite('32bitint/sine1khz-32bitint.wav', y32bitint, Fs, 'BitsPerSample', 32);
audiowrite('24bitint/sine1khz-24bitint.wav', y, Fs, 'BitsPerSample', 24);

% and the ideal wave
f_ideal = 1000;            % Sine wave frequency (Hz)
Fs_ideal = 44100;         % Sampling frequency (samples per second)
duration_ideal = 4;       % Duration of the sound (seconds)
amplitude_ideal = 0.9;    % Amplitude (between 0 and 1)

t_ideal = 0:1/Fs_ideal:duration_ideal; % Time vector
t_ideal(end) = [];%remove one from end

y_ideal = amplitude_ideal * sin(2 * pi * f_ideal * t_ideal); % Sine wave signal
audiowrite('44kHzIdealResponse/sine1khz-64bitfloat.wav', y_ideal, Fs_ideal, 'BitsPerSample', 64);

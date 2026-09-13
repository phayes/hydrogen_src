if ~exist('input_dir', 'var') || isempty(input_dir)
  args = argv();
  if numel(args) >= 1
    input_dir = args{1};
  else
    input_dir = '.';
  endif
endif
if ~exist('output_dir', 'var') || isempty(output_dir)
  if exist('args', 'var') && numel(args) >= 2
    output_dir = args{2};
  else
    output_dir = '.';
  endif
endif
mkdir(output_dir);

pkg load signal
pkg load image

% Create a figure with visibility turned off.
% The handle 'h' is used for the print command.
h = figure('Visible', 'off');

samplesshowaroundbreak = 25; % zoomed display, 25 before break and 25fs after

FirstSR = 96000;
FinalSR = 44100;

firstcut = 3288;
lastcut=firstcut + 2409;

cutAtFinalSR=floor((firstcut * FinalSR) / FirstSR) + 1;
LastcutAtFinalSR=floor((lastcut * FinalSR) / FirstSR);

% generate full wave from 2
[firsthalf, Fs] = audioread(fullfile(input_dir, 'gaplesstest_s.wav'));
[secondhalf, Fs] = audioread(fullfile(input_dir, 'gaplesstest_m.wav'));
firstandsecondFULL = [firsthalf; secondhalf];

while (length(firstandsecondFULL) < LastcutAtFinalSR)   % ensure always right length
  firstandsecondFULL(end + 1) = 0;
end

while (length(firstandsecondFULL) > LastcutAtFinalSR)   % ensure always right length
  firstandsecondFULL(end) = [];
end

% gen short version from full
firstandsecondSHORT = firstandsecondFULL(cutAtFinalSR - samplesshowaroundbreak: cutAtFinalSR + samplesshowaroundbreak - 1);

% now cut in two for display in different cols
firstandsecondSHORTBeforeCut = firstandsecondSHORT(1:samplesshowaroundbreak);
t_firstandsecondSHORTBeforeCut = ([0:length(firstandsecondSHORTBeforeCut) - 1]) / Fs;

firstandsecondSHORTAfterCut = firstandsecondSHORT(samplesshowaroundbreak+1:end);
t_firstandsecondSHORTAfterCut = ([length(firstandsecondSHORTBeforeCut):length(firstandsecondSHORTBeforeCut)+length(firstandsecondSHORTAfterCut)-1]) / Fs;

% create display chunk from full wave
firstandsecond = firstandsecondFULL(cutAtFinalSR - samplesshowaroundbreak: cutAtFinalSR + samplesshowaroundbreak);
t_firstandsecond = ([0:length(firstandsecond) - 1]) / Fs;

% create example
[exampleFULL, Fs] = audioread(fullfile(input_dir, 'gaplesstest_c-64bitfloat-InternalUseAt44kHz.wav'));

while (length(exampleFULL) < LastcutAtFinalSR)   % ensure always right length
  exampleFULL(end + 1) = 0;
end

while (length(exampleFULL) > LastcutAtFinalSR)   % ensure always right length
  exampleFULL(end) = [];
end

exampleSHORT = exampleFULL(cutAtFinalSR - samplesshowaroundbreak: cutAtFinalSR + samplesshowaroundbreak);
t_exampleSHORT = ([0:length(exampleSHORT) - 1]) / Fs;




% do FFT up sample then iFFT and grab data
upsample_factor = 10;

n_firstandsecondFULL = length(firstandsecondFULL);
XFFT_firstandsecondFULL = fft(firstandsecondFULL);
firstandsecondFULL_upsampledFFT = zeros(1, n_firstandsecondFULL * upsample_factor);
mid_point_firstandsecondFULL = ceil(n_firstandsecondFULL/2);
firstandsecondFULL_upsampledFFT(1:mid_point_firstandsecondFULL) = XFFT_firstandsecondFULL(1:mid_point_firstandsecondFULL);
firstandsecondFULL_upsampledFFT(end-mid_point_firstandsecondFULL+1:end) = XFFT_firstandsecondFULL(mid_point_firstandsecondFULL:end);
firstandsecondFULL_upsampledasRealNImag = ifft(firstandsecondFULL_upsampledFFT);
firstandsecondFULL_upsampledasRealNImag = firstandsecondFULL_upsampledasRealNImag * upsample_factor;
firstandsecondFULL_upsampled = real(firstandsecondFULL_upsampledasRealNImag);
t_firstandsecondFULL_upsampled = ([1:length(firstandsecondFULL_upsampled)]) / (Fs * upsample_factor);

firstandsecond_upsampledSHORT = firstandsecondFULL_upsampled((cutAtFinalSR - samplesshowaroundbreak - 1) * upsample_factor : (cutAtFinalSR + samplesshowaroundbreak - 1) * upsample_factor);
t_firstandsecond_upsampledSHORT = ([0:length(firstandsecond_upsampledSHORT) - 1]) / (Fs * upsample_factor);


leftplotcol = [254/255 227/255 0/255];
rightplotcol = [227/255 0/255 254/255];
playbackplotcol = [0/255 145/255 206/255];

% Plot the waveform
%figure;

exampleplotcol = [128/255 128/255 128/255];
plot(t_exampleSHORT, exampleSHORT, 'LineWidth', 2, 'Color', exampleplotcol);
ylim([-1, 1])

hold on;

plot(t_firstandsecond_upsampledSHORT, firstandsecond_upsampledSHORT, 'LineWidth', 1.5, 'Color', playbackplotcol);

stem(t_firstandsecondSHORTBeforeCut, firstandsecondSHORTBeforeCut,'o', 'MarkerSize', 6, 'MarkerFaceColor', leftplotcol, 'MarkerEdgeColor', leftplotcol, 'LineWidth', 1.25, 'Color', leftplotcol);
stem(t_firstandsecondSHORTAfterCut, firstandsecondSHORTAfterCut, 'o', 'MarkerSize', 6, 'MarkerFaceColor', rightplotcol, 'MarkerEdgeColor', rightplotcol, 'LineWidth', 1.25, 'Color', rightplotcol);

text(0.04, 0.98, 'Ideal Response', 'Color', exampleplotcol, 'FontSize', 8, 'Units', 'normalized');
text(0.04, 0.95, 'Pre-cut processed signal', 'Color', leftplotcol, 'FontSize', 8, 'Units', 'normalized');
text(0.04, 0.92, 'Post-cut processed signal', 'Color', rightplotcol, 'FontSize', 8, 'Units', 'normalized');
text(0.04, 0.89, 'Resampled Signal', 'Color', playbackplotcol, 'FontSize', 8, 'Units', 'normalized');

hold off;

% Add labels and title
xlabel('Time (seconds)');
ylabel('Amplitude');

set(gca,'Color','k')
set(gcf,'InvertHardCopy','Off');
%set(gca, 'TickDir', 'out', 'YMinorTick', 'off', 'Box', 'on', 'XGrid', 'on', 'YGrid', 'on')
set(gca, 'TickDir', 'out', 'XTick', 0:(samplesshowaroundbreak * 2) / Fs / 4:(samplesshowaroundbreak * 2) / Fs, 'YTick', -1:0.25:1, 'XMinorTick', 'on', 'YMinorTick', 'off', 'Box', 'off', 'XGrid', 'on', 'YGrid', 'on')
set(gca, 'FontSize', 8)

% Get the current axes handle
ax = gca;
set(ax, "gridcolor", [0.5 0.5 0.5]);
set(ax, "gridlinestyle", "--"); % Dashed lines
set(ax, "gridalpha", 0.7);     % 70% transparency

set(h, 'paperunits', 'centimeters');
set(h, 'papersize', [20 10]);
set(h, 'paperposition', [0 0 20 10]);
print(h, '-dpng', fullfile(output_dir, 'gaplesstestsine.png'), '-r200');


%-----------------------------------------------------------------------------------------------------------------------------

% Close the figure after saving.
%-----------------------------------------------------------------------------------------------------------------------------
%freqency
%-----------------------------------------------------------------------------------------------------------------------------


N = 2048;
YdBMin = -90;

FFTfirstandsecondRealNImag = fft(firstandsecondFULL, N);
P2 = abs(FFTfirstandsecondRealNImag/N);
P1 = P2(1:N/2+1);      % Take the first half of the spectrum
P1(2:end-1) = 2*P1(2:end-1); % Double the amplitudes (except for DC and Nyquist)
FFTfirstandsecond_db = 20*log10(P1);  % Convert to dB
f_FFFfirstandsecond = Fs*(0:(N/2))/N; % Frequency vector for the plot

FFTexampleFULL = fft(exampleFULL, N);
P2exampleFULL = abs(FFTexampleFULL/N);
PexampleFULL = P2exampleFULL(1:N/2+1);      % Take the first half of the spectrum
PexampleFULL(2:end-1) = 2*PexampleFULL(2:end-1); % Double the amplitudes (except for DC and Nyquist)
FFTexampleFULL_db = 20*log10(PexampleFULL);  % Convert to dB
f_exampleFULL = Fs*(0:(N/2))/N; % Frequency vector for the plot

colIdeal = [0/255 145/255 206/255];
colMeasured = [255/255 255/255 0/255];

f_exampleFULL(end) = [];
FFTexampleFULL_db(end) = [];
semilogx(f_exampleFULL, FFTexampleFULL_db, '-', 'Color', colIdeal, 'LineWidth', 1.5);

hold on;
f_FFFfirstandsecond(end) = [];
FFTfirstandsecond_db(end) = [];
semilogx(f_FFFfirstandsecond, FFTfirstandsecond_db, '-', 'Color', colMeasured, 'LineWidth', 0.5);

text(0.04, 0.97, 'Ideal Frequency Response', 'Color', colIdeal, 'FontSize', 8, 'Units', 'normalized');
text(0.04, 0.93, 'Resampled Signal', 'Color', colMeasured, 'FontSize', 8, 'Units', 'normalized');

xlim([0, Fs/2])
ylim([YdBMin, 0])
xlabel('Frequency (Hz)');
ylabel('Magnitude (dB)');
set(gca,'Color','k')
set(gcf,'InvertHardCopy','Off');
set(gca, 'TickDir', 'out', 'YTick', YdBMin:20:0, 'XMinorTick', 'on', 'YMinorTick', 'on', 'Box', 'off', 'XGrid', 'on', 'YGrid', 'on')
set(gca, 'FontSize', 8)
% Get the current axes handle
ax = gca;
set(ax, "gridcolor", [0.5 0.5 0.5]);
set(ax, "gridlinestyle", "--"); % Dashed lines
set(ax, "gridalpha", 0.7);     % 70% transparency
hold off;

set(h, 'paperunits', 'centimeters');
set(h, 'papersize', [20 10]);
set(h, 'paperposition', [0 0 20 10]);
print(h, '-dpng', fullfile(output_dir, 'gaplesstest-frequency.png'), '-r200');

% -------------------------DO QUALITY-------------------------

%upsample the exampleFULL, then create example_upsampledSHORT from that

% do FFT up sample then iFFT and grab data
n_exampleFULL = length(exampleFULL);
XFFT_n_exampleFULL = fft(exampleFULL);
exampleFULL_upsampledFFT = zeros(1, n_exampleFULL * upsample_factor);
mid_point_exampleFULL = ceil(n_exampleFULL/2);
exampleFULL_upsampledFFT(1:mid_point_exampleFULL) = XFFT_n_exampleFULL(1:mid_point_exampleFULL);
exampleFULL_upsampledFFT(end-mid_point_exampleFULL+1:end) = XFFT_n_exampleFULL(mid_point_exampleFULL:end);
exampleFULL_upsampledasRealNImag = ifft(exampleFULL_upsampledFFT);
exampleFULL_upsampledasRealNImag = exampleFULL_upsampledasRealNImag * upsample_factor;
exampleFULL_upsampled = real(exampleFULL_upsampledasRealNImag);
%t_exampleFULL_upsampled = ([1:length(exampleFULL_upsampled)]) / (Fs * upsample_factor);

example_upsampledSHORT = exampleFULL_upsampled((cutAtFinalSR - samplesshowaroundbreak - 1) * upsample_factor : (cutAtFinalSR + samplesshowaroundbreak - 1) * upsample_factor);
%t_example_upsampledSHORT = ([0:length(example_upsampledSHORT) - 1]) / (Fs * upsample_factor);


counter = 10;
average = 100000000000;

max = length(example_upsampledSHORT);
if (length(firstandsecond_upsampledSHORT) < max)
  max = length(firstandsecond_upsampledSHORT);
end
for offset=0:50
  thiscounter = 0;
  thisaverage = 0;
  for index=1:max - 1 - offset
    difference = abs(example_upsampledSHORT(index) - firstandsecond_upsampledSHORT(index + offset));
    thisaverage = thisaverage + difference;
    thiscounter++;
  end
  if (thisaverage < average)
      counter = thiscounter;
      average = thisaverage;
  end
end
average = average / counter;
average = average * 100000;
if (average < 1)
  average = 1;
end
qualityresult = 1 / average;
qualityresult = qualityresult * 10000;
if (qualityresult > 100)
  qualityresult = 100;
end

fid = fopen(fullfile(output_dir, 'quality-gapless.txt'), "w"); % Open for writing, overwrites if exists
fprintf(fid, '%f', qualityresult);
fclose(fid);

close(h);

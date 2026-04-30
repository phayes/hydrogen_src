
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

[y, Fs] = audioread(fullfile(input_dir, 'intermodulation_sine.wav'));
[y44, Fs44] = audioread(fullfile(input_dir, 'intermodulation_sine-InternalUseAt44kHz.wav'));

overlap = 0.5;
win_size = 16384 /4;
nfft = win_size;       % FFT size. Keep same as window size for this purpose.

w = blackmanharris(win_size);
[Pxx, f] = pwelch(y, w, overlap, nfft, Fs);

w44 = blackmanharris(win_size);
[Pxx44, f44] = pwelch(y44, w44, overlap, nfft, Fs44);

colIdeal = [0/255 145/255 206/255];
colMeasured = [255/255 255/255 0/255];

plot(f44/1000, 10*log10(Pxx44), '-', 'Color', colIdeal, 'LineWidth', 1.5)
hold on;
plot(f/1000, 10*log10(Pxx), '-', 'Color', colMeasured, 'LineWidth', 0.5)

set(gca,'Color','k')
xlabel('Frequency (kHz)');
ylabel('Power (dBFS)');

xlim([-0.1,Fs/2000 + 0.1]);
YdBMin = -280;
ylim([YdBMin, 0]);

colBits = [128/255 128/255 128/255];

text(0.04, 0.97, 'Ideal Frequency Response', 'Color', colIdeal, 'FontSize', 8, 'Units', 'normalized');
text(0.04, 0.93, 'Measured Frequency', 'Color', colMeasured, 'FontSize', 8, 'Units', 'normalized');

plot([-0.1, Fs], [-96, -96], '--', 'Color', colBits, 'LineWidth', 0.75)
plot([-0.1, Fs], [-144, -144], '--', 'Color', colBits, 'LineWidth', 0.75)
plot([-0.1, Fs], [-192, -192], '--', 'Color', colBits, 'LineWidth', 0.75)

text(Fs/2000*0.05, -90, '-96 dBFS 16 bit', 'Color', colBits, 'FontSize', 8);
text(Fs/2000*0.05, -138, '-144 dBFS 24 bit', 'Color', colBits, 'FontSize', 8);
text(Fs/2000*0.05, -186, '-192 dBFS 32 bit', 'Color', colBits, 'FontSize', 8);

hold off


set(gca, 'TickDir', 'out', 'XTick', 0:2:Fs/2000, 'YTick', YdBMin:20:0, 'XMinorTick', 'on', 'YMinorTick', 'off', 'Box', 'off', 'XGrid', 'off', 'YGrid', 'on')
set(gcf,'InvertHardCopy','Off');
set(gca, 'FontSize', 8)

% Get the current axes handle
ax = gca;
set(ax, "gridcolor", [0.5 0.5 0.5]);
set(ax, "gridlinestyle", "--"); % Dashed lines
set(ax, "gridalpha", 0.7);     % 70% transparency
set(h, 'paperunits', 'centimeters');
set(h, 'papersize', [20 10]);
set(h, 'paperposition', [0 0 20 10]);
print(h, '-dpng', fullfile(output_dir, 'intermodulation-harmonic-distortion.png'), '-r200')


%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%
% Difference
%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%
while (length(y) > length(y44))
    y(end) = [];
end
while (length(y44) > length(y))
    y44(end) = [];
end
yDifference = y - y44;

overlap_diff = 0.5;
win_size_diff = 16384 / 16;
nfft_diff = win_size_diff;       % FFT size. Keep same as window size for this purpose.


wDiff = hanning(win_size_diff);
[PxxDiff, fDiff] = pwelch(yDifference, wDiff, overlap_diff, nfft_diff, Fs);

LogDiff = 10*log10(PxxDiff);
for index=1:length(LogDiff)   % clamp on graph - ideal plot is off scale
  if (LogDiff(index) < -355)
    LogDiff(index) = -355;
  end
end

plot(fDiff/1000, LogDiff, '-', 'Color', colMeasured, 'LineWidth', 0.5)
hold on;

set(gca,'Color','k')
xlabel('Frequency (kHz)');
ylabel('Power (dBFS)');


xlim([-0.1,Fs/2000 + 0.1]);
YdBMin = -360;
YdBMax = 0;
ylim([YdBMin, YdBMax]);

text(0.04, 0.97, 'Difference (Ideal - Measured)', 'Color', colMeasured, 'FontSize', 8, 'Units', 'normalized');

set(gca, 'TickDir', 'out', 'XTick', 0:2:Fs/2000, 'YTick', YdBMin:20:YdBMax, 'XMinorTick', 'on', 'YMinorTick', 'off', 'Box', 'off', 'XGrid', 'off', 'YGrid', 'on')
set(gcf,'InvertHardCopy','Off');
set(gca, 'FontSize', 8)

% Get the current axes handle
ax = gca;
set(ax, "gridcolor", [0.5 0.5 0.5]);
set(ax, "gridlinestyle", "--"); % Dashed lines
set(ax, "gridalpha", 0.7);     % 70% transparency
set(h, 'paperunits', 'centimeters');
set(h, 'papersize', [20 10]);
set(h, 'paperposition', [0 0 20 10]);
print(h, '-dpng', fullfile(output_dir, 'intermodulation-harmonic-distortion-difference.png'), '-r200')

hold off;

%------------------quality----------------------
averageDiff = abs(mean(LogDiff));
qualityscore = ((averageDiff - 140) * 100) / 140; % the highest practical

if (qualityscore > 100)
  qualityscore = 100;
end
if (qualityscore < 0)
  qualityscore = 0;
end

fid = fopen(fullfile(output_dir, 'quality-intermoddiff.txt'), "w"); % Open for writing, overwrites if exists
fprintf(fid, '%f', qualityscore);
fclose(fid);


% Close the figure after saving.
close(h);


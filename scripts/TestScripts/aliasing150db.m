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

[y, Fs] = audioread(fullfile(input_dir, 'aliasing150db.wav'));
overlap = 0.5;
segments = 4;
w = hanning(floor(rows(y) / segments / overlap));
w /= norm(w);
[Pxx, f] = pwelch(y, w, overlap, rows(w), Fs);
yPlot = 10*log10(Pxx) - 10;
plot(f/1000, yPlot, '-', 'Color', [0/255 145/255 206/255], 'LineWidth', 1.5)
set(gca,'Color','k')
xlabel('Frequency (kHz)');
ylabel('Power (dBFS)');
xlim([0,Fs/2000]);

YdBMin = -240;
ylim([YdBMin, 0]);

colBits = [128/255 128/255 128/255];

hold on
  plot([0, Fs], [-150, -150], '--', 'Color', [254/255 227/255 0/255], 'LineWidth', 1.75)

  plot([0, Fs], [-96, -96], '--', 'Color', colBits, 'LineWidth', 1)
  plot([0, Fs], [-144, -144], '--', 'Color', colBits, 'LineWidth', 1)
  plot([0, Fs], [-192, -192], '--', 'Color', colBits, 'LineWidth', 1)
hold off

text(Fs/2000*0.8, -144, '-150 dBFS Noise', 'Color', [254/255 227/255 0/255], 'FontSize', 10);
text(Fs/2000*0.05, -90, '-96 dBFS 16 bit', 'Color', colBits, 'FontSize', 8);
text(Fs/2000*0.05, -138, '-144 dBFS 24 bit', 'Color', colBits, 'FontSize', 8);
text(Fs/2000*0.05, -186, '-192 dBFS 32 bit', 'Color', colBits, 'FontSize', 8);


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
print(h, '-dpng', fullfile(output_dir, 'aliasing150db.png'), '-r200')

%------------------zoomed to show Nyquist filter----------------
%------------------zoomed to show Nyquist filter----------------
%------------------zoomed to show Nyquist filter----------------
%------------------zoomed to show Nyquist filter----------------
FreqFrom = 18000;
FreqTo = (44100 / 2);
NeedFrom = floor((length(f) * FreqFrom) / FreqTo);
PlotYZoomed = yPlot(NeedFrom:end);

% average either neighbour to smooth the image
AVFreqFrom = 6000;
NeedAVFrom = floor((length(f) * AVFreqFrom) / FreqTo);
YAveraged = yPlot(NeedAVFrom:end);
for index=1:1000
  YAveraged = movmean(YAveraged, 3);
end
%apply a hard window
ClipBelow = -154;
ClipAbove = -160;
ClipTo = ClipBelow + ((ClipAbove - ClipBelow) / 2);

for index=1:length(YAveraged) - 1
  if (YAveraged(index) < ClipBelow && YAveraged(index) > ClipAbove)
    YAveraged(index) = ClipTo;
  endif
end

fZoomed = f(NeedFrom:end);

plot(fZoomed/1000, PlotYZoomed, 'Color', [0/255 145/255 206/255], 'LineWidth', 1)
set(gca,'Color','k')
xlabel('Frequency (kHz)');
ylabel('Power (dBFS)');
xlim([FreqFrom/1000, FreqTo/1000]);

YdBMin = -240;
PlotYUpper = -140;
ylim([YdBMin, PlotYUpper]);

colBits = [128/255 128/255 128/255];

hold on
  plot([0, Fs], [-150, -150], '--', 'Color', [254/255 227/255 0/255], 'LineWidth', 1.75)
hold off

text(Fs/2000*0.92, -146, '-150 dBFS', 'Color', [254/255 227/255 0/255], 'FontSize', 10);

set(gca, 'TickDir', 'out', 'XTick', FreqFrom/1000:1:FreqTo/1000, 'YTick', YdBMin:20:PlotYUpper, 'XMinorTick', 'on', 'YMinorTick', 'off', 'Box', 'off', 'XGrid', 'off', 'YGrid', 'on')
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
print(h, '-dpng', fullfile(output_dir, 'nyquist-filter.png'), '-r200')



%--------------generate quality (BW)------------------------------
counter = 0;
numatclipto = 0;
for index=1:length(YAveraged) - 1
  if (YAveraged(index) == ClipTo)
    numatclipto = numatclipto + 1;
  endif
  counter = counter + 1;
end

quality = (numatclipto * 100) / counter;


fid = fopen(fullfile(output_dir, 'quality-bandwidth.txt'), "w"); % Open for writing, overwrites if exists
fprintf(fid, '%f', quality);
fclose(fid);

%--------------generate quality (Aliasing Spikes)------------------------------


HalfMainY = yPlot(1:end / 2);   % we need the middle of the noise, before likely filter rolloff
AveragedHalfMainPlotY = mean(HalfMainY);
SpikeIsAbove = AveragedHalfMainPlotY + 15;    % should be around -140db

counterAliasSpikes = 0;
isbelow = 1;
for index=1:length(yPlot) - 1
  if (isbelow == 1)
    if (yPlot(index) > SpikeIsAbove)
      counterAliasSpikes = counterAliasSpikes + 1;
      isbelow = 0;
    end
  else
    if (yPlot(index) <= SpikeIsAbove)
      isbelow = 1;
    end
  end
end

qualityAlias = 100;
for index=1:counterAliasSpikes
  qualityAlias = qualityAlias / 2;    % first alias 50%  2nd 25%  3rd  11.25% etc
end

fid = fopen(fullfile(output_dir, 'quality-alias.txt'), "w"); % Open for writing, overwrites if exists
fprintf(fid, '%f', qualityAlias);
fclose(fid);


% Close the figure after saving.
close(h);


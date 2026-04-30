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

[y1, Fs1] = audioread(fullfile(input_dir, 'impulse-64bitfloat-InternalUse.wav')); % unprocessed 96KHz
[y2, Fs2] = audioread(fullfile(input_dir, 'impulse.wav'));   % processed 44KHz

% work out delay first impulse should be 2366*impulse_multiplier
impulse_multiplier = 6;
posfirstimpulse=1;
for index=1:length(y2)-1
  if (y2(index) > 0.2)
    posfirstimpulse = index;
    break;
  end
end
delaytoremove=posfirstimpulse-(2366*impulse_multiplier);
calcdelay=0;
if (delaytoremove > 1)
  y2 = y2(delaytoremove:end);
%  y2 = cat(1, y2(delaytoremove:end), zeros(delaytoremove,1));
  calcdelay = delaytoremove - 1;
end
if (delaytoremove < 0)
  numberofblanksadd = delaytoremove * -1;
  y2 = cat(1, zeros(numberofblanksadd,1), y2);
  calcdelay = delaytoremove;
end

y2len = length(y2);
needLen = floor((length(y1) * 44100) / 96000) + 1;
if (y2len < needLen)
  extraadd = needLen - y2len;
  y2(needLen) = 0;
end

% the calculated delay isn't accurate
%filename = "calculateddelay.txt";
%fid = fopen(filename, "w"); % Open for writing, overwrites if exists
%fprintf(fid, '%d\n', calcdelay);
%fclose(fid);



pos = find(y1) - 1;
ggt = gcd(Fs1, Fs2);
x1 = pos(1);
dx = pos(2) - pos(1);
rdx1 = dx - mod(dx, ggt);
rdx2 = rdx1 * Fs2 / Fs1;
f1 = Fs1 / ggt;
f2 = Fs2 / ggt;
if mod(rdx2, 2) != 0
  printf("rdx2 is not even\n");
  tdx = ceil(rdx1 / f1 / 2) * 2;
  rdx1 = tdx * f1;
  rdx2 = tdx * f2;
end;
len = rdx1 * f2;
res = zeros(len + 1, 1);
mid = len / 2;
% printf("Fs1: %d, Fs2: %d, x1: %d, dx: %d, rdx1: %d, rdx2: %d, len: %d, mid: %d\n", Fs1, Fs2, x1, dx, rdx1, rdx2, len, mid);

for index=1:f1
	mid1 = round(pos(index) / f1) * f1;
	mid2 = mid1 * Fs2 / Fs1;
	% x1 = upsample(y1(mid1 - rdx1/2 + 1:mid1 + rdx1/2), f2);
	x2 = upsample(y2(mid2 - rdx2/2 + 1:mid2 + rdx2/2), f1);
	diff = (pos(index) - (mid1 - rdx1/2)) * f2 - mid;
	if diff >= 0
		res(1 : len - diff) += x2(diff + 1 : len);
	else
		res(1 - diff : len) += x2(1 : len + diff);
	end
end

% audiowrite('res-impulse-64bitfloat.wav', res, Fs2, 'BitsPerSample', 64); % real samplerate would be Fs2*f1 but audio editors don't like Hz that high

% find possible subsample delay
peak=0;
pidx1=0;
pidx2=0;
for index=floor(length(res)/2-15000/2):floor(length(res)/2+15000/2)
  s=res(index);
  if (s > peak)
    peak=s;
    pidx1=index;
    pidx2=index;
  elseif (s == peak)
    pidx2=index;
  end
end
% printf("peak: %f, idx1: %d, idx2: %d, mid: %f\n", peak, pidx1, pidx2, (pidx1+pidx2)/2);
idealpeakpos=4233601;
peakdelta=(pidx1+pidx2)/2 - idealpeakpos;
subsampledelay=floor((peakdelta / f1) * (Fs1 / Fs2));
printf("subsample delay: %d\n", subsampledelay);
% align the impulse peak at the center - fixes phase, pre-ringing checks, impulse waveform
if (subsampledelay > 0)
  res = cat(1, res(floor(peakdelta):end), zeros(floor(peakdelta),1));
elseif (subsampledelay < 0)
  res = cat(1, zeros(floor(-peakdelta),1), res(1:length(res)-floor(-peakdelta)));
end

filename = fullfile(output_dir, "calculateddelay.txt");
fid = fopen(filename, "w"); % Open for writing, overwrites if exists
% fprintf(fid, '%d\n', round(subsampledelay * Fs2 / Fs1)); % convert subsample delay to sample delay
finaldelay = subsampledelay + calcdelay
fprintf(fid, '%d\n', finaldelay); % write subsample delay
fclose(fid);


ColIdeal = [0/255 145/255 206/255];
ColMeasured = [254/255 227/255 0/255];
%-----------------------------------------------------------------------------------------------------------------------------
samplesaroundmidneed = 15000;
impulseresponse_y = res(mid-samplesaroundmidneed:mid+samplesaroundmidneed);
impulseresponse_y = impulseresponse_y * 2;
impulseresponse_x = ([1:size(impulseresponse_y,1)] - (samplesaroundmidneed)) / f2;

plot(impulseresponse_x, impulseresponse_y, 'LineWidth', 1.25, 'Color', ColMeasured);

Yrangemin = -0.5;
Yrangemax = 1.0;
xlim([impulseresponse_x(1), impulseresponse_x(end)])
ylim([Yrangemin, Yrangemax])

yleft_ticks = (Yrangemin:0.1:Yrangemax);
yleft_ticks_labels = num2str(yleft_ticks',3);
yright_ticks = 20*log10(abs(yleft_ticks));
yright_ticks_labels = num2str(yright_ticks',2);
set(gca, 'YTick', yleft_ticks,'YTickLabel', yright_ticks_labels );
xlabel('Samples');
ylabel('Amplitude (dB)');
set(gca,'Color','k')
set(gcf,'InvertHardCopy','Off');
set(gca, 'TickDir', 'out', 'YMinorTick', 'off', 'Box', 'on', 'XGrid', 'on', 'YGrid', 'on')
set(gca, 'FontSize', 8)

text(0.04, 0.97, 'Impulse Response', 'Color', ColMeasured, 'FontSize', 8, 'Units', 'normalized');

text(0.45, 0.1, '[Pre-ringing]', 'Units', 'normalized', 'HorizontalAlignment', 'right', 'fontsize', 10, 'Color', [128/255 128/255 128/255]);
text(0.55, 0.1, '[Post-ringing]', 'Units', 'normalized', 'fontsize', 10, 'Color', [128/255 128/255 128/255]);

% Get the current axes handle
ax = gca;
set(ax, "gridcolor", [0.5 0.5 0.5]);
set(ax, "gridlinestyle", "--"); % Dashed lines
set(ax, "gridalpha", 0.7);     % 70% transparency
set(h, 'paperunits', 'centimeters');
set(h, 'papersize', [20 10]);
set(h, 'paperposition', [0 0 20 10]);
print(h, '-dpng', fullfile(output_dir, 'impulse-response.png'), '-r200');

%--------quality based on pre-ringing-------------
PreRingingY = impulseresponse_y(1:length(impulseresponse_y) / 2);
PreRingingY = abs(PreRingingY);
averagePreRinging = mean(PreRingingY);
averagePreRinging = averagePreRinging * 1000;
averagePreRinging = averagePreRinging - 2;
if (averagePreRinging < 0)
  averagePreRinging = 0;
end
averagePreRinging = ((30 - averagePreRinging) * 100) / 30;
if (averagePreRinging > 100)
  averagePreRinging = 100;
end
fid = fopen(fullfile(output_dir, 'quality-preringing.txt'), "w"); % Open for writing, overwrites if exists
fprintf(fid, '%f\n', averagePreRinging);
fclose(fid);

%-----------------------------------------------------------------------------------------------------------------------------

fftres = fft(res / f1);
f = [0:rows(res)-1] * Fs2 * f1 / rows(res) / 1000;
amplitude = 20 * log10(Fs2/Fs1);
YdBMin = -380;
impulsefreq = 20 * log10(abs(fftres));
plot([0, Fs2/2000, Fs2/2000], [amplitude, amplitude, YdBMin], '--', 'Color', ColIdeal, 'LineWidth', 1.25, f, impulsefreq, '-', 'Color', ColMeasured, 'LineWidth', 0.75)


text(0.04, 0.97, 'Ideal Frequency Response', 'Color', ColIdeal, 'FontSize', 8, 'Units', 'normalized');
text(0.04, 0.93, 'Measured Frequency', 'Color', ColMeasured, 'FontSize', 8, 'Units', 'normalized');

xlim([0, Fs1/100])
ylim([YdBMin, 0])
xlabel('Frequency (kHz)');
ylabel('Magnitude (dB)');
set(gca,'Color','k')
set(gcf,'InvertHardCopy','Off');
set(gca, 'TickDir', 'out', 'XTick', 0:Fs1/1000:Fs1*f2/10000, 'YTick', YdBMin:20:0, 'XMinorTick', 'on', 'YMinorTick', 'on', 'Box', 'off', 'XGrid', 'on', 'YGrid', 'on')
set(gca, 'FontSize', 8)
% Get the current axes handle
ax = gca;
set(ax, "gridcolor", [0.5 0.5 0.5]);
set(ax, "gridlinestyle", "--"); % Dashed lines
set(ax, "gridalpha", 0.7);     % 70% transparency

set(h, 'paperunits', 'centimeters');
set(h, 'papersize', [20 10]);
set(h, 'paperposition', [0 0 20 10]);
print(h, '-dpng', fullfile(output_dir, 'impulse-frequency.png'), '-r200');

% work out quality based on average
fullFreqRamge = Fs1*f2;
calcimpulsefrom = 50000;
pointFrom = floor(calcimpulsefrom * length(impulsefreq) / fullFreqRamge);
calcArray = impulsefreq(pointFrom:end);
for index=1:length(calcArray)-1
  if (calcArray(index) < -400 || isinf(calcArray(index)))
    calcArray(index) = -400;
  end
end

average_impulse_freq = mean(calcArray);

qualityPC = average_impulse_freq;
qualityPC = qualityPC * -1;
qualityPC = (qualityPC * 100) / 338;
if (qualityPC > 100)
  qualityPC = 100;
end


fid = fopen(fullfile(output_dir, 'quality-impulse_freq.txt'), "w"); % Open for writing, overwrites if exists
fprintf(fid, '%f\n', average_impulse_freq);
fprintf(fid, '%f\n', qualityPC);
fclose(fid);
%-----------------------------------------------------------------------------------------------------------------------------
plot([0, Fs2/2000, Fs2/2000], [0, 0, -200], '--', 'Color', ColIdeal, 'LineWidth', 1.25)
hold on
impulsepass = 20 * log10(abs(fftres)/abs(fftres(1)));
plot(f, impulsepass, '-', 'Color', ColMeasured, 'LineWidth', 1.25)
hold off
xlim([0, Fs2/2000]*1.1)
ylim([-3, 1.5])

  % **** this is also on next graph ****fs
text(0.02, 0.97, 'Ideal Filter Transition', 'Color', ColIdeal, 'FontSize', 8, 'Units', 'normalized');
text(0.02, 0.93, 'Measured Transition', 'Color', ColMeasured, 'FontSize', 8, 'Units', 'normalized');

xlabel('Frequency (kHz)');
ylabel('Magnitude (dB)');
set(gca,'Color','k')
set(gcf,'InvertHardCopy','Off');
set(gca, 'TickDir', 'out', 'XTick', 0:2:Fs1/2000, 'YTick', -3:0.5:1.5, 'XMinorTick', 'on', 'YMinorTick', 'on', 'Box', 'off', 'XGrid', 'on', 'YGrid', 'on')
set(gca, 'FontSize', 8)
% Get the current axes handle
ax = gca;
set(ax, "gridcolor", [0.5 0.5 0.5]);
set(ax, "gridlinestyle", "--"); % Dashed lines
set(ax, "gridalpha", 0.7);     % 70% transparency
set(h, 'paperunits', 'centimeters');
set(h, 'papersize', [20 10]);
set(h, 'paperposition', [0 0 20 10]);
print(h, '-dpng', fullfile(output_dir, 'impulse-passband.png'), '-r200');

% zoomed graph, using existing

set(gca, 'TickDir', 'out', 'XTick', 0:1:Fs2*1.1/2000, 'YTick', -96:6:6, 'XMinorTick', 'on', 'YMinorTick', 'on', 'Box', 'off', 'XGrid', 'on', 'YGrid', 'on')
xlim([0.8, 1.1]*Fs2/2000)
ylim([-100, 12])

set(h, 'paperunits', 'centimeters');
set(h, 'papersize', [20 10]);
set(h, 'paperposition', [0 0 20 10]);
print(h, '-dpng', fullfile(output_dir, 'impulse-transition.png'), '-r200');

%-------work out if need to have different sample set for impulse, if there is a presense between 22.2 kHz and 24KHz-----
accumulator_av = 0;
accumulator_ctr = 0;

for index=1:length(f)
  if (f(index) >= 22.200)
     accumulator_ctr = accumulator_ctr + 1;
     accumulator_av = accumulator_av + impulsepass(index);
  end
  if (f(index) > 24.000)
    break;
  end
end
if (accumulator_ctr == 0)
  accumulator_ctr = 1;
end
accumulator_av = accumulator_av / accumulator_ctr;

if (accumulator_av > -90)
 fid = fopen(fullfile(output_dir, 'impulse_recommend_spaced_impulse.txt'), "w"); % Open for writing, overwrites if exists
 fprintf(fid, '1\n');
 fclose(fid);
end
%-----------------------------------------------------------------------------------------------------------------------------

phi = angle(fftres);

plot([0, Fs2/2000], [0,0], '--', 'Color', ColIdeal, 'LineWidth', 1.25, f, unwrap(phi*2) * 90 / pi, '-', 'Color', ColMeasured, 'LineWidth', 0.75)

text(0.02, 0.97, 'Ideal Phase', 'Color', ColIdeal, 'FontSize', 8, 'Units', 'normalized');
text(0.02, 0.93, 'Measured Phase', 'Color', ColMeasured, 'FontSize', 8, 'Units', 'normalized');

xlim([0, Fs2/2000])
ylim([-180, 180])
xlabel('Frequency (kHz)');
ylabel('Phase (deg)');
set(gca,'Color','k')
set(gcf,'InvertHardCopy','Off');

set(gca, 'TickDir', 'out', 'XTick', 0:2:Fs1/2000, 'YTick', -180:30:180, 'XMinorTick', 'on', 'YMinorTick', 'on', 'Box', 'off', 'XGrid', 'on', 'YGrid', 'on')
set(gca, 'FontSize', 8)

% Get the current axes handle
ax = gca;
set(ax, "gridcolor", [0.5 0.5 0.5]);
set(ax, "gridlinestyle", "--"); % Dashed lines
set(ax, "gridalpha", 0.7);     % 70% transparency

set(h, 'paperunits', 'centimeters');
set(h, 'papersize', [20 10]);
set(h, 'paperposition', [0 0 20 10]);
print(h, '-dpng', fullfile(output_dir, 'impulse-phase.png'), '-r200');

%-----------------------------------------------------------------------------------------------------------------------------

% Close the figure after saving.
close(h);


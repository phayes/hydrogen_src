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

[y, fs] = audioread(fullfile(input_dir, 'sweep_1_to_44KHz.wav'));

window_length = 8192; % Size of the Hann window
window_overlap = round(window_length*0.97);
zero_pad_factor = 2;
nfft = window_length * zero_pad_factor;

%[S, F, T] = spectrogram(y, hann(window_length), window_overlap, nfft, fs, 'yaxis');
[S, F, T] = specgram(y, nfft, fs, hann(window_length), window_overlap);
S = abs(S);
S = S/max(S(:));
S_dB = 20*log10(S); % Convert magnitude to dB

imagesc(T, F/1000, S_dB);
axis xy;
%max_dB = max(S_dB(:));
max_dB = 0;
clim([max_dB - 180, max_dB]);
colormap('hot')

% scale plot is disabled, only enabled to extract scale statically
%c_limits = caxis;
%tick_interval = 20;
%min_tick = floor(c_limits(1) / tick_interval) * tick_interval;
%max_tick = ceil(c_limits(2) / tick_interval) * tick_interval;
%ticks = min_tick:tick_interval:max_tick;
%hcb = colorbar;
%set(hcb, 'YTick', ticks);
%ylabel(hcb, 'dBFS');

xlabel('Time (s)');
ylabel('Frequency (kHz)');
set(gca, 'TickDir', 'out', 'YTick', 0:2:fs/2000, 'XMinorTick', 'on', 'YMinorTick', 'on', 'YDir', 'normal')
set(gca, 'FontSize', 8)
set(h, 'paperunits', 'centimeters');
set(h, 'papersize', [20 10]);
set(h, 'paperposition', [0 0 20 10]);
print(h, '-dpng', fullfile(output_dir, 'sweep-1-to-44KHz.png'), '-r200');

%------now zoomed--------
xlim([0,11]);

%ylim([20000 / 1000, 23000 / 1000]);	% test code for zooming close on nyquist
%xlim([9.5,10.5]);

set(h, 'paperunits', 'centimeters');
set(h, 'papersize', [20 10]);
set(h, 'paperposition', [0 0 20 10]);
print(h, '-dpng', fullfile(output_dir, 'sweep-1-to-44KHz-1to11sec.png'), '-r200');

% create high res version for quality test
xticks([])    % Remove all x-axis ticks and labelsfs
yticks([])  % Remove all y-axis ticks and labels
xlabel('');
ylabel('');
set(h, 'paperunits', 'centimeters');
set(h, 'papersize', [120 60]);
set(h, 'paperposition', [0 0 120 60]);
print(h, '-dpng', fullfile(output_dir, 'sweep-1-to-44KHz-1to11secHighRES.png'), '-r200');


% Close the figure after saving.
close(h);

% -------------------------DO QUALITY-------------------------
I1 = imread(fullfile(output_dir, 'sweep-1-to-44KHz-1to11secHighRES.png'));
I2 = imread(fullfile(input_dir, 'sweep-1-to-44KHz-1to11secHighRES-REF.png'));


% Convert to grayscale if they are color images, as most quality metrics operate on grayscale
if size(I1, 3) == 3
    I1 = rgb2gray(I1);
end
if size(I2, 3) == 3
    I2 = rgb2gray(I2);
end

% Calculate the Mean Squared Error (MSE) - closer to 0 means better match
mse_score = immse(I2, I1);

% Display the scores
%fprintf('Mean Squared Error (MSE) closer to 0 means better match: %f\n', mse_score);

if (mse_score > 400)
  mse_score = 400;
end
qualityresult = (400 - mse_score) / 4;    %should be in a % closer to 100% the better

fid = fopen(fullfile(output_dir, 'quality-spectrogram.txt'), "w"); % Open for writing, overwrites if exists
fprintf(fid, '%f', qualityresult);
fclose(fid);


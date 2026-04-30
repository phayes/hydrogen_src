
function average = bit_depth_calc_av(filename_str)

  pkg load signal;
  pkg load image;

  if exist('input_dir', 'var') && !isempty(input_dir)
    [parent_dir, ~, ~] = fileparts(filename_str);
    if isempty(parent_dir)
      filename_str = fullfile(input_dir, filename_str);
    endif
  endif

  [y, Fs] = audioread(filename_str);
  N = 256;
  Y = fft(y, N);   % Compute the N-point FFT
  P2 = abs(Y / N);
  P1 = P2(1:N/2+1);
  P1(2:end-1) = 2 * P1(2:end-1);
  P1_dB = 20 * log10(P1);
%  f = Fs * (0:(N/2)) / N;
%  plot(f/1000, movmean(P1_dB, [20 20]), '-', 'Color', col64fbit, 'LineWidth', 3);
  average = mean(P1_dB);

endfunction


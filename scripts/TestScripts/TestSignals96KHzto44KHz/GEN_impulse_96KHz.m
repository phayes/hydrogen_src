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

impulse_multiplier = 6;    
Fs = 96000;         % Sampling frequency (samples per second)
amplitude = 1;
startImpulsePos = 5150 * impulse_multiplier;
impulseEvery = 9637 * impulse_multiplier;
if mod(impulseEvery, 2) == 0    % cannot be even
    impulseEvery = impulseEvery + 1;
end
SampleCount = startImpulsePos + (impulseEvery * 320) - 1;

modverify = zeros(320, 1);
for i = startImpulsePos : impulseEvery : SampleCount
  modverify(mod(i, 320) + 1) = modverify(mod(i, 320) + 1) + 1;
end

extras = 0;
missing = 0;
for i = 1 : 320
  if modverify(i) == 0 
      missing = missing + 1; 
  end
  if modverify(i) > 1 
      extras = extras + 1; 
  end
end

if extras >  0 || missing > 0
  printf("Aborting. ");
  if missing > 0
    printf("Modulo missing for %d values. ", missing);
  end
  if extras > 0
    printf("Extra hits for %d values. ", extras);
  end
  printf("\n");
  quit(1);
end
y = zeros(1, SampleCount); 

for i = startImpulsePos : impulseEvery : SampleCount
    y(i + 1) = amplitude;
end

y32bitint = int32(y * 2147483647);
audiowrite(fullfile(dir64, 'impulse-64bitfloat.wav'), y, Fs, 'BitsPerSample', 64);
audiowrite(fullfile(dir32f, 'impulse-32bitfloat.wav'), y, Fs, 'BitsPerSample', 32);
audiowrite(fullfile(dir32i, 'impulse-32bitint.wav'), y32bitint, Fs, 'BitsPerSample', 32);
audiowrite(fullfile(dir24i, 'impulse-24bitint.wav'), y, Fs, 'BitsPerSample', 24);

audiowrite(fullfile(output_root, 'impulse-64bitfloat-InternalUse.wav'), y, Fs, 'BitsPerSample', 64);


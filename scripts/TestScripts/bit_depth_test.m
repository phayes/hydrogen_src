
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

filename = fullfile(output_dir, "bitdepthresult.txt");
fid = fopen(filename, "w"); % Open for writing, overwrites if exists


faverage16 = fullfile(input_dir, 'bitdepth_16bitint.wav');
faverage24 = fullfile(input_dir, 'bitdepth_24bitint.wav');
faverage32 = fullfile(input_dir, 'bitdepth_32bitint.wav');
faverage32f = fullfile(input_dir, 'bitdepth_32bitfloat.wav');
faverage64f = fullfile(input_dir, 'bitdepth_64bitfloat.wav');


if isfile(faverage16)
 average16 = bit_depth_calc_av(faverage16);
 if (average16 < -1 && average16 > -96)
   fputs(fid, "16\n");
 end
end

if isfile(faverage24)
 average24 = bit_depth_calc_av(faverage24);
 if (average24 < -150 && average24 > -180)
   info = audioinfo(faverage24);
   bitDepth = info.BitsPerSample;
   if (bitDepth >= 24)
      fputs(fid, "24\n");
   end
 end
end

if isfile(faverage32)
 average32 = bit_depth_calc_av(faverage32);
 if (average32 < -200 && average32 > -230)
   info = audioinfo(faverage32);
   bitDepth = info.BitsPerSample;
   if (bitDepth >= 32)
      fputs(fid, "32\n");
   end
 end
end

if isfile(faverage32f)
 average32f = bit_depth_calc_av(faverage32f);
 if (average32f < -200 && average32f > -230)
   info = audioinfo(faverage32f);
   bitDepth = info.BitsPerSample;
   if (bitDepth >= 32)
       fputs(fid, "32f\n");
   end
 end
end

if isfile(faverage64f)
 average64f = bit_depth_calc_av(faverage64f);
 if (average64f < -240 && average64f > -300)
   info = audioinfo(faverage64f);
   bitDepth = info.BitsPerSample;
   if (bitDepth == 64)
      fputs(fid, "64f\n");
   end
 end
end

fclose(fid);


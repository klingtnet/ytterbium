function freqs = freqs(fs, wave)
  len = length(wave);
  if (mod(len, 2) == 0)
      t = (-len/2)+1 : len/2;
  else
      t = -floor(len/2) : floor(len/2);
  end
  freqs = t * fs/len;
endfunction

function spectrum = spectrum(wave)
  spectrum = fftshift(
    abs(fft(wave))
  );
  % normalize
  spectrum /= max(spectrum);
endfunction

function spectrum_plot(fs, wave, plot_title)
  frq = freqs(fs, wave);
  spc = spectrum(wave);
  semilogx(frq, spc, ".-");
  title(plot_title);
  xlabel('f');
  ylabel('|f|');
endfunction

% generate 1s of sine waves
function wave = naive(f,fs,phi,dur)
  w = 2*pi*f/fs;
  wave = zeros(1,fs*dur);
  for i = 1:fs*dur
      wave(i) = sin(phi);
      phi += w;
  endfor
endfunction

function wave = rot(f, fs, phi, dur)
  w = 2*pi*f/fs;
  wave = zeros(1, fs*dur);
  v = [cos(phi); sin(phi)];
  R = [cos(w), -sin(w); sin(w), cos(w)];
  for i = 1:fs*dur
    v = R*v;
    wave(i) = v(2);
  endfor
endfunction

function wave = phasor(f, fs, phi, dur)
  w = 2*pi*f/fs;
  a = 2*cos(w);
  wave = zeros(1, fs*dur);
  phasor = phi;
  wave(1) = sin(phasor);
  phasor += w;
  wave(2) = sin(phasor);
  for i = 3:fs*dur
    wave(i) = a*wave(i-1)-wave(i-2);
  endfor
endfunction

% interpolated table lookup
function wave = itlo(f, fs, phi, dur, table)
  len = length(table);
  wave = zeros(1, fs*dur);
  w = f*len/fs;
  for i = 1:fs*dur
    % octave table indices start at 1
    idx = w*i;
    j = mod(floor(idx), len) + 1;
    k = mod(ceil(idx), len) + 1;
    wave(i) = (table(k) + table(j))/2;
  endfor
endfunction

function table = create_sine_table(len, phi)
  % use specific table length, not the sampling frequency
  w = 2*pi/len;
  table = arrayfun(@sin, (1:len)*w);
endfunction

function table = create_saw_table(len, phi, ord)
  w = 2*pi/len;
  table = zeros(1, len);
  for i = 1:len
    table(i) = 1/2 * sum(
      arrayfun(@(k) sin(k*(w+phi)*i)/k,[1:ord])
    );
  endfor
endfunction

function table = create_triangle_table(len, phi, ord)
  w = 2*pi/len;
  table = zeros(1, len);
  for i = 1:len
    table(i) = 8/(pi^2) * sum(
      arrayfun(@(k) (-1)^k * sin((2*k+1)*(w+phi)*i)/(2*k+1)^2, [0:ord-1])
    );
  endfor
endfunction

function table = create_square_table(len, phi, ord)
  w = 2*pi/len;
  table = zeros(1, len);
  for i = 1:len
    table(i) = 4/pi * sum(
      arrayfun(@(k) sin((2*k-1)*(w+phi)*i)/(2*k-1), [1:ord])
    );
  endfor
endfunction
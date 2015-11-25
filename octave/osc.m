% sampling rate
fs = 8*10^3;
f  = 2;
% table size
ts = 1024;
phi = 0;
dur = 1;
order = 64;

source('osc_functions.m');

naive_wave = naive(f,fs,phi,dur);
rot_wave = rot(f,fs,phi,dur); rot_err = abs(sum(rot_wave - naive_wave));
phasor_wave = phasor(f,fs,phi,dur); phasor_err = abs(sum(phasor_wave - naive_wave));
sine_table = create_sine_table(ts,phi);
itlo_wave  = itlo(f,fs,phi,dur,sine_table); itlo_err = abs(sum(itlo_wave - naive_wave));
saw_table = create_saw_table(ts,phi,order);
tri_table = create_triangle_table(ts,phi,order);
sqr_table = create_square_table(ts,phi,order);
itlo_saw_wave = itlo(f,fs,phi,dur,saw_table);
itlo_tri_wave = itlo(f,fs,phi,dur,tri_table);
itlo_sqr_wave = itlo(f,fs,phi,dur,sqr_table);

subplot(2,3,1);
spectrum_plot(fs, rot_wave, 'rot');

subplot(2,3,2);
spectrum_plot(fs, naive_wave, 'naive');

subplot(2,3,3);
spectrum_plot(fs, phasor_wave, 'phasor');

subplot(2,3,4);
spectrum_plot(fs, itlo_wave, 'itlo');

subplot(2,3,5);
#plot(itlo_saw_wave);
plot(itlo_sqr_wave);

subplot(2,3,6);
#spectrum_plot(fs, itlo_saw_wave, 'itlo_saw');
spectrum_plot(fs, itlo_sqr_wave, 'itlo_sqr');

disp("Rotation Matrix error: ");disp(rot_err);
disp("Phasor error: ");disp(phasor_err);
disp("Interpolated Table-Lookup error: ");disp(itlo_err);
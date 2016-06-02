#!/bin/bash
SOX_OPTIONS="-n spectrogram -x 1920"

for f in $(ls ytterbium*.wav); do
    sox $f $SOX_OPTIONS -o "${f%.wav}.png"
done

set terminal svg enhanced background rgb "#0D1117" size 480 480

set style data histogram
set style histogram cluster gap 1

set ylabel "speed in GB/s (higher is better)"
set auto x
set yrange [0:4]

set style line 12 lc rgb '#1F2430' lt 1 lw 2 dt 22

set grid ytics ls 12

set output "sped.svg"
set title "encoding speed"

plot 'plot.dat' using 2:xtic(1) title col, \
        '' using 3:xtic(1) title col
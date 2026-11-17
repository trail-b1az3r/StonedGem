#!/bin/sh
# Usage: xvfb-run -a -s "-screen 0 1200x800x24" crates/stonedgem-chrome/xvfb-check.sh OUTDIR
# Runs inside xvfb-run: start the browser, screenshot, drive it, screenshot.
OUT=$1
${BROWSER:-target/debug/stonedgem-browser} > $OUT/browser.log 2>&1 &
PID=$!
sleep 10
import -window root $OUT/1-start.png
if command -v xdotool >/dev/null; then
  WID=$(xdotool search --name StonedGem | head -1)
  xdotool windowfocus --sync $WID 2>/dev/null
  xdotool mousemove 600 50 click 1
  sleep 1
  xdotool type --delay 40 "example.com"
  xdotool key Return
  sleep 10
  import -window root $OUT/2-example.png
  xdotool key ctrl+t
  sleep 3
  import -window root $OUT/2b-newtab.png
  xdotool type --delay 40 "en.wikipedia.org/wiki/Servo_(software)"
  xdotool key Return
  sleep 15
  import -window root $OUT/3-wikipedia.png
  xdotool mousemove 1100 500 click 1
  sleep 1
  xdotool click 5 click 5 click 5 click 5 click 5 click 5
  sleep 3
  import -window root $OUT/4-scrolled.png
fi
kill $PID

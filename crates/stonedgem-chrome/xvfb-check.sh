#!/bin/sh
# Usage: xvfb-run -a -s "-screen 0 1200x800x24" crates/stonedgem-chrome/xvfb-check.sh OUTDIR
# Starts the browser on a virtual display, drives it with xdotool and takes
# screenshots: start page, a web page, find in page, the menu, History,
# Bookmarks and Settings.
OUT=$1
BROWSER=${BROWSER:-target/debug/stonedgem-browser}
export XDG_CONFIG_HOME=${XDG_CONFIG_HOME:-$OUT/config} XDG_DATA_HOME=${XDG_DATA_HOME:-$OUT/data}
$BROWSER > $OUT/browser.log 2>&1 &
PID=$!
sleep 8
WID=$(xdotool search --name StonedGem | head -1)
xdotool windowfocus --sync $WID 2>/dev/null
shot() { sleep ${2:-2}; import -window root $OUT/$1.png; }
shot 1-start 1
xdotool mousemove 600 59; sleep 0.3; xdotool click 1; sleep 0.5
xdotool type --delay 30 "en.wikipedia.org/wiki/Servo_(software)"; xdotool key Return
shot 2-page 15
xdotool key ctrl+d
xdotool key ctrl+f; sleep 1
xdotool type --delay 40 "servo"; sleep 2; xdotool key Return Return
shot 3-find 4
xdotool key Escape; xdotool mousemove 1175 59; sleep 0.3; xdotool click 1
shot 4-menu 1
xdotool mousemove 960 176; sleep 0.3; xdotool click 1
shot 5-history 2
xdotool key ctrl+shift+o
shot 6-bookmarks 2
xdotool key ctrl+comma
shot 7-settings 2
xdotool mousemove 250 231; sleep 0.3; xdotool click 1
shot 8-settings-light 2
xdotool mousemove 250 231; sleep 0.3; xdotool click 1
kill $PID

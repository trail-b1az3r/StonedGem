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
WID=$(xdotool search --name "StonedGem|OyNIx" | head -1)
xdotool windowfocus --sync $WID 2>/dev/null
shot() { sleep ${2:-2}; import -window root $OUT/$1.png; }
shot 1-start 1
xdotool mousemove 600 80; sleep 0.3; xdotool click 1; sleep 0.5
xdotool type --delay 30 "en.wikipedia.org/wiki/Servo_(software)"; xdotool key Return
shot 2-page 15
xdotool key ctrl+d
xdotool key ctrl+f; sleep 2
xdotool type --delay 40 "servo"; sleep 2; xdotool key Return Return
shot 3-find 4
xdotool key Escape; sleep 2; xdotool mousemove 1164 80; sleep 0.3; xdotool click 1
shot 4-menu 1
xdotool mousemove 960 291; sleep 0.3; xdotool click 1
shot 5-history 2
xdotool key ctrl+shift+o
shot 6-bookmarks 2
xdotool key ctrl+comma
shot 7-settings 2
xdotool mousemove 932 304; sleep 0.3; xdotool click 1
shot 8-settings-light 2
xdotool mousemove 932 304; sleep 0.3; xdotool click 1
# A page with a link at a known place: right-click it, save it, see Downloads.
xdotool key ctrl+t; sleep 1
xdotool type --delay 10 'data:text/html,<title>Links</title><a href="https://example.com/" style="position:fixed;left:200px;top:200px;font:32px sans-serif">example.com</a>'
xdotool key Return; sleep 3
xdotool mousemove 280 320; sleep 0.5; xdotool click 3
shot 9-context 2
xdotool mousemove 360 424; sleep 0.3; xdotool click 1; sleep 4
xdotool key ctrl+j
shot 10-downloads 2
# Zoom the page.
xdotool mousemove 110 32; sleep 0.3; xdotool click 1; sleep 1
xdotool key ctrl+equal ctrl+equal
shot 11-zoom 4
# Restart: the tabs come back.
sleep 3
kill $PID; sleep 2
$BROWSER >> $OUT/browser.log 2>&1 &
PID=$!
sleep 12
shot 12-restored 1
kill $PID

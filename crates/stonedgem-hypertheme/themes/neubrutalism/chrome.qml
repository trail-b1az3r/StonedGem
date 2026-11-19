// Neubrutalism for OyNIx's Qt chrome (Theme Studio).
import QtQuick

QtObject {
    property bool dark: false
    property color background: dark ? "#18181b" : "#fff4dc"
    property color surface: dark ? "#27272a" : "#ffffff"
    property color surfaceAlt: dark ? "#3f3f46" : "#ffeaa1"
    property color text: dark ? "#fafafa" : "#111111"
    property color muted: dark ? "#d4d4d8" : "#545454"
    property color accent: dark ? "#ff5c8a" : "#ff5c8a"
    property color onAccent: dark ? "#111111" : "#111111"
    property color border: dark ? "#fafafa" : "#111111"
    property color danger: dark ? "#e63946" : "#e63946"
    property color shadow: dark ? "#fafafa" : "#111111"
    property real radius: 6
    property real borderWidth: 3
    property real shadowX: 4
    property real shadowY: 4
    property real shadowBlur: 0
    property string fontFamily: "Archivo Black, Arial Black, Helvetica, sans-serif"
}

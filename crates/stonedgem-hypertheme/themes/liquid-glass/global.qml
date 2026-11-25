import QtQuick

// The rim of the glass: a hairline of light around the whole window,
// brighter along the top, as light catches a pane's edge.
Item {
    property var ui
    property var window
    Rectangle {
        anchors.fill: parent
        color: "transparent"
        radius: 10
        border.width: 1
        border.color: Qt.rgba(1, 1, 1, 0.28)
    }
    Rectangle {
        anchors { left: parent.left; right: parent.right; top: parent.top; leftMargin: 18; rightMargin: 18; topMargin: 1 }
        height: 1
        gradient: Gradient {
            orientation: Gradient.Horizontal
            GradientStop { position: 0.0; color: Qt.rgba(1, 1, 1, 0) }
            GradientStop { position: 0.5; color: Qt.rgba(1, 1, 1, 0.75) }
            GradientStop { position: 1.0; color: Qt.rgba(1, 1, 1, 0) }
        }
    }
}

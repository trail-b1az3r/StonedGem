import QtQuick

// Liquid Glass animations. OyNIx calls these functions as things happen:
// a page appears and a glint of light runs across the glass; a tab opens
// and a soft ripple spreads from it. The layer takes no input.
Item {
    id: root
    property var ui
    property var window
    property var theme
    property var anim
    clip: true

    function pageShown(item) { glint.restart() }
    function tabOpened(item) {
        if (!item) return
        var p = item.mapToItem(root, item.width / 2, item.height / 2)
        ripple.x = p.x - ripple.width / 2
        ripple.y = p.y - ripple.height / 2
        rippleAnim.restart()
    }
    function menuOpened(item) {}
    function navigated(url) {}

    Rectangle {
        id: sweep
        width: 180
        height: root.height * 1.6
        y: -root.height * 0.3
        x: -400
        rotation: 18
        opacity: 0
        gradient: Gradient {
            orientation: Gradient.Horizontal
            GradientStop { position: 0.0; color: Qt.rgba(1, 1, 1, 0) }
            GradientStop { position: 0.5; color: Qt.rgba(1, 1, 1, 0.22) }
            GradientStop { position: 1.0; color: Qt.rgba(1, 1, 1, 0) }
        }
    }
    ParallelAnimation {
        id: glint
        NumberAnimation { target: sweep; property: "x"; from: -400; to: root.width + 200; duration: 760; easing.type: Easing.OutCubic }
        SequentialAnimation {
            NumberAnimation { target: sweep; property: "opacity"; from: 0; to: 1; duration: 160 }
            NumberAnimation { target: sweep; property: "opacity"; to: 0; duration: 600 }
        }
    }

    Rectangle {
        id: ripple
        width: 48; height: 48; radius: 24
        color: Qt.rgba(1, 1, 1, 0.10)
        border.width: 1
        border.color: Qt.rgba(1, 1, 1, 0.7)
        opacity: 0
    }
    ParallelAnimation {
        id: rippleAnim
        NumberAnimation { target: ripple; property: "scale"; from: 0.3; to: 3.2; duration: 520; easing.type: Easing.OutCubic }
        NumberAnimation { target: ripple; property: "opacity"; from: 0.8; to: 0; duration: 520 }
    }
}

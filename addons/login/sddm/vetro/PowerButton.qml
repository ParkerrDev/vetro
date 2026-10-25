import QtQuick

// Round outline button that needs a second click to act, so a stray click on
// the login screen can't reboot the machine.
Item {
  id: root

  property string icon: "power_settings_new"
  property string label: ""
  property string confirmLabel: "Click again"
  property bool enabled: true
  property bool armed: false

  signal activated

  implicitWidth: 44
  implicitHeight: 44

  Timer {
    id: disarm

    interval: 3000
    onTriggered: root.armed = false
  }

  Rectangle {
    anchors.fill: parent
    radius: width / 2
    color: root.armed ? Style.text : Qt.rgba(1, 1, 1, mouse.containsMouse ? 0.16 : 0.06)
    border.width: 1
    border.color: root.armed ? Style.text : Qt.rgba(1, 1, 1, mouse.containsMouse ? 0.8 : 0.4)

    Behavior on color {
      ColorAnimation {
        duration: 150
      }
    }

    Text {
      anchors.centerIn: parent
      text: root.icon
      color: root.armed ? "black" : Style.text
      font.family: "Material Symbols Rounded"
      font.pixelSize: 22
    }
  }

  Text {
    anchors.right: parent.right
    anchors.bottom: parent.top
    anchors.bottomMargin: 10
    visible: mouse.containsMouse || root.armed
    text: root.armed ? root.confirmLabel : root.label
    color: Style.text
    font.family: Style.family
    font.pixelSize: 11
    font.weight: Font.Bold
    font.capitalization: Font.AllUppercase
    font.letterSpacing: 11 * 0.28
    style: Text.Raised
    styleColor: Qt.rgba(0, 0, 0, 0.5)
  }

  MouseArea {
    id: mouse

    anchors.fill: parent
    hoverEnabled: true
    cursorShape: Qt.PointingHandCursor
    enabled: root.enabled
    onClicked: {
      if (root.armed) {
        root.armed = false;
        root.activated();
      } else {
        root.armed = true;
        disarm.restart();
      }
    }
  }
}

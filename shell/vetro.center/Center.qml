pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Hyprland
import Quickshell.Wayland
import "../vetro.lib"

// Vetro's notification centre: a pane of glass on the right edge with every
// notification not yet dismissed, Do Not Disturb, and clear all.
//
//   omarchy-shell shell toggle vetro.center
//
// Omarchy's "Open notification history" key (Super+Shift+Alt+comma) and the
// bell in the bar (vetro.center-button) both do that.
Item {
  id: root

  // Injected by Omarchy's shell.
  property string omarchyPath: Quickshell.env("OMARCHY_PATH")
  property var shell: null
  property var manifest: null
  property var service: null

  property bool opened: false
  function open(payloadJson) {
    opened = true;
  }
  function close() {
    opened = false;
  }

  Component.onCompleted: Notifications.centerAvailable = true
  Component.onDestruction: Notifications.centerAvailable = false

  readonly property real gap: Settings.sideGap
  readonly property int count: Notifications.all.length

  // It comes in from the right and goes back there by Hyprland's own slide
  // (hypr/vetro.lua, the layer rule for vetro:center), not by moving the pane
  // inside its surface: moved inside, its outline changed on every frame and
  // the glass engine built the pane again for each one it could, so the glass
  // trailed the pane. Slid whole, the picture stays still and the pane with
  // it.
  PanelWindow {
    id: window

    visible: root.opened
    color: "transparent"
    // Its own namespace, for its slide; hypr/vetro.lua gives it the glass
    WlrLayershell.namespace: "vetro:center"
    WlrLayershell.layer: WlrLayer.Top
    WlrLayershell.keyboardFocus: root.opened ? WlrKeyboardFocus.OnDemand : WlrKeyboardFocus.None
    exclusiveZone: 0
    implicitWidth: 390 + root.gap

    anchors {
      top: true
      right: true
      bottom: true
    }

    // A click anywhere else closes it
    HyprlandFocusGrab {
      active: root.opened
      windows: [window]
      onCleared: root.close()
    }

    mask: Region {
      item: root.opened ? pane : null
    }

    Rectangle {
      id: pane

      width: 390
      anchors.top: parent.top
      anchors.bottom: parent.bottom
      anchors.bottomMargin: root.gap
      radius: Look.radius
      color: Look.film
      border.width: Look.frameWidth
      border.color: Look.frame
      focus: true
      Keys.onEscapePressed: root.close()

      ColumnLayout {
        anchors.fill: parent
        anchors.margins: 14
        spacing: 12

        RowLayout {
          Layout.fillWidth: true
          Layout.leftMargin: 4
          spacing: 8

          Label {
            Layout.fillWidth: true
            text: "Notifications"
            font.weight: Font.Bold
            font.pointSize: Look.huge
          }

          Round {
            icon: Notifications.dnd ? "do_not_disturb_on" : "notifications"
            lit: Notifications.dnd
            onPressed: Notifications.setDnd(!Notifications.dnd)
          }

          Round {
            icon: "clear_all"
            enabled: root.count > 0
            onPressed: Notifications.clear()
          }
        }

        Item {
          Layout.fillWidth: true
          Layout.fillHeight: true

          ListView {
            id: list

            anchors.fill: parent
            clip: true
            spacing: 8
            boundsBehavior: Flickable.StopAtBounds
            // How far a glide carries: it slows at this many px/s each second
            flickDeceleration: 2200
            maximumFlickVelocity: 6000

            KineticWheel {
              view: list
            }

            model: ScriptModel {
              values: Notifications.all
            }

            delegate: Toast {
              required property var modelData
              width: ListView.view.width
              note: modelData
              inCenter: true
            }

            // A new one comes down from the top and the rest make room; one
            // dismissed shrinks away and the rest close up.
            add: Transition {
              NumberAnimation {
                property: "opacity"
                from: 0
                to: 1
                duration: Look.moderate
                easing.type: Easing.BezierSpline
                easing.bezierCurve: Look.settle
              }
              NumberAnimation {
                property: "scale"
                from: 0.92
                to: 1
                duration: Look.moderate
                easing.type: Easing.BezierSpline
                easing.bezierCurve: Look.spring
              }
            }
            remove: Transition {
              NumberAnimation {
                property: "opacity"
                to: 0
                duration: Look.quick
                easing.type: Easing.BezierSpline
                easing.bezierCurve: Look.leave
              }
              NumberAnimation {
                property: "scale"
                to: 0.92
                duration: Look.quick
                easing.type: Easing.BezierSpline
                easing.bezierCurve: Look.leave
              }
            }
            displaced: Transition {
              NumberAnimation {
                property: "y"
                duration: Look.moderate
                easing.type: Easing.BezierSpline
                easing.bezierCurve: Look.settle
              }
              NumberAnimation {
                property: "opacity"
                to: 1
                duration: Look.quick
              }
              NumberAnimation {
                property: "scale"
                to: 1
                duration: Look.quick
              }
            }
          }

          ColumnLayout {
            anchors.centerIn: parent
            visible: root.count === 0
            spacing: 6

            Icon {
              Layout.alignment: Qt.AlignHCenter
              text: Notifications.dnd ? "do_not_disturb_on" : "notifications"
              size: 34
              color: Look.faint
            }
            Label {
              Layout.alignment: Qt.AlignHCenter
              text: Notifications.dnd ? "Do Not Disturb" : "No notifications"
              color: Look.quiet
            }
          }
        }
      }
    }
  }

  // A round button in the header.
  component Round: Rectangle {
    id: button

    property string icon: ""
    property bool lit: false
    signal pressed

    implicitWidth: 32
    implicitHeight: 32
    radius: 16
    opacity: enabled ? 1 : 0.35
    color: lit ? Look.wellLit : (pointing.hovered ? Look.wellHover : Look.well)
    Behavior on color {
      ColorAnimation {
        duration: Look.quick
      }
    }

    Icon {
      anchors.centerIn: parent
      text: button.icon
      size: 18
      filled: button.lit
    }
    HoverHandler {
      id: pointing
    }
    TapHandler {
      gesturePolicy: TapHandler.ReleaseWithinBounds
      onTapped: button.pressed()
    }
  }
}

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Widgets

// One notification as a card: its picture or app icon, the summary with how
// long ago it came, the body, and its buttons. A click does what it offers
// (its default action, or what it asked to run); the cross, shown under the
// pointer, dismisses it.
//
// As a toast the card is a pane of its own, the film over the glass. In the
// centre it lies on the centre's pane, as a well, and is swiped to the right
// to dismiss it.
Item {
  id: card

  required property var note
  // In the centre rather than up as a toast.
  property bool inCenter: false

  implicitWidth: 360
  implicitHeight: content.implicitHeight + 2 * pad

  readonly property real pad: 14
  readonly property bool hovered: hover.hovered

  function activate() {
    const acts = note.actions.length > 0;
    const runs = note.exec.length > 0;
    Notifications.launch(note);
    if (acts)
      Notifications.invoke(note.id);
    else if (runs)
      Notifications.dismiss(note.id);
    else if (!inCenter)
      Notifications.hide(note.id);
  }

  // Where its picture comes from: one sent with it, its image, its icon, or
  // its app's icon.
  function source(name) {
    const text = String(name || "");
    if (text === "")
      return "";
    if (text.startsWith("/"))
      return "file://" + text;
    if (text.startsWith("file://"))
      return text;
    return Quickshell.iconPath(text, true);
  }
  readonly property string picture: {
    const named = note.desktopEntry || note.app;
    const app = named ? DesktopEntries.heuristicLookup(named) : null;
    const names = [note.picture, note.image, note.icon, app ? app.icon : ""];
    for (const name of names) {
      const found = source(name);
      if (found !== "")
        return found;
    }
    return "";
  }

  HoverHandler {
    id: hover
  }

  Rectangle {
    id: face

    width: parent.width
    height: parent.height
    radius: Look.radius
    color: card.inCenter ? (card.hovered ? Look.wellHover : Look.well) : Look.film
    border.width: card.note.critical ? 1.5 : Look.frameWidth
    border.color: card.note.critical ? Look.urgent : Look.frame
    // Pressed, it goes a shade deeper, as a button does.
    Rectangle {
      anchors.fill: parent
      radius: parent.radius
      color: Look.ink
      opacity: press.pressed ? 0.08 : (card.hovered && !card.inCenter ? 0.04 : 0)
      Behavior on opacity {
        NumberAnimation {
          duration: Look.quick
          easing.type: Easing.BezierSpline
          easing.bezierCurve: Look.settle
        }
      }
    }
    Behavior on color {
      ColorAnimation {
        duration: Look.quick
      }
    }

    // A swipe to the right in the centre dismisses it.
    x: 0
    opacity: 1 - Math.max(0, x) / Math.max(1, width)
    DragHandler {
      id: swipe
      enabled: card.inCenter
      target: face
      xAxis.minimum: 0
      yAxis.enabled: false
      onActiveChanged: {
        if (active)
          return;
        if (face.x > face.width * 0.35)
          away.start();
        else
          back.start();
      }
    }
    NumberAnimation {
      id: back
      target: face
      property: "x"
      to: 0
      duration: Look.moderate
      easing.type: Easing.BezierSpline
      easing.bezierCurve: Look.spring
    }
    SequentialAnimation {
      id: away
      NumberAnimation {
        target: face
        property: "x"
        to: face.width
        duration: Look.quick
        easing.type: Easing.BezierSpline
        easing.bezierCurve: Look.leave
      }
      ScriptAction {
        script: Notifications.dismiss(card.note.id)
      }
    }

    TapHandler {
      id: press
      enabled: card.note.clickable || !card.inCenter
      gesturePolicy: TapHandler.ReleaseWithinBounds
      onTapped: card.activate()
    }

    RowLayout {
      id: content

      anchors.fill: parent
      anchors.margins: card.pad
      spacing: 12

      // The app's icon, a rounded square as on a phone, or a glyph when it
      // has none.
      ClippingRectangle {
        Layout.alignment: Qt.AlignTop
        implicitWidth: 40
        implicitHeight: 40
        radius: 11
        color: card.picture === "" ? Look.well : "transparent"

        IconImage {
          anchors.fill: parent
          visible: card.picture !== ""
          source: card.picture
          asynchronous: true
        }
        Icon {
          anchors.centerIn: parent
          visible: card.picture === ""
          text: "notifications"
          size: 22
          filled: true
        }
      }

      ColumnLayout {
        Layout.fillWidth: true
        Layout.alignment: Qt.AlignTop
        spacing: 3

        RowLayout {
          Layout.fillWidth: true
          spacing: 8

          Label {
            Layout.fillWidth: true
            text: card.note.summary || card.note.app
            font.weight: Font.Bold
            font.pointSize: Look.large
          }

          // How long ago, and in its place under the pointer, the cross.
          Item {
            implicitWidth: Math.max(ago.implicitWidth, 22)
            implicitHeight: 22

            Label {
              id: ago
              anchors.right: parent.right
              anchors.verticalCenter: parent.verticalCenter
              visible: !card.hovered
              text: Notifications.ago(card.note.time)
              color: Look.quiet
              font.pointSize: Look.small
            }
            Rectangle {
              anchors.right: parent.right
              width: 22
              height: 22
              radius: 11
              visible: card.hovered
              color: closing.hovered ? Look.wellLit : Look.wellHover

              Icon {
                anchors.centerIn: parent
                text: "close"
                size: 14
              }
              HoverHandler {
                id: closing
              }
              TapHandler {
                gesturePolicy: TapHandler.ReleaseWithinBounds
                onTapped: Notifications.dismiss(card.note.id)
              }
            }
          }
        }

        Label {
          Layout.fillWidth: true
          visible: text !== ""
          text: card.note.body
          textFormat: Text.StyledText
          wrapMode: Text.Wrap
          maximumLineCount: card.inCenter ? 6 : 4
          color: Look.text
          opacity: 0.85
        }

        // Its buttons, as pills.
        Flow {
          Layout.fillWidth: true
          Layout.topMargin: 6
          visible: card.note.buttons.length > 0
          spacing: 6

          Repeater {
            model: card.note.buttons

            Rectangle {
              id: pill

              required property var modelData

              width: Math.min(word.implicitWidth + 24, card.width - 2 * card.pad)
              height: 28
              radius: 14
              color: pressing.pressed ? Look.wellLit : (pointing.hovered ? Look.wellHover : Look.well)

              Label {
                id: word
                anchors.centerIn: parent
                width: Math.min(implicitWidth, pill.width - 24)
                text: pill.modelData.label
                font.weight: Font.DemiBold
              }
              HoverHandler {
                id: pointing
              }
              TapHandler {
                id: pressing
                gesturePolicy: TapHandler.ReleaseWithinBounds
                onTapped: Notifications.invoke(card.note.id, pill.modelData.key)
              }
            }
          }
        }
      }
    }
  }
}

import QtQuick
import "../../vetro.lib" as V

// The next-launch card as an item, for something else to place: a desktop
// widget layer that lets it be moved and resized. It is the same card as the
// one in the corner, with its own feed and the same settings, drawn without
// its glass (whoever places it draws that) and scaled to fit the room it is
// given.
Item {
  id: root

  LaunchData {
    id: feed
  }

  LaunchCard {
    id: card

    visible: !!feed.next
    width: implicitWidth
    height: implicitHeight
    anchors.centerIn: parent
    scale: Math.min(root.width / Math.max(1, implicitWidth), root.height / Math.max(1, implicitHeight), 1.4)
    launchData: feed
    glassColor: "transparent"
    edgeColor: "transparent"
    radius: V.Settings.radius
    basePointSize: V.Settings.textSize
    use12h: V.Settings.twelveHour
    rows: feed.overlay.upcoming_rows !== undefined ? feed.overlay.upcoming_rows : 3
  }

  Text {
    visible: !feed.next
    anchors.centerIn: parent
    text: "No launch ahead"
    color: Style.dim
    font.family: Style.family
    font.pointSize: 8
    font.capitalization: Font.AllUppercase
    font.letterSpacing: 8 * Style.capsTracking
  }
}

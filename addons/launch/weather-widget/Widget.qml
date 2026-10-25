import QtQuick
import Quickshell

// The weather card as a Homescreen widget: the same card Vetro puts in a
// corner of the desktop, placed on Homescreen's grid so that it can be moved
// and resized like any other widget. While it is placed, the one in the
// corner stands down (Placed.qml in the cards' plugin).
//
// The card is Vetro's own, loaded from where the cards' plugin is installed,
// so there is one card and not a copy of it to keep alike.
Item {
  id: root

  // Given by the layer.
  property int cols: 3
  property int rows: 3
  property var settings: ({})

  // The card brings its own padding and shades its own type.
  readonly property bool wholeCard: true
  readonly property bool ownShadow: true

  Loader {
    id: card

    anchors.fill: parent
    source: "file://" + Quickshell.env("HOME") + "/.config/omarchy/plugins/vetro.cards/launch/HostedWeather.qml"
  }

  Text {
    visible: card.status === Loader.Error || card.status === Loader.Null
    anchors.centerIn: parent
    width: parent.width - 24
    textFormat: Text.PlainText
    text: "Vetro's weather card is not installed"
    color: "white"
    opacity: 0.7
    horizontalAlignment: Text.AlignHCenter
    wrapMode: Text.WordWrap
  }
}

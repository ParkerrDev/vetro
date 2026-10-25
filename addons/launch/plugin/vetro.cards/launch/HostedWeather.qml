import QtQuick
import "../../vetro.lib" as V

// The weather card as an item, for something else to place: a desktop widget
// layer that lets it be moved and resized. The same card as the one in the
// corner, drawn without its glass and scaled to fit the room it is given.
Item {
  id: root

  LaunchData {
    id: feed
  }

  WeatherData {
    id: forecast
  }

  WeatherCard {
    id: card

    visible: forecast.ready
    width: implicitWidth
    height: implicitHeight
    anchors.centerIn: parent
    scale: Math.min(root.width / Math.max(1, implicitWidth), root.height / Math.max(1, implicitHeight), 1.4)
    weather: forecast
    glassColor: "transparent"
    edgeColor: "transparent"
    hours: feed.weatherCard.hours !== undefined ? feed.weatherCard.hours : 6
    radius: V.Settings.radius
    basePointSize: V.Settings.textSize
    use12h: V.Settings.twelveHour
  }

  Text {
    visible: !forecast.ready
    anchors.centerIn: parent
    text: "No forecast yet"
    color: Style.dim
    font.family: Style.family
    font.pointSize: 8
    font.capitalization: Font.AllUppercase
    font.letterSpacing: 8 * Style.capsTracking
  }
}

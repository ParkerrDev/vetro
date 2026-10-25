import QtQuick
import QtQuick.Effects
import QtQuick.Layouts

// The weather card, in the launch card's style: white type on clear glass.
// Presentation only; `weather` is a WeatherData (WeatherData.qml).
Item {
  id: root

  required property var weather
  property int hours: 6
  property real radius: 8
  property real basePointSize: 10
  property bool use12h: false
  property color glassColor: Style.glass
  // The line round the glass. Whoever draws the glass itself clears both.
  property color edgeColor: Style.glassEdge

  readonly property var now: weather.current
  readonly property var today: weather.today
  readonly property real pad: 18

  implicitWidth: 318
  implicitHeight: body.implicitHeight + 2 * pad

  function pt(k) {
    return basePointSize * k;
  }

  function deg(v) {
    return v === null || v === undefined ? "--" : v + "°";
  }

  // Short enough for six columns: "15", or "3PM".
  function hourLabel(epoch, first) {
    if (first)
      return "Now";
    return Qt.formatDateTime(new Date(epoch * 1000), use12h ? "hAP" : "HH");
  }

  function clock(epoch) {
    return Qt.formatDateTime(new Date(epoch * 1000), use12h ? "h:mm AP" : "HH:mm");
  }

  // Sunrise or sunset, whichever comes next today.
  readonly property string sunLine: {
    const t = Date.now() / 1000;
    if (!today || !today.sunset)
      return "";
    if (t < today.sunrise)
      return "Sunrise " + clock(today.sunrise);
    if (t < today.sunset)
      return "Sunset " + clock(today.sunset);
    const tomorrow = weather.daily.length > 1 ? weather.daily[1] : null;
    return tomorrow ? "Sunrise " + clock(tomorrow.sunrise) : "";
  }

  component Caps: Text {
    property real size: root.pt(0.78)

    color: Style.dim
    font.family: Style.family
    font.pointSize: size
    font.capitalization: Font.AllUppercase
    font.letterSpacing: size * Style.capsTracking
    elide: Text.ElideRight
  }

  // Outlined, like the rest of the card's thin lines. (The filled variant of
  // this variable font renders with holes where its shapes overlap.)
  component Icon: Text {
    property real size: 18

    color: Style.text
    font.family: "Material Symbols Rounded"
    font.pixelSize: size
  }

  // Glass
  Rectangle {
    anchors.fill: parent
    radius: root.radius
    color: root.glassColor
    border.width: 1
    border.color: root.edgeColor
  }


  ColumnLayout {
    id: body

    spacing: 0
    layer.enabled: true
    layer.effect: MultiEffect {
      shadowEnabled: true
      shadowColor: "black"
      shadowOpacity: 0.45
      shadowBlur: 0.35
      shadowVerticalOffset: 1
    }

    anchors {
      left: parent.left
      right: parent.right
      top: parent.top
      margins: root.pad
    }

    // Header: what this is, and where.
    RowLayout {
      Layout.fillWidth: true
      spacing: 8

      Caps {
        text: "Weather"
        font.weight: Font.Bold
      }

      Caps {
        Layout.fillWidth: true
        horizontalAlignment: Text.AlignRight
        text: root.weather.location
        color: Style.faint
      }
    }

    // Now: icon, temperature, conditions.
    RowLayout {
      Layout.fillWidth: true
      Layout.topMargin: 10
      spacing: 12

      Icon {
        Layout.alignment: Qt.AlignVCenter
        text: root.now.icon || "question_mark"
        size: 42
      }

      Text {
        Layout.alignment: Qt.AlignVCenter
        text: root.deg(root.now.temp)
        color: Style.text
        font.family: Style.wide
        font.pointSize: root.pt(2.9)
      }

      ColumnLayout {
        Layout.alignment: Qt.AlignVCenter
        Layout.fillWidth: true
        spacing: 3

        Caps {
          Layout.fillWidth: true
          text: root.now.text || ""
          color: Style.text
          size: root.pt(0.86)
          font.weight: Font.Bold
        }

        Caps {
          Layout.fillWidth: true
          text: "Feels " + root.deg(root.now.feels)
          size: root.pt(0.72)
        }
      }
    }

    Caps {
      Layout.fillWidth: true
      Layout.topMargin: 10
      text: ["H " + root.deg(root.today.high) + "  L " + root.deg(root.today.low), "Wind " + (root.now.wind ?? "--") + " " + root.weather.windUnit, (root.now.humidity ?? "--") + "% humidity"].join("  ·  ")
      size: root.pt(0.7)
    }

    Rectangle {
      Layout.fillWidth: true
      Layout.topMargin: 12
      Layout.bottomMargin: 10
      visible: hourStrip.count > 0
      implicitHeight: 1
      color: Style.hairline
    }

    // The next few hours, in equal columns.
    Row {
      id: hourRow

      Layout.fillWidth: true

      Repeater {
        id: hourStrip

        model: root.weather.hourly.slice(0, root.hours)

        Column {
          required property var modelData
          required property int index

          width: hourRow.width / Math.max(1, hourStrip.count)
          spacing: 5

          Caps {
            anchors.horizontalCenter: parent.horizontalCenter
            text: root.hourLabel(modelData.time, index === 0)
            color: index === 0 ? Style.text : Style.faint
            size: root.pt(0.62)
            font.weight: index === 0 ? Font.Bold : Font.Normal
          }

          Icon {
            anchors.horizontalCenter: parent.horizontalCenter
            text: modelData.icon
            size: 20
          }

          Text {
            anchors.horizontalCenter: parent.horizontalCenter
            text: root.deg(modelData.temp)
            color: Style.text
            font.family: Style.family
            font.pointSize: root.pt(0.9)
          }

          Caps {
            anchors.horizontalCenter: parent.horizontalCenter
            // Rain chance, only when there is some.
            text: modelData.precip > 0 ? modelData.precip + "%" : " "
            color: Style.faint
            size: root.pt(0.56)
          }
        }
      }
    }

    Caps {
      Layout.fillWidth: true
      Layout.topMargin: 6
      visible: text !== ""
      text: root.sunLine
      color: Style.faint
      size: root.pt(0.66)
    }
  }
}

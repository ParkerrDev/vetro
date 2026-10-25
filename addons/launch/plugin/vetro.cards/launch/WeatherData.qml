import QtQuick
import Quickshell
import Quickshell.Io

// The forecast vetro-launchd keeps for the weather card, from
// ~/.cache/vetro/weather.json: where it is for, the reading now, today's
// highs and lows, the hours ahead and the days. The [weather] section of
// launches.toml says where (`location`, empty for where this machine is)
// and in what (`units`).
Scope {
  id: root

  property var file: ({})

  readonly property bool ready: file.ready === true
  readonly property string location: file.location || ""
  readonly property string unit: file.unit || "°F"
  readonly property string windUnit: file.windUnit || "mph"
  readonly property string error: file.error || ""
  // {temp, feels, code, isDay, wind, humidity, text, icon}
  readonly property var current: file.current || ({})
  // {high, low, sunrise, sunset, precip, icon}
  readonly property var today: file.today || ({})
  // [{time, temp, precip, icon, isDay}], from the current hour on
  readonly property var hourly: file.hourly || []
  // [{time, high, low, precip, sunrise, sunset, icon}], from today on
  readonly property var daily: file.daily || []

  FileView {
    id: forecast

    path: Quickshell.env("HOME") + "/.cache/vetro/weather.json"
    watchChanges: true
    printErrors: false
    onFileChanged: reload()
    onLoaded: {
      try {
        root.file = JSON.parse(forecast.text());
      } catch (e) {}
    }
  }

  // The daemon replaces the file with a rename, which a file watch can lose
  // track of. A slow re-read covers that.
  Timer {
    interval: 60000
    repeat: true
    running: true
    onTriggered: forecast.reload()
  }
}

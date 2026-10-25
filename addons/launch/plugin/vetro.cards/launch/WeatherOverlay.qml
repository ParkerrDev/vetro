import QtQuick
import Quickshell
import Quickshell.Wayland
import "../../vetro.lib" as V

// The weather card on the desktop, opposite the launch card: above the video
// wallpaper, below windows, click-through. It fades away while a launch
// webcast plays and comes back when it ends. Settings are the [weather]
// section of ~/.config/vetro/launches.toml, via the daemon's state file.
Scope {
  id: root

  required property var launchData
  // Drawn somewhere else, as a widget that can be moved: not here as well.
  property bool standDown: false

  readonly property var cfg: launchData.weatherCard
  readonly property bool enabled: cfg.enabled !== false
  readonly property string corner: cfg.corner || "top-right"
  readonly property var outputs: cfg.outputs || ["*"]
  readonly property var excluded: cfg.exclude_outputs || ["HEADLESS-*"]
  readonly property bool hidden: launchData.live && cfg.hide_when_live !== false

  // 1 shown, 0 gone. Animated, and the windows unmap once it reaches 0.
  property real reveal: hidden ? 0 : 1

  Behavior on reveal {
    NumberAnimation {
      duration: V.Look.ms(550)
      easing.type: Easing.InOutCubic
    }
  }

  function globToRegExp(glob) {
    const escaped = glob.replace(/[.+^${}()|[\]\\]/g, "\\$&").replace(/\*/g, ".*").replace(/\?/g, ".");
    return new RegExp("^" + escaped + "$");
  }

  function wanted(name) {
    return root.outputs.some(p => root.globToRegExp(p).test(name)) && !root.excluded.some(p => root.globToRegExp(p).test(name));
  }

  WeatherData {
    id: forecast
  }

  Variants {
    model: root.enabled && !root.standDown ? Quickshell.screens.filter(s => root.wanted(s.name)) : []

    PanelWindow {
      required property ShellScreen modelData

      screen: modelData
      visible: forecast.ready && root.reveal > 0.005
      color: "transparent"
      WlrLayershell.namespace: "vetro:weather"
      WlrLayershell.layer: WlrLayer.Bottom
      exclusionMode: ExclusionMode.Normal
      mask: Region {}
      implicitWidth: card.implicitWidth
      implicitHeight: card.implicitHeight

      anchors {
        top: root.corner.startsWith("top")
        bottom: root.corner.startsWith("bottom")
        left: root.corner.endsWith("left")
        right: root.corner.endsWith("right")
      }

      margins {
        top: 16
        bottom: 16
        left: V.Settings.sideGap + 11
        right: V.Settings.sideGap + 11
      }

      WeatherCard {
        id: card

        anchors.fill: parent
        weather: forecast
        hours: root.cfg.hours !== undefined ? root.cfg.hours : 6
        radius: V.Settings.radius
        basePointSize: V.Settings.textSize
        use12h: V.Settings.twelveHour
        opacity: root.reveal

        // Lift away a little as it fades.
        transform: Translate {
          y: (1 - root.reveal) * (root.corner.startsWith("top") ? -18 : 18)
        }
      }
    }
  }
}

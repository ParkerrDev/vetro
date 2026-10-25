import QtQuick
import Quickshell
import Quickshell.Wayland
import "../../vetro.lib" as V

// The next-launch card on the desktop: above the video wallpaper, below
// windows, and click-through. Placement and outputs come from the [overlay]
// section of ~/.config/vetro/launches.toml via the daemon's state file.
// While a launch webcast plays it shrinks toward its corner, so the video
// has the screen.
Scope {
  id: root

  required property var launchData
  // Drawn somewhere else, as a widget that can be moved: not here as well.
  property bool standDown: false

  readonly property var overlay: launchData.overlay
  readonly property string corner: overlay.corner || "top-left"
  readonly property var outputs: overlay.outputs || ["*"]
  readonly property var excluded: overlay.exclude_outputs || ["HEADLESS-*"]
  readonly property real liveScale: overlay.live_scale !== undefined ? overlay.live_scale : 0.86

  function globToRegExp(glob) {
    const escaped = glob.replace(/[.+^${}()|[\]\\]/g, "\\$&").replace(/\*/g, ".*").replace(/\?/g, ".");
    return new RegExp("^" + escaped + "$");
  }

  function wanted(name) {
    return root.outputs.some(p => root.globToRegExp(p).test(name)) && !root.excluded.some(p => root.globToRegExp(p).test(name));
  }

  Variants {
    model: root.overlay.enabled === false || root.standDown ? [] : Quickshell.screens.filter(s => root.wanted(s.name))

    PanelWindow {
      required property ShellScreen modelData

      screen: modelData
      visible: !!root.launchData.next
      color: "transparent"
      WlrLayershell.namespace: "vetro:launch"
      WlrLayershell.layer: WlrLayer.Bottom
      // Stay clear of the bar, and don't reserve space of our own.
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

      LaunchCard {
        id: card

        anchors.fill: parent
        launchData: root.launchData
        radius: V.Settings.radius
        basePointSize: V.Settings.textSize
        use12h: V.Settings.twelveHour
        rows: root.overlay.upcoming_rows !== undefined ? root.overlay.upcoming_rows : 3

        transformOrigin: root.corner === "top-right" ? Item.TopRight : root.corner === "bottom-left" ? Item.BottomLeft : root.corner === "bottom-right" ? Item.BottomRight : Item.TopLeft
        scale: root.launchData.live ? root.liveScale : 1

        Behavior on scale {
          NumberAnimation {
            duration: V.Look.ms(600)
            easing.type: Easing.InOutCubic
          }
        }
      }
    }
  }
}

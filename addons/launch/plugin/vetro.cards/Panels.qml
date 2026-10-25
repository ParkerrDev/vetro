import QtQuick
import Quickshell
import "launch" as LO

// The launch addon's always-on surfaces: the next-launch card and the weather
// card over the wallpaper. A keepLoaded panel, so it is up whenever the plugin
// is enabled. It sits next to Omarchy's own bar and needs nothing of Vetro's
// but its settings (shell/vetro.lib).
Item {
  id: root

  // Injected by Omarchy's shell.
  property string omarchyPath: Quickshell.env("OMARCHY_PATH")
  property var shell: null
  property var manifest: null
  property var service: null

  // Always open; summon/hide don't apply.
  readonly property bool opened: true
  function open(payloadJson) {}
  function close() {}

  LO.LaunchData {
    id: feed
  }

  // Where a desktop widget layer has placed one of these cards as a widget,
  // the one in the corner stands down.
  LO.Placed {
    id: placed
  }

  LO.LaunchOverlay {
    launchData: feed
    standDown: placed.ids.indexOf("launches") >= 0
  }

  LO.WeatherOverlay {
    launchData: feed
    standDown: placed.ids.indexOf("forecast") >= 0
  }
}

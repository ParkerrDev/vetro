pragma Singleton

import QtQuick
import Quickshell
import qs.Commons as Omarchy

// The launch look, shared by the desktop cards and the login screen: pure
// white type and hairlines on black glass that Hyprland blurs, set in
// Omarchy's own font like the rest of the desktop. Black, white and
// transparency only: no tints, no gradients, no accent colors.
Singleton {
  id: root

  // Omarchy's font (the `monospace` that `omarchy font set` points at,
  // JetBrainsMono Nerd Font as it ships), for the type and, as Nerd Font
  // glyphs, for the icons. `wide` is what the few big numbers are set in.
  readonly property string family: Omarchy.Style.font.family
  readonly property string wide: family

  // The weather's icons, by the names vetro-launchd gives them (Material's),
  // as the glyphs Omarchy's own weather panel draws for the same weather.
  readonly property var weatherGlyphs: ({
    "sunny": "\u{e30d}", "clear_night": "\u{e32b}",
    "partly_cloudy_day": "\u{e302}", "partly_cloudy_night": "\u{e32e}",
    "cloud": "\u{e33d}", "foggy": "\u{e313}",
    "rainy_light": "\u{e318}", "rainy": "\u{e318}", "rainy_heavy": "\u{e318}",
    "weather_snowy": "\u{e31a}", "thunderstorm": "\u{e31d}", "weather_hail": "\u{e31d}"
  })
  function weatherGlyph(name) {
    return root.weatherGlyphs[String(name || "")] || "\u{e33d}"
  }

  readonly property color text: "#ffffff"
  readonly property color dim: Qt.rgba(1, 1, 1, 0.72)
  readonly property color faint: Qt.rgba(1, 1, 1, 0.5)
  readonly property color hairline: Qt.rgba(1, 1, 1, 0.22)
  // Glass: transparent black, blurred behind by Hyprland (the login screen
  // blurs in-scene), dark enough for white type over bright sky and smoke.
  // The same fifth of black as every other surface of Vetro's, and no outline:
  // the compositor's glass draws the edge.
  readonly property color glass: Qt.rgba(0, 0, 0, 0.2)
  readonly property color glassEdge: "transparent"
  // A live broadcast is marked in white too.
  readonly property color live: "#ffffff"

  // Letter spacing for small capitals, in pixels per point of type.
  readonly property real capsTracking: 0.22
}

pragma Singleton

import QtQuick
import Quickshell

// The launch look, shared by the desktop cards and the login screen: pure
// white type and hairlines on black glass that Hyprland blurs, set in D-DIN
// like SpaceX's own site and webcast graphics. Black, white and transparency
// only: no tints, no gradients, no accent colors.
Singleton {
  id: root

  // D-DIN is bundled (fonts/, SIL Open Font License), so it works on the
  // login screen too, where nothing from ~/.local/share/fonts is visible.
  readonly property string family: din.status === FontLoader.Ready ? din.font.family : "sans-serif"
  // The extended cut, for the few big numbers.
  readonly property string wide: dinExp.status === FontLoader.Ready ? dinExp.font.family : family

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

  FontLoader {
    id: din

    source: Qt.resolvedUrl("fonts/D-DIN.otf")
  }

  FontLoader {
    source: Qt.resolvedUrl("fonts/D-DIN-Bold.otf")
  }

  FontLoader {
    id: dinExp

    source: Qt.resolvedUrl("fonts/D-DINExp.otf")
  }

  FontLoader {
    source: Qt.resolvedUrl("fonts/D-DINExp-Bold.otf")
  }
}

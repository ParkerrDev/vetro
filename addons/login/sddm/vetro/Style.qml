pragma Singleton

import QtQuick

// The launch look, as the desktop's launch/Style.qml has it: pure white
// type and hairlines on black glass, set in D-DIN. A plain QtObject here,
// since the login screen runs in SDDM, not Quickshell. The fonts load from
// the theme folder: the sddm user can't see the ones in your home.
QtObject {
  readonly property string family: din.status === FontLoader.Ready ? din.font.family : "sans-serif"
  // The extended cut, for the few big numbers.
  readonly property string wide: dinExp.status === FontLoader.Ready ? dinExp.font.family : family

  readonly property color text: "#ffffff"
  readonly property color dim: Qt.rgba(1, 1, 1, 0.72)
  readonly property color faint: Qt.rgba(1, 1, 1, 0.5)
  readonly property color hairline: Qt.rgba(1, 1, 1, 0.22)
  // Glass: transparent black over the in-scene blur
  readonly property color glass: Qt.rgba(0, 0, 0, 0.3)
  readonly property color glassEdge: Qt.rgba(1, 1, 1, 0.24)
  readonly property color live: "#ffffff"

  // Letter spacing for small capitals, in pixels per point of type.
  readonly property real capsTracking: 0.22

  property FontLoader din: FontLoader {
    source: Qt.resolvedUrl("fonts/D-DIN.otf")
  }
  property FontLoader dinBold: FontLoader {
    source: Qt.resolvedUrl("fonts/D-DIN-Bold.otf")
  }
  property FontLoader dinExp: FontLoader {
    source: Qt.resolvedUrl("fonts/D-DINExp.otf")
  }
  property FontLoader dinExpBold: FontLoader {
    source: Qt.resolvedUrl("fonts/D-DINExp-Bold.otf")
  }
  // The icons are a font too ("Material Symbols Rounded")
  property FontLoader symbols: FontLoader {
    source: Qt.resolvedUrl("fonts/MaterialSymbolsRounded.ttf")
  }
}

import QtQuick
import "BarModel.js" as BarModel

// A stand-in for Omarchy's Bar.qml, for the tests that run without Omarchy:
// nothing but the places tools/bar-build.py changes, each of them once and
// spelled as it is in Omarchy's. tests/qml.sh builds from the real one.
Item {
  id: root

  property bool vertical: false
  property real barSize: 40
  property bool transparent: false
  property var shell: null

  function toggleTransparency() {}

  PanelWindow {
    WlrLayershell.namespace: "omarchy-bar"
    WlrLayershell.layer: WlrLayer.Top

    Loader {
      anchors.fill: parent
    }
  }

  component CenterGestureArea: MouseArea {
    property bool dragging: false

    function startDrag(x, y) {
      if (dragging) return
      dragging = true
    }

    onDoubleClicked: function(mouse) {
      if (mouse.button === Qt.LeftButton) {
        root.toggleTransparency()
        mouse.accepted = true
      }
    }
  }

  component ModuleSlot: Item {
    MouseArea {
      readonly property bool canReorder: root.shell && typeof root.shell.mutateShellConfig === "function"
    }
  }
}

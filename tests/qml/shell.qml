// A throwaway Quickshell config for tests/qml.sh: it compiles the entry point of
// every Vetro plugin the way Omarchy's shell loads one, by the URL it has under
// ~/.config/omarchy/plugins (a link into this repository), and says what came
// of each. It creates no window and instantiates nothing.
import QtQuick
import Quickshell

ShellRoot {
  property var entries: [
    "vetro.barglass/Glass.qml", "vetro.center/Center.qml", "vetro.center-button/Bell.qml",
    "vetro.lock/Service.qml", "vetro.lock/LockView.qml", "vetro.menu/Menu.qml", "vetro.menu/BarWidget.qml",
    "vetro.notifications/Service.qml", "vetro.osd/Osd.qml", "vetro.cards/Panels.qml",
    "vetro.workspaces/Workspaces.qml", "vetro.bar/Bar.qml", "vetro.bar/VetroPane.qml"
  ]

  Component.onCompleted: {
    const base = "file://" + Quickshell.env("PLUGINS") + "/"
    for (const entry of entries) {
      const component = Qt.createComponent(base + entry)
      const errors = component.status === Component.Error ? component.errorString().replace(/\n/g, " | ") : ""
      console.log("COMPONENT " + entry + " status=" + component.status + " " + errors)
    }
    console.log("COMPONENTS DONE")
  }
}

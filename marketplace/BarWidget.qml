import QtQuick
import Quickshell
import Quickshell.Io
import qs.Commons
import qs.Ui

// Optional tray control. Installing or enabling it changes no desktop settings.
BarWidget {
  id: root
  moduleName: "dev.vetro.control"
  readonly property string control: decodeURIComponent(String(Qt.resolvedUrl("control")).replace(/^file:\/\//, ""))
  property string mode: "setup"
  Translations { id: strings; directory: Qt.resolvedUrl("i18n") }

  Process {
    id: probe
    command: [root.control, "state"]
    stdout: StdioCollector {
      onStreamFinished: {
        const value = text.trim()
        if (["setup", "ready", "off", "repair"].indexOf(value) >= 0) root.mode = value
      }
    }
  }
  Timer {
    interval: 30000
    running: true
    repeat: true
    triggeredOnStart: true
    onTriggered: if (!probe.running) probe.running = true
  }
  function shellQuote(value) { return "'" + value.replace(/'/g, "'\\''") + "'" }
  function activate(button) {
    if (mode === "setup" || mode === "repair") {
      Util.execArgv(["omarchy-launch-floating-terminal-with-presentation", shellQuote(control) + " " + mode])
    } else {
      Util.execArgv([control, button === Qt.RightButton ? "toggle" : "open"])
    }
    refresh.restart()
  }
  Timer { id: refresh; interval: 1500; onTriggered: if (!probe.running) probe.running = true }
  implicitWidth: button.implicitWidth
  implicitHeight: button.implicitHeight
  WidgetButton {
    id: button
    anchors.fill: parent
    bar: root.bar
    text: root.mode === "repair" ? "!" : String.fromCodePoint(0xf00b5)
    active: root.mode === "ready"
    tooltipText: strings.t(root.mode)
    onPressed: function(button) { root.activate(button) }
  }
}

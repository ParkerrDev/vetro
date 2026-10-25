import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import "../vetro.lib"

// Vetro's bar shape for Omarchy's own bar: a rounded pane with a hairline
// frame, drawn on its own under omarchy.bar. Omarchy's bar stays exactly what
// it is (its widgets, panels, gestures and keybinds); it only gives up its flat
// background, which ~/.config/omarchy/shell.toml turns off ([bar]
// background-alpha = 0).
//
// The shape is set by bar.radius, the gaps, panels.baseOpacity and
// panels.borders in ~/.config/vetro/shell.json. The colors are Omarchy's bar
// colors, which override/glass.toml sets to black and white while the glass
// draws, so the pane looks the same under every theme and wallpaper.
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

  // What Omarchy tells a plugin about the bar in use.
  readonly property var bar: shell ? shell.bar : null
  readonly property string barId: shell && shell.barConfig ? String(shell.barConfig.id || "") : ""
  readonly property int barSize: bar ? bar.barSize : 0
  readonly property string position: bar ? bar.position : "top"
  readonly property bool vertical: position === "left" || position === "right"
  // Only under Omarchy's own bar, and gone with it when the bar is hidden
  // (Super+Shift+Space).
  readonly property bool show: !!bar && !bar.barHidden && barSize > 0 && (barId === "" || barId === "omarchy.bar")

  // Omarchy's own see-through bar (double-click on the bar's middle) picks the
  // text colour from the wallpaper image, which comes out black on black
  // glass over a dark or video wallpaper. The glass is the bar's background
  // here, so that mode is switched straight back off.
  property bool barTransparent: false
  function undoTransparent() {
    if (barTransparent && show)
      Quickshell.execDetached(["omarchy-bar", "transparent", "false"]);
  }
  onBarTransparentChanged: undoTransparent()
  onShowChanged: undoTransparent()

  FileView {
    id: shellJson
    path: Quickshell.env("HOME") + "/.config/omarchy/shell.json"
    watchChanges: true
    printErrors: false
    onFileChanged: reload()
    onLoaded: {
      try {
        const bar = JSON.parse(shellJson.text()).bar;
        root.barTransparent = !!bar && bar.transparent === true;
      } catch (e) {}
    }
  }

  // Between the glass and the screen edge the bar is on, and the same again
  // on its other side: Omarchy centers its widgets in the bar's thickness.
  readonly property real edgeGap: Math.max(0, Math.min(Settings.edgeGap, barSize / 2 - 4))
  // At the two ends of the bar.
  readonly property real endGap: Settings.sideGap

  Variants {
    model: root.show ? Quickshell.screens : []

    PanelWindow {
      required property ShellScreen modelData

      screen: modelData
      color: "transparent"
      WlrLayershell.namespace: "vetro:barglass"
      // Under windows, and so always under Omarchy's bar.
      WlrLayershell.layer: WlrLayer.Bottom
      // Omarchy's bar reserves the space; this lies in it.
      exclusionMode: ExclusionMode.Ignore
      mask: Region {}
      // The surface is the pane and nothing more: the glass engine bends
      // light around a surface's own edge, so a pane inset in a larger
      // surface would have its rim in the wrong place.
      implicitWidth: root.vertical ? Math.max(1, root.barSize - 2 * root.edgeGap) : 0
      implicitHeight: root.vertical ? 0 : Math.max(1, root.barSize - 2 * root.edgeGap)
      margins {
        top: root.vertical ? root.endGap : root.edgeGap
        bottom: root.vertical ? root.endGap : root.edgeGap
        left: root.vertical ? root.edgeGap : root.endGap
        right: root.vertical ? root.edgeGap : root.endGap
      }

      anchors {
        top: root.position === "top" || root.vertical
        bottom: root.position === "bottom" || root.vertical
        left: root.position === "left" || !root.vertical
        right: root.position === "right" || !root.vertical
      }

      Rectangle {
        anchors.fill: parent
        radius: Math.min(Settings.barRadius, width / 2, height / 2)
        color: Look.film
        border.width: Look.frameWidth
        border.color: Look.frame
      }
    }
  }
}

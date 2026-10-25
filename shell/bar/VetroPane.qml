import QtQuick
import Quickshell
import "../vetro.lib"

// The bar's pane of glass, drawn by the bar's own surface: a rounded strip
// with a hairline frame, a little in from the bar's edges. tools/bar-build.py
// puts one into Vetro's build of Omarchy's bar, under everything else on it,
// and links this file in beside that Bar.qml (so the library is one folder
// up and one across, as it is for every plugin).
//
// It is the strip shell/vetro.barglass draws under Omarchy's own bar, in the
// same place and of the same shape (bar.radius, the gaps, panels.baseOpacity
// and panels.borders in ~/.config/vetro/shell.json), in the bar's surface and
// not under it. The bar's surface is above windows, so the glass is too. The
// engine takes the strip for a pane by how solid it is (hypr/vetro.lua).
Item {
  id: pane

  // From the bar this is in.
  property bool vertical: false
  property real barSize: 0
  // Omarchy's own see-through bar picks the text colour from the wallpaper,
  // which comes out black on black glass over a dark or video wallpaper. The
  // glass is the background here, so that mode is switched straight back off.
  property bool seeThrough: false

  // Between the glass and the screen edge the bar is on, and the same again
  // on its other side: Omarchy centers its widgets in the bar's thickness.
  readonly property real edgeGap: Math.max(0, Math.min(Settings.edgeGap, barSize / 2 - 4))
  // At the two ends of the bar.
  readonly property real endGap: Settings.sideGap

  function undoSeeThrough() {
    if (seeThrough)
      Quickshell.execDetached(["omarchy-bar", "transparent", "false"]);
  }
  onSeeThroughChanged: undoSeeThrough()
  Component.onCompleted: undoSeeThrough()

  Rectangle {
    anchors.fill: parent
    anchors.topMargin: pane.vertical ? pane.endGap : pane.edgeGap
    anchors.bottomMargin: pane.vertical ? pane.endGap : pane.edgeGap
    anchors.leftMargin: pane.vertical ? pane.edgeGap : pane.endGap
    anchors.rightMargin: pane.vertical ? pane.edgeGap : pane.endGap
    radius: Math.min(Settings.barRadius, width / 2, height / 2)
    color: Look.film
    border.width: Look.frameWidth
    border.color: Look.frame
  }
}

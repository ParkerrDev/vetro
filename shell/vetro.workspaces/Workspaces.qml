import QtQuick
import QtQuick.Layouts
import QtQuick.Effects
import Quickshell
import Quickshell.Hyprland
import Quickshell.Io
import qs.Commons
import qs.Ui
import "../vetro.lib" as V

// Omarchy's workspace numbers, with one thing added: a pane of glass over the
// focused number, which slides to the one that is focused now. It is a pane,
// not a drop: it moves as one piece, with a soft landing, and never stretches,
// wobbles or reaches for the pointer. It is clear but for the light its edge
// catches (pane.frag), lit all the way round from a little left of straight
// above; while it slides, that light swings with the motion and brightens,
// then settles (2026-10-22: "the lighting doesn't feel dynamic, its split down
// the center", "I would rather it look like a pane of glass", "I don't like how
// it follows and moves with the mouse", "it only wiggles horizontally").
//
// The numbers are Hyprland's global workspaces. Every bar shows the same set:
// 1 to 5 always, plus any other numbered workspace that exists, dimmer when
// empty. Pressing one focuses that workspace wherever it is shown; a right
// click locks the display that shows it to it, or frees it (a locked one
// is a little display with its number cut out of the screen, and room of
// its own). The pane
// on each bar marks the workspace that bar's monitor shows; its number
// remains visible.
//
// A button beside the numbers that stands for a workspace of its own is one
// more place for the pane to go: it slides over to that button when the
// focus goes to its workspace, and back. Such a button says so itself, with
// two properties this widget looks for and nothing else:
//   workspaceName  the name of the workspace it stands for
//   marked         (optional) set here while the pane is on it, so that
//                  it can leave the marking to the pane
// passthru's Windows button is one. A widget with several workspaces of its
// own says so instead with these:
//   workspaceNames  the names of the workspaces it stands for
//   spotOf(name)    (optional) where along the bar, from the widget's own
//                   start, that one's button stands; its middle if absent
// agent-workspace's robot is one. Without any, the numbers are all there is.
//
// A number blinks when asked (`omarchy-shell vetro.workspaces blink N`):
// three flashes in the bar's attention colour. vetro-focus-or-open asks when
// the app wanted is already open, to say which workspace it went to.
BarWidget {
  id: root
  moduleName: "vetro.workspaces"

  // The workspace whose number blinks, 0 for none, and the flashes left: the
  // number is lit on even steps, so 6 steps are three flashes.
  property int blinkId: 0
  property int blinkStep: 0

  function blink(id) {
    root.blinkId = id
    root.blinkStep = 6
    blinker.restart()
  }

  Timer {
    id: blinker
    interval: 170
    repeat: true
    onTriggered: {
      root.blinkStep -= 1
      if (root.blinkStep <= 0) {
        stop()
        root.blinkId = 0
      }
    }
  }

  // One target serves every bar: the instance that holds it blinks them all.
  IpcHandler {
    target: "vetro.workspaces"
    function blink(workspace: int): void {
      var items = root.bar && typeof root.bar.moduleWidgets === "function"
        ? root.bar.moduleWidgets(root.moduleName) : [root]
      for (var i = 0; i < items.length && i < 16; i++) {
        if (items[i] && typeof items[i].blink === "function") items[i].blink(workspace)
      }
    }
  }

  function workspaceById(id) {
    var values = Hyprland.workspaces.values
    for (var i = 0; i < values.length; i++) {
      if (values[i].id === id) return values[i]
    }

    return null
  }

  // The monitor this bar is on.
  readonly property var monitor: {
    var window = root.QsWindow.window
    var screen = window ? window.screen : null
    return screen ? Hyprland.monitorFor(screen) : Hyprland.focusedMonitor
  }

  // The workspace this monitor shows.
  readonly property var shownWorkspace: root.monitor ? root.monitor.activeWorkspace : Hyprland.focusedWorkspace

  // Show the first five global workspaces, then every other numeric workspace
  // that exists, on every monitor's bar.
  function workspaceIds() {
    var ids = [1, 2, 3, 4, 5]
    var values = Hyprland.workspaces.values

    for (var i = 0; i < values.length; i++) {
      var id = values[i].id
      if (id <= 0) continue
      if (ids.indexOf(id) === -1) ids.push(id)
    }

    ids.sort(function(left, right) { return left - right })
    return ids
  }

  // Displays locked to a workspace (Super+Shift+L, or a right click on a
  // number) cannot be switched; such a number stands on a little display on every bar. The
  // locks belong to the Hyprland side, which keeps them in this file, one
  // "MONITOR ID" a line. Without it there is no file and nothing is locked;
  // a right click then does nothing.
  property var lockedIds: []
  function readLocks(text) {
    var ids = []
    var lines = String(text || "").split("\n")
    for (var i = 0; i < lines.length && i < 32; i++) {
      var match = /^\S+\s+(-?\d+)\s*$/.exec(lines[i])
      if (match) ids.push(Number(match[1]))
    }
    if (JSON.stringify(ids) !== JSON.stringify(root.lockedIds)) root.lockedIds = ids
  }
  FileView {
    id: locksFile
    path: (Quickshell.env("XDG_RUNTIME_DIR") || "/tmp") + "/hypr-workspace-locks"
    watchChanges: true
    printErrors: false
    onFileChanged: reload()
    onLoaded: root.readLocks(text())
    onLoadFailed: root.readLocks("")
  }
  // The file is replaced, not written in place, and may not exist yet, so it
  // is read again now and then as well.
  Timer {
    interval: 1000
    running: true
    repeat: true
    onTriggered: locksFile.reload()
  }

  function toggleLock(id) {
    root.dispatch("(o.workspaces.toggle_lock or function() return function() end end)(" + id
      + (root.monitor ? ", " + JSON.stringify(root.monitor.name) : "") + ")")
  }

  function dispatch(expression) {
    if (root.bar) root.bar.run("hyprctl dispatch " + Util.shellQuote(expression))
  }

  function focusWorkspace(id) {
    root.dispatch("o.workspaces.focus(" + id + (root.monitor ? ", " + JSON.stringify(root.monitor.name) : "") + ")")
  }

  // Quickshell credits every workspace switch to the focused monitor, and
  // misses a workspace that changes number or moves between monitors, so ask
  // Hyprland for the truth then. Every bar shares what Hyprland answers, so
  // one asks after a plain switch.
  Connections {
    target: Hyprland
    function onRawEvent(event) {
      var name = event ? String(event.name) : ""
      var switched = name === "workspacev2" && root.monitor !== null && root.monitor.focused
      if (switched || name === "moveworkspacev2" || name === "changeworkspaceid") {
        Hyprland.refreshMonitors()
        Hyprland.refreshWorkspaces()
      }
    }
  }

  readonly property real trailingGap: root.vertical ? 0 : Style.spaceReal(1.5)
  // The room a number has along the bar, and the gap to the next one.
  readonly property real cell: root.vertical ? root.barSize : Style.space(20)
  readonly property real gap: root.vertical ? Style.space(2) : Style.space(1)
  // A locked number has more room, so the display drawn behind it stands
  // clear of the numbers beside it.
  readonly property real lockedCell: root.vertical ? root.cell : Math.round(root.cell + Style.font.body * 1.3)
  function cellFor(id) {
    return root.lockedIds.indexOf(id) !== -1 ? root.lockedCell : root.cell
  }

  implicitWidth: grid.implicitWidth + trailingGap
  implicitHeight: grid.implicitHeight

  // The bar keeps every widget in a place of its own, and the places of one
  // side stand in one row: this widget's, and those beside it.
  function placeOf(widget) {
    var loader = widget ? widget.parent : null
    return loader ? loader.parent : null
  }

  function along(item) {
    return item ? (root.vertical ? item.y : item.x) : 0
  }

  function extent(item) {
    return item ? (root.vertical ? item.height : item.width) : 0
  }

  readonly property var row: {
    var place = root.placeOf(root)
    return place ? place.parent : null
  }

  // The widgets of that row that stand for a workspace, and the one the glass
  // is drawn in: the first of them and this one, as the row goes. What comes
  // first is drawn first, so glass drawn at the bottom of that one lies under
  // every number and every logo it passes.
  readonly property var beside: {
    var found = { stops: [], host: root }
    var places = root.row ? root.row.children : []
    var first = true
    for (var i = 0; i < places.length && i < 64; i++) {
      var widget = places[i].activeItem
      if (!widget) continue
      var names = widget.workspaceNames
      var stop = widget !== root && widget.visible
        && ((typeof widget.workspaceName === "string" && widget.workspaceName !== "") || (names && names.length > 0))
      if (stop) found.stops.push(widget)
      if (first && (stop || widget === root)) {
        found.host = widget
        first = false
      }
    }
    return found
  }
  readonly property var stops: beside.stops
  readonly property Item host: beside.host

  function stopFor(workspace) {
    for (var i = 0; workspace !== null && i < root.stops.length; i++) {
      var each = root.stops[i]
      if (each.workspaceName === workspace.name) return each
      if (each.workspaceNames && each.workspaceNames.indexOf(workspace.name) !== -1) return each
    }

    return null
  }

  // The workspace the pane marks: the one this bar's monitor shows, so each
  // monitor's bar marks its own workspace (2026-11-27: "each monitor should
  // have the current workspace it is on selected").
  readonly property var current: root.shownWorkspace

  // Where the marked number is among those shown: -1 when it has no number
  // here (a special one, or a neighbour's).
  readonly property int focusedIndex: {
    var focused = root.current
    return focused === null ? -1 : root.workspaceIds().indexOf(focused.id)
  }
  // The neighbour whose workspace is marked, if one is.
  readonly property var focusedStop: root.stopFor(root.current)
  // With the focus on a workspace that has no button, the selector is put away.
  readonly property bool pointing: focusedStop !== null || focusedIndex >= 0

  // Where the selector belongs: the middle of the focused workspace's button,
  // along the bar, measured from the host's place. Every number has the same
  // room, so a number's middle follows from where it stands among them; a
  // neighbour's is where the bar has put it. Not a number, with the focus on
  // a workspace that has no button.
  function spot() {
    return root.spotFor(root.current)
  }

  // The same for any workspace.
  function spotFor(focused) {
    if (focused === null || focused === undefined) return NaN
    var origin = root.along(root.placeOf(root.host))
    var stop = root.stopFor(focused)
    if (stop !== null) {
      var place = root.placeOf(stop)
      if (!root.vertical && typeof stop.spotOf === "function") {
        var at = stop.spotOf(focused.name)
        if (!isNaN(at)) return root.along(place) + at - origin
      }
      return root.along(place) + root.extent(place) / 2 - origin
    }

    var ids = root.workspaceIds()
    var index = ids.indexOf(focused.id)
    if (index < 0) return NaN
    var before = 0
    for (var i = 0; i < index && i < 64; i++) before += root.cellFor(ids[i]) + root.gap
    return root.along(root.placeOf(root)) + before + root.cellFor(focused.id) / 2 - origin
  }

  // The same, worked out again whenever anything it is made of changes: the
  // focus, the numbers shown, a place in the row.
  readonly property real wanted: {
    var at = root.spot()
    return isNaN(at) ? 0 : at
  }

  // Where the pane stands along the bar, in the host's own measure. It moves
  // as one piece.
  property real centre: 0
  property bool placed: false
  property bool settled: false
  // The workspace the pane was last put on, and the neighbour that has been
  // told the pane is on it.
  property int cameFrom: 0
  property var circled: null
  // 1 or -1 while the pane slides right or left, easing back to 0 as it lands.
  property real sweep: 0

  Behavior on centre {
    enabled: root.settled
    NumberAnimation { duration: V.Look.ms(240); easing.type: Easing.OutCubic }
  }

  SequentialAnimation {
    id: flight
    property real toward: 1
    NumberAnimation { target: root; property: "sweep"; to: flight.toward; duration: V.Look.ms(90); easing.type: Easing.OutQuad }
    NumberAnimation { target: root; property: "sweep"; to: 0; duration: V.Look.ms(380); easing.type: Easing.InOutQuad }
  }

  function tell(stop) {
    if (root.circled === stop) return
    if (root.circled && root.circled.marked !== undefined) root.circled.marked = false
    if (stop && stop.marked !== undefined) stop.marked = true
    root.circled = stop
  }

  // Put the pane where it belongs: sliding there when the focus has gone to
  // another workspace, at once when the bar has only moved the button. With
  // the same workspace in the same place there is nothing to do.
  function follow() {
    var focused = root.current
    var at = root.spot()
    root.tell(isNaN(at) ? null : root.stopFor(focused))
    if (isNaN(at)) return
    var elsewhere = root.placed && focused.id !== root.cameFrom
    if (root.placed && !elsewhere && Math.abs(at - root.centre) < 0.5) return
    root.settled = elsewhere
    if (elsewhere) {
      flight.stop()
      flight.toward = at > root.centre ? 1 : -1
      flight.start()
    }
    root.centre = at
    root.cameFrom = focused.id
    root.placed = true
  }

  // Put anew, without a slide: another host measures from another corner,
  // and a bar of another size has a pane of another size.
  function anew() {
    root.placed = false
    Qt.callLater(root.follow)
  }

  onWantedChanged: Qt.callLater(root.follow)
  onPointingChanged: Qt.callLater(root.follow)
  onHostChanged: root.anew()
  Component.onCompleted: Qt.callLater(root.follow)
  Component.onDestruction: root.tell(null)

  // The pane: a tile of clear glass a little wider than it is tall, its light
  // drawn by pane.frag. The bar's own glass is what shows through it.
  ShaderEffect {
    id: selector
    parent: root.host
    z: -1
    visible: root.pointing && root.placed
    readonly property real thickness: Math.min(root.barSize - Style.space(10), Style.space(20))
    readonly property real tile: thickness * 1.2
    readonly property real across: parent ? (root.vertical ? parent.width : parent.height) : 0
    onThicknessChanged: root.anew()

    x: root.vertical ? (across - width) / 2 : root.centre - tile / 2
    y: root.vertical ? root.centre - tile / 2 : (across - height) / 2
    width: root.vertical ? thickness : tile
    height: root.vertical ? tile : thickness

    property size paneSize: Qt.size(width, height)
    property real radius: Math.min(width, height) * 0.34
    // A little left of straight above; a slide swings it by up to 18 degrees.
    property real lightAngle: (-24 + 18 * root.sweep) * Math.PI / 180
    property real glare: 0.62
    property real opposite: 0.55
    property real sharpness: 2.2
    property real rimDepth: 2.2
    property real fresnel: 0.10
    property real body: 0.035
    property real hairline: 0.34
    property real glint: Math.abs(root.sweep)
    property real pixel: 1 / Math.max(1, Screen.devicePixelRatio)

    fragmentShader: Qt.resolvedUrl("pane.frag.qsb")
  }

  // The workspace the mouse is on, on every bar: its number is the brightest
  // (2026-11-27: "the cursor selection should be visible from every
  // display"; "it should just become brighter instead of a square").
  // Hyprland's focus follows the mouse, so the focused monitor's workspace is
  // the mouse's.
  readonly property var mouseWorkspace: Hyprland.focusedWorkspace

  // Which workspaces some display shows now: each such number is at full
  // brightness, one with windows but not shown a little dimmer, an empty one
  // dimmest (2026-11-27: "more clear what workspace numbers are
  // currently active on each display").
  readonly property var shownIds: {
    var ids = []
    var all = Hyprland.monitors.values
    for (var i = 0; i < all.length && i < 16; i++) {
      var ws = all[i].activeWorkspace
      if (ws && ids.indexOf(ws.id) === -1) ids.push(ws.id)
    }
    return ids
  }


  GridLayout {
    id: grid
    anchors.fill: parent
    anchors.rightMargin: root.trailingGap
    columns: root.vertical ? 1 : root.workspaceIds().length
    columnSpacing: root.vertical ? 0 : root.gap
    rowSpacing: root.vertical ? root.gap : 0

    Repeater {
      id: numbers
      model: root.workspaceIds()

      WidgetButton {
        required property int modelData

        readonly property var workspace: root.workspaceById(modelData)
        readonly property bool occupied: workspace !== null && workspace.toplevels.values.length > 0
        readonly property bool focused: root.current !== null && root.current.id === modelData
        // Some display shows it now.
        readonly property bool shown: root.shownIds.indexOf(modelData) !== -1

        readonly property bool blinking: root.blinkId === modelData
        // Lit on even steps: full, a little larger and in the attention colour;
        // between them, all but gone. A theme whose attention colour is its
        // text colour still shows the flashes.
        readonly property bool lit: blinking && root.blinkStep % 2 === 0

        bar: root.bar
        text: modelData === 10 ? "0" : String(modelData)
        active: lit
        readonly property bool mouseHere: root.mouseWorkspace !== null && root.mouseWorkspace.id === modelData
        opacity: blinking ? (lit ? 1 : 0.15) : (mouseHere ? 1 : shown ? 0.72 : occupied ? 0.5 : 0.25)
        Behavior on opacity {
          enabled: !blinking
          NumberAnimation { duration: V.Look.ms(160) }
        }
        scale: lit ? 1.25 : 1
        Behavior on scale {
          NumberAnimation { duration: V.Look.ms(110); easing.type: Easing.OutQuad }
        }
        horizontalMargin: 6
        verticalPadding: 6
        fixedWidth: root.vertical ? root.barSize : root.cellFor(modelData)
        labelVisible: !lockedHere
        fixedHeight: root.vertical ? root.cell : root.barSize
        // A right click locks this display to it, or frees it.
        onPressed: function(button) {
          if (button === Qt.RightButton) root.toggleLock(modelData)
          else root.focusWorkspace(modelData)
        }

        readonly property bool lockedHere: root.lockedIds.indexOf(modelData) !== -1
        readonly property color lockInk: root.bar ? root.bar.barForeground : Color.foreground
        // A locked one is a little display: a filled screen on a stand, its
        // number cut out of the screen so the bar shows through it.
        Item {
          id: plate
          visible: parent.lockedHere
          anchors.fill: parent
          readonly property string label: parent.text
          readonly property real screenHeight: Math.round(Style.font.body * 1.3)
          readonly property real screenWidth: Math.round(screenHeight * 1.5)
          readonly property real stem: Math.max(2, Math.round(Style.font.body * 0.16))

          Item {
            id: shape
            anchors.fill: parent
            visible: false
            layer.enabled: true
            Rectangle {
              id: screen
              width: plate.screenWidth
              height: plate.screenHeight
              anchors.centerIn: parent
              anchors.verticalCenterOffset: -Math.round(Style.font.body * 0.2)
              radius: Math.round(height * 0.2)
              color: parent.parent.parent.lockInk
            }
            Rectangle {
              width: Math.max(2, Math.round(Style.font.body * 0.14))
              height: plate.stem
              anchors.horizontalCenter: parent.horizontalCenter
              anchors.top: screen.bottom
              color: screen.color
            }
            Rectangle {
              width: Math.round(plate.screenWidth * 0.42)
              height: plate.stem
              radius: height / 2
              anchors.horizontalCenter: parent.horizontalCenter
              anchors.top: screen.bottom
              anchors.topMargin: plate.stem
              color: screen.color
            }
          }
          Item {
            id: hole
            anchors.fill: parent
            visible: false
            layer.enabled: true
            Text {
              text: plate.label
              font.family: root.bar ? root.bar.fontFamily : Style.font.family
              font.pixelSize: Math.round(Style.font.body * 0.95)
              font.bold: true
              color: "black"
              x: screen.x + (screen.width - width) / 2
              y: screen.y + (screen.height - height) / 2
            }
          }
          MultiEffect {
            anchors.fill: parent
            source: shape
            maskEnabled: true
            maskSource: hole
            maskInverted: true
            maskThresholdMin: 0.4
            maskSpreadAtMin: 0.3
            // Full under the number's own brightness; brighter still where
            // the mouse is.
            opacity: plate.parent.mouseHere ? 0.85 : 0.75
          }
        }

      }
    }
  }
}

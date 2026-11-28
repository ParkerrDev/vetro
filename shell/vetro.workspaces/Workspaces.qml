import QtQuick
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
// The numbers are Omarchy's: each display's block of ten, from the file
// Omarchy keeps (default/hypr/monitor-workspaces.lua), those that exist.
// Pressing one goes to it through o.workspaces.focus, as Omarchy's keys do. The pane marks the workspace this
// bar's display shows.
//
// The numbers stand in trays, one for each display that holds any of them,
// in the order the displays stand, each led by the display's icon (laptop,
// monitor, or cast for a virtual one), all alike. A dot under a number says it has windows; one without is there
// but empty, and dimmer. Only workspaces that exist are shown. The mouse's
// display has its icon at full brightness (the others are dimmed), and its
// workspace's number too. Agents' workspaces (agent-workspace)
// have a tray of their own, led by a robot.
//
// Where the Hyprland side keeps locks in $XDG_RUNTIME_DIR/hypr-workspace-locks
// (shared-workspaces does), a locked number is a little display with its
// number cut out, a right click locks or frees, and the pane stays off a
// locked number: the display already says it is shown.
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

  // Each display's block of ten, as Omarchy keeps them: connector -> block.
  readonly property int slots: 10
  readonly property string monitorName: root.monitor ? root.monitor.name : ""
  property var blocks: ({})

  function readBlocks(text) {
    var next = {}
    var lines = String(text || "").split("\n")
    for (var i = 0; i < lines.length && i < 64; i++) {
      var fields = lines[i].split("\t")
      if (fields.length >= 3 && fields[1] !== "") next[fields[1]] = parseInt(fields[0], 10)
    }
    root.blocks = next
  }

  FileView {
    path: (Quickshell.env("XDG_STATE_HOME") || Quickshell.env("HOME") + "/.local/state") + "/omarchy/monitor-workspaces"
    watchChanges: true
    printErrors: false
    onFileChanged: reload()
    onLoaded: root.readBlocks(text())
    onLoadFailed: root.readBlocks("")
  }

  function blockOf(id) {
    return Math.floor((id - 1) / root.slots)
  }

  function slotOf(id) {
    return (id - 1) % root.slots + 1
  }

  // Until Hyprland has written the file, the workspace a monitor shows is in
  // its own block.
  readonly property int block: {
    var block = root.blocks[root.monitorName]
    if (block !== undefined) return block
    var shown = root.shownWorkspace
    return shown && shown.id > 0 ? root.blockOf(shown.id) : 0
  }

  // Those of this display's own block that exist (2026-11-27: the unused
  // ones "are not needed").
  function workspaceIds() {
    var ids = []
    var values = Hyprland.workspaces.values
    for (var i = 0; i < values.length && i < 128; i++) {
      var id = values[i].id
      if (id > 0 && root.blockOf(id) === root.block && ids.indexOf(id) === -1) ids.push(id)
    }
    return ids.sort(function(left, right) { return left - right })
  }

  // Displays locked to a workspace, from the Hyprland side's file, one
  // "MONITOR ID" a line (see the top). The file being there at all says
  // locks can be made.
  property bool locksKnown: false
  property var lockedIds: []
  property var lockedMonitors: []
  property var lockedBy: ({})
  function readLocks(text) {
    var ids = []
    var monitors = []
    var by = {}
    var lines = String(text || "").split("\n")
    for (var i = 0; i < lines.length && i < 32; i++) {
      var match = /^(\S+)\s+(-?\d+)\s*$/.exec(lines[i])
      if (!match) continue
      ids.push(Number(match[2]))
      monitors.push(match[1])
      by[Number(match[2])] = match[1]
    }
    if (JSON.stringify(ids) !== JSON.stringify(root.lockedIds)) root.lockedIds = ids
    if (JSON.stringify(monitors) !== JSON.stringify(root.lockedMonitors)) root.lockedMonitors = monitors
    if (JSON.stringify(by) !== JSON.stringify(root.lockedBy)) root.lockedBy = by
  }
  FileView {
    id: locksFile
    path: (Quickshell.env("XDG_RUNTIME_DIR") || "/tmp") + "/hypr-workspace-locks"
    watchChanges: true
    printErrors: false
    onFileChanged: reload()
    onLoaded: {
      root.locksKnown = true
      root.readLocks(text())
    }
    onLoadFailed: {
      root.locksKnown = false
      root.readLocks("")
    }
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
    root.dispatch("o.workspaces.focus(" + root.slotOf(id) + ", " + JSON.stringify(root.monitorName) + ")")
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
      // Quickshell can lose track of which workspace a window is on (one
      // moved to a headless display, say), so a window's coming, going or
      // moving asks Hyprland for each workspace's count too (2026-11-27:
      // "the icons are not getting dots even though I put windows on them").
      if (name === "openwindow" || name === "closewindow" || name === "movewindowv2")
        recount.restart()
    }
  }
  Timer {
    id: recount
    interval: 120
    onTriggered: Hyprland.refreshWorkspaces()
  }

  // Whether a workspace has windows: by Quickshell's count, or by Hyprland's
  // own, whichever sees one.
  function hasWindows(workspace) {
    if (!workspace) return false
    if (workspace.toplevels.values.length > 0) return true
    var ipc = workspace.lastIpcObject
    return ipc !== null && ipc !== undefined && Number(ipc.windows) > 0
  }

  readonly property real trailingGap: root.vertical ? 0 : Style.spaceReal(1.5)
  // The room a number has along the bar, and the gap to the next one.
  readonly property real cell: root.vertical ? root.barSize : Style.space(20)
  readonly property real gap: root.vertical ? Style.space(2) : Style.space(1)
  // A locked number has more room, so the display drawn behind it stands
  // clear of the numbers beside it.
  readonly property real lockedCell: root.vertical ? root.cell : Math.round(root.cell + Style.font.body * 1.3)

  // A tray's inner margin, and the room between trays.
  readonly property real trayPad: Style.space(3)
  readonly property real trayGap: Style.space(4)

  function monitorPosition(m) {
    var ipc = m.lastIpcObject || {}
    return { x: m.x !== undefined ? m.x : (ipc.x || 0), y: m.y !== undefined ? m.y : (ipc.y || 0) }
  }

  // The displays, left to right (then top to bottom), as they stand.
  readonly property var monitorOrder: {
    var list = []
    var all = Hyprland.monitors.values
    for (var i = 0; i < all.length && i < 16; i++) {
      var at = root.monitorPosition(all[i])
      list.push({ name: String(all[i].name), x: at.x, y: at.y })
    }
    list.sort(function (a, b) { return a.x !== b.x ? a.x - b.x : a.y - b.y })
    var names = []
    for (var j = 0; j < list.length; j++) names.push(list[j].name)
    return names
  }

  // The kind of display, for its icon: the laptop's own panel, a virtual one
  // (another machine's screen), or a monitor.
  function displayKind(name) {
    if (/^(eDP|LVDS|DSI)/.test(name)) return "laptop"
    if (/^(HEADLESS|VIRTUAL|WL-|X11-)/i.test(name)) return "remote"
    return "monitor"
  }
  function displayGlyph(kind) {
    return kind === "laptop" ? "\u{f0322}" : kind === "remote" ? "\u{f0118}" : "\u{f0379}"
  }

  function monitorOfLock(id) {
    return root.lockedBy[id] || ""
  }

  // The numbers in trays, one tray for each display that holds any of them,
  // in the order the displays stand, and the numbers no display holds:
  //   { trays: [{ monitor, kind, ids, shown, here, locked }], free: [ids] }
  readonly property var layout: {
    var ids = root.workspaceIds()
    var byMonitor = {}
    var free = []
    for (var i = 0; i < ids.length && i < 64; i++) {
      var ws = root.workspaceById(ids[i])
      var name = ws && ws.monitor ? String(ws.monitor.name) : ""
      if (name === "") { free.push(ids[i]); continue }
      if (!byMonitor[name]) byMonitor[name] = []
      byMonitor[name].push(ids[i])
    }
    var all = Hyprland.monitors.values
    var trays = []
    for (var j = 0; j < root.monitorOrder.length && j < 16; j++) {
      var monitor = root.monitorOrder[j]
      var shown = 0
      // A named workspace it shows (the agents' workspace, a desk): a robot
      // in its tray says so.
      var agent = ""
      for (var k = 0; k < all.length && k < 16; k++) {
        var active = all[k].activeWorkspace
        if (String(all[k].name) !== monitor || !active) continue
        shown = active.id
        if (active.id < 0 && String(active.name).indexOf("special:") !== 0) agent = String(active.name)
      }
      if (!byMonitor[monitor] && agent === "") continue
      if (!byMonitor[monitor]) byMonitor[monitor] = []
      var locked = root.lockedMonitors.indexOf(monitor) !== -1
      var held = byMonitor[monitor]
      // A locked display is just its little display: the one it is locked
      // to; anything else hidden there stands with the free ones.
      if (locked) {
        var lockedId = -1
        for (var key in root.lockedBy) if (root.lockedBy[key] === monitor) lockedId = Number(key)
        if (held.indexOf(lockedId) !== -1) {
          for (var h = 0; h < held.length; h++) if (held[h] !== lockedId) free.push(held[h])
          held = [lockedId]
        } else {
          locked = false
        }
      }
      trays.push({ monitor: monitor, kind: root.displayKind(monitor), ids: held, agent: agent,
                   shown: shown, here: monitor === root.monitorName, locked: locked })
    }
    free.sort(function(left, right) { return left - right })
    return { trays: trays, free: free }
  }

  // The layout the trays are drawn from, changed only when it says something
  // new. Hyprland's events come many times a second, and a fresh layout each
  // time would make every number anew, so for a moment the pane would find
  // the new numbers still standing at the start of their tray (2026-11-27:
  // "falls back to the first workspace number of that display").
  property var stable: ({ trays: [], free: [] })
  property string stableText: ""
  function settle() {
    var text = JSON.stringify(root.layout)
    if (text === root.stableText) return
    root.stableText = text
    root.stable = root.layout
  }
  onLayoutChanged: root.settle()

  implicitWidth: (root.vertical ? strip.implicitWidth : strip.implicitWidth + trailingGap)
  implicitHeight: strip.implicitHeight

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
  // Each number's button, by workspace id, so the pane can find its middle.
  property var buttons: ({})
  property int buttonsVersion: 0
  // A number's button by workspace; an agents' one is kept by name.
  function buttonFor(workspace) {
    if (!workspace) return null
    return root.buttons[workspace.id] || root.buttons["name:" + workspace.name] || null
  }
  function register(id, button) {
    root.buttons[id] = button
    root.buttonsVersion++
  }
  function unregister(id, button) {
    if (root.buttons[id] === button) delete root.buttons[id]
    root.buttonsVersion++
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
    void root.buttonsVersion
    return focused === null ? -1 : (root.buttonFor(focused) ? 0 : -1)
  }
  // The neighbour whose workspace is marked, if one is.
  readonly property var focusedStop: root.stopFor(root.current)
  // With the focus on a workspace that has no button, or on a locked one, the
  // pane is put away: the little display already says it is shown.
  readonly property bool pointing: focusedStop !== null
    || (focusedIndex >= 0 && root.lockedIds.indexOf(root.current.id) === -1)

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

    var button = root.buttonFor(focused)
    if (!button) return NaN
    return root.along(root.placeOf(root)) + button.middle - origin
  }

  // The same, worked out again whenever anything it is made of changes: the
  // focus, the numbers shown, a place in the row.
  readonly property real wanted: {
    var at = root.spot()
    void root.buttonsVersion
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
  // Numbers made anew stand where their row puts them only after it lays
  // them out, so look again once they have settled.
  onButtonsVersionChanged: resettle.restart()
  Timer {
    id: resettle
    interval: 80
    onTriggered: root.follow()
  }
  onPointingChanged: Qt.callLater(root.follow)
  onHostChanged: root.anew()
  Component.onCompleted: {
    root.settle()
    Qt.callLater(root.follow)
  }
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

  // One number's button. `tray` is the tray it stands in, null for a free one.
  component WorkspaceNumber: WidgetButton {
    id: number
    required property int workspaceId
    property var tray: null
    property bool inTray: tray !== null

    readonly property var workspace: root.workspaceById(workspaceId)
    readonly property bool occupied: root.hasWindows(workspace)
    // Its display shows it now.
    readonly property bool shown: inTray && tray.shown === workspaceId
    readonly property bool mouseHere: root.mouseWorkspace !== null && root.mouseWorkspace.id === workspaceId
    readonly property bool blinking: root.blinkId === workspaceId
    // Lit on even steps: full, a little larger and in the attention colour;
    // between them, all but gone. A theme whose attention colour is its text
    // colour still shows the flashes.
    readonly property bool lit: blinking && root.blinkStep % 2 === 0
    readonly property int slot: root.slotOf(workspaceId)
    readonly property bool lockedHere: root.lockedIds.indexOf(workspaceId) !== -1
    // Its middle along the bar, from this widget's start.
    readonly property real middle: {
      var at = root.vertical ? y + height / 2 : x + width / 2
      for (var item = parent; item && item !== root && item !== null; item = item.parent)
        at += root.vertical ? item.y : item.x
      return at
    }
    onMiddleChanged: root.buttonsVersion++
    Component.onCompleted: root.register(workspaceId, number)
    Component.onDestruction: root.unregister(workspaceId, number)

    bar: root.bar
    text: slot === root.slots ? "0" : String(slot)
    foreground: mouseHere && !blinking ? root.mouseInk : root.ink
    active: lit
    // With apps, bright; there but empty, dim; unused, dimmer still.
    opacity: blinking ? (lit ? 1 : 0.15) : (mouseHere ? 1 : occupied ? 0.9 : inTray ? 0.5 : 0.28)
    Behavior on opacity {
      enabled: !number.blinking
      NumberAnimation { duration: V.Look.ms(160) }
    }
    scale: lit ? 1.25 : 1
    Behavior on scale {
      NumberAnimation { duration: V.Look.ms(110); easing.type: Easing.OutQuad }
    }
    horizontalMargin: 6
    verticalPadding: 6
    fixedWidth: root.vertical ? root.barSize - 2 * root.trayPad : lockedHere ? root.lockedCell : root.cell
    labelVisible: !lockedHere
    fixedHeight: root.vertical ? root.cell : root.barSize
    tooltipText: (occupied ? "Has windows" : inTray ? "Empty" : "Unused")
                 + (inTray ? ", on " + tray.monitor + (shown ? ", shown there" : "") : "")
    // A right click locks this display to it, or frees it, where the
    // Hyprland side has locks.
    onPressed: function(button) {
      if (button === Qt.RightButton && root.locksKnown) root.toggleLock(workspaceId)
      else root.focusWorkspace(workspaceId)
    }

    // A dot under the number says it has windows (2026-11-27: "underline
    // should just show a workspace has applications on it, or maybe use
    // dots").
    AppDot {
      visible: number.occupied && !number.lockedHere
    }

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
      // Drawn at the display's own pixels, not the bar's units, so a scaled
      // display does not blow the cut-out up into steps (2026-11-27: "the
      // numbers and dots inside the display icons are pixilated").
      readonly property size sharp: Qt.size(Math.ceil(width * Screen.devicePixelRatio),
                                            Math.ceil(height * Screen.devicePixelRatio))

      Item {
        id: shape
        anchors.fill: parent
        Rectangle {
          id: screen
          width: plate.screenWidth
          height: plate.screenHeight
          anchors.centerIn: parent
          anchors.verticalCenterOffset: -Math.round(Style.font.body * 0.2)
          radius: Math.round(height * 0.2)
          color: root.ink
        }
        Rectangle {
          width: Math.max(2, Math.round(Style.font.body * 0.14))
          height: plate.stem
          anchors.horizontalCenter: screen.horizontalCenter
          anchors.top: screen.bottom
          color: screen.color
        }
        Rectangle {
          width: Math.round(plate.screenWidth * 0.42)
          height: plate.stem
          radius: height / 2
          anchors.horizontalCenter: screen.horizontalCenter
          anchors.top: screen.bottom
          anchors.topMargin: plate.stem
          color: screen.color
        }
      }
      Item {
        id: hole
        anchors.fill: parent
        Text {
          text: plate.label
          font.family: root.bar ? root.bar.fontFamily : Style.font.family
          font.pixelSize: Math.round(Style.font.body * 0.95)
          font.bold: true
          color: "black"
          // Aside, to make room for the dot, when it has windows.
          x: screen.x + (screen.width - width) / 2 - (number.occupied ? dot.width * 0.9 : 0)
          y: screen.y + (screen.height - height) / 2
        }
        // A dot cut beside the number says the display has windows on it
        // (2026-11-27: "displays with windows on them should get little dots
        // inside their screen icon").
        Rectangle {
          id: dot
          width: Math.max(3, Math.round(Style.font.body * 0.26))
          height: width
          radius: width / 2
          color: number.occupied ? "black" : "transparent"
          x: screen.x + screen.width - width - Math.round(screen.height * 0.28)
          y: screen.y + (screen.height - height) / 2
        }
      }
      // The screen and its cut-out, each taken as a picture the effect reads.
      // They are live sources that hide what they take: a hidden item with
      // layer.enabled was never drawn again, so a dot that came or went
      // after the first frame never showed (2026-11-27: "if i take it away
      // and put it back it is still stale").
      ShaderEffectSource {
        id: shapeSource
        anchors.fill: parent
        visible: false
        sourceItem: shape
        hideSource: true
        live: true
        smooth: true
        textureSize: plate.sharp
      }
      ShaderEffectSource {
        id: holeSource
        anchors.fill: parent
        visible: false
        sourceItem: hole
        hideSource: true
        live: true
        smooth: true
        textureSize: plate.sharp
      }
      MultiEffect {
        anchors.fill: parent
        source: shapeSource
        maskEnabled: true
        maskSource: holeSource
        maskInverted: true
        // The cut follows the text's own soft edge, not a hard threshold.
        maskThresholdMin: 0.5
        maskSpreadAtMin: 1
        // Full under the number's own brightness; brighter still where
        // the mouse is.
        opacity: plate.parent.mouseHere ? 1 : 0.7
      }
    }

  }

  readonly property color ink: root.bar ? root.bar.barForeground : Color.foreground
  // The mouse's display has its icon at full brightness, the others dimmed,
  // and its workspace's number at full brightness (2026-11-27: "more clear
  // which display/workspace the mouse is on"; no accent colour, no outline:
  // "just the icon should get brighter").
  readonly property color mouseInk: root.ink
  readonly property string mouseMonitor: Hyprland.focusedMonitor ? String(Hyprland.focusedMonitor.name) : ""

  // The dot under a number with windows (beside it on a vertical bar).
  component AppDot: Rectangle {
    readonly property real size: Math.max(3, Math.round(Style.font.body * 0.28))
    width: size
    height: size
    radius: size / 2
    color: root.ink
    x: root.vertical ? Math.round(Style.font.body * 0.15) : (parent.width - width) / 2
    y: root.vertical ? (parent.height - height) / 2 : parent.height / 2 + Math.round(Style.font.body * 0.62)
  }

  // A tray's back: a capsule, brighter on this bar's own display.
  // Every tray looks the same: the one for this bar's own display is not
  // lit apart (2026-11-27: highlighting it is "kinda pointless").
  component TrayBack: Rectangle {
    anchors.centerIn: parent
    readonly property real slim: selector.thickness + Style.space(2)
    width: root.vertical ? Math.min(parent.width - Style.space(6), slim) : parent.width
    height: root.vertical ? parent.height : Math.min(parent.height - Style.space(10), slim)
    radius: Math.min(width, height) / 2
    color: Qt.rgba(root.ink.r, root.ink.g, root.ink.b, 0.06)
    border.width: 1
    border.color: Qt.rgba(root.ink.r, root.ink.g, root.ink.b, 0.14)
  }

  // The agents' workspaces, in a tray of their own led by a robot (once
  // agent-workspace's own bar button; 2026-11-27: "the agent workspaces should
  // share this exact same look, in-fact just merge the projects together").
  // They are, in order, the agents' workspace while it has windows (its live
  // view, agent.watch, does not count), each agent desk (agent-ws new), and
  // each other named workspace with windows. A click goes to one, or back
  // from it; a right click on a desk shuts the desk down. Nothing shows while
  // none is open.
  readonly property string agentsWorkspace: "agents"
  readonly property string deskPrefix: "agents-"
  property int agentsRunning: 0

  readonly property var agentEntries: {
    var all = Hyprland.workspaces.values
    var main = []
    var desks = []
    var others = []
    var shownName = root.current ? String(root.current.name) : ""
    for (var i = 0; i < all.length && i < 128; i++) {
      var name = String(all[i].name)
      var wins = all[i].toplevels.values
      var count = wins.length
      if (/^-?[0-9]+$/.test(name) || name.indexOf("special:") === 0) continue
      if (name === root.agentsWorkspace) {
        for (var w = 0; w < wins.length && w < 256; w++) {
          var ipc = wins[w].lastIpcObject
          var app = wins[w].wayland ? String(wins[w].wayland.appId) : ""
          if (app === "agent.watch" || (ipc && ipc["class"] === "agent.watch")) count--
        }
        if (count > 0 || name === shownName)
          main.push({ name: name, kind: "agents", label: "Agents' workspace", windows: count, workspace: all[i] })
      } else if (name.indexOf(root.deskPrefix) === 0) {
        desks.push({ name: name, kind: "desk", label: "Desk " + name.slice(root.deskPrefix.length), windows: count, workspace: all[i] })
      } else if (count > 0) {
        others.push({ name: name, kind: "other", label: "Workspace " + name, windows: count, workspace: all[i] })
      }
    }
    var byName = function (a, b) { return a.name < b.name ? -1 : a.name > b.name ? 1 : 0 }
    desks.sort(byName)
    others.sort(byName)
    return main.concat(desks, others)
  }

  Process {
    id: agentProbe
    command: ["agent-ws", "agents", "--json"]
    stdout: StdioCollector {
      onStreamFinished: {
        try {
          root.agentsRunning = JSON.parse(text).length
        } catch (e) {
          root.agentsRunning = 0
        }
      }
    }
  }
  Timer {
    interval: 5000
    running: root.agentEntries.length > 0
    repeat: true
    triggeredOnStart: true
    onTriggered: if (!agentProbe.running) agentProbe.running = true
  }

  function goAgent(entry, button) {
    if (!root.bar) return
    if (entry.kind === "desk" && button === Qt.RightButton) {
      root.bar.run("agent-ws close " + entry.name.slice(root.deskPrefix.length))
      return
    }
    var target = root.current && String(root.current.name) === entry.name ? "previous_per_monitor" : "name:" + entry.name
    root.dispatch("hl.dsp.focus({ workspace = " + JSON.stringify(target) + " })")
  }

  readonly property bool mouseOnAgents: {
    var m = root.mouseWorkspace
    if (!m) return false
    for (var i = 0; i < root.agentEntries.length && i < 64; i++) {
      if (root.agentEntries[i].name === String(m.name)) return true
    }
    return false
  }

  // Which workspace each display shows, by name.
  readonly property var shownNames: {
    var names = []
    var all = Hyprland.monitors.values
    for (var i = 0; i < all.length && i < 16; i++) {
      if (all[i].activeWorkspace) names.push(String(all[i].activeWorkspace.name))
    }
    return names
  }

  // A robot standing for the named workspace a display shows.
  component ShownAgent: WidgetButton {
    id: shownAgent
    required property string name
    readonly property var workspace: {
      var all = Hyprland.workspaces.values
      for (var i = 0; i < all.length && i < 128; i++) if (String(all[i].name) === name) return all[i]
      return null
    }
    readonly property bool occupied: root.hasWindows(workspace)
    readonly property real middle: {
      var at = root.vertical ? y + height / 2 : x + width / 2
      for (var item = parent; item && item !== root && item !== null; item = item.parent)
        at += root.vertical ? item.y : item.x
      return at
    }
    onMiddleChanged: root.buttonsVersion++
    Component.onCompleted: root.register("name:" + name, shownAgent)
    Component.onDestruction: root.unregister("name:" + name, shownAgent)

    bar: root.bar
    text: "\u{f06a9}"
    centerFigures: false
    foreground: root.ink
    opacity: 1
    horizontalMargin: 6
    verticalPadding: 6
    fixedWidth: root.vertical ? root.barSize - 2 * root.trayPad : root.cell
    fixedHeight: root.vertical ? root.cell : root.barSize
    tooltipText: (name === root.agentsWorkspace ? "Agents' workspace"
                  : name.indexOf(root.deskPrefix) === 0 ? "Desk " + name.slice(root.deskPrefix.length)
                  : "Workspace " + name) + ", shown here"
    onPressed: function(button) {
      root.goAgent({ name: name, kind: name.indexOf(root.deskPrefix) === 0 ? "desk" : "agents" }, button)
    }

    AppDot {
      visible: shownAgent.occupied
    }
  }

  // One agents' workspace's button: numbered in order, marked like the rest.
  component AgentNumber: WidgetButton {
    id: agentNumber
    required property var entry
    required property int index
    readonly property bool occupied: entry.windows > 0
    readonly property bool shown: root.shownNames.indexOf(entry.name) !== -1
    readonly property bool mouseHere: root.mouseWorkspace !== null && String(root.mouseWorkspace.name) === entry.name
    readonly property real middle: {
      var at = root.vertical ? y + height / 2 : x + width / 2
      for (var item = parent; item && item !== root && item !== null; item = item.parent)
        at += root.vertical ? item.y : item.x
      return at
    }
    onMiddleChanged: root.buttonsVersion++
    Component.onCompleted: root.register("agent:" + entry.name, agentNumber)
    Component.onDestruction: root.unregister("agent:" + entry.name, agentNumber)

    bar: root.bar
    text: String(index + 1)
    foreground: mouseHere ? root.mouseInk : root.ink
    opacity: mouseHere ? 1 : occupied ? 0.9 : 0.5
    Behavior on opacity {
      NumberAnimation { duration: V.Look.ms(160) }
    }
    horizontalMargin: 6
    verticalPadding: 6
    fixedWidth: root.vertical ? root.barSize - 2 * root.trayPad : root.cell
    fixedHeight: root.vertical ? root.cell : root.barSize
    tooltipText: entry.kind === "agents"
                 ? entry.label + ": " + entry.windows + (entry.windows === 1 ? " window, " : " windows, ")
                   + root.agentsRunning + (root.agentsRunning === 1 ? " agent running" : " agents running")
                 : entry.kind === "desk" ? entry.label + " (right click shuts it down)"
                 : entry.label + ": " + entry.windows + (entry.windows === 1 ? " window" : " windows")
    onPressed: function(button) { root.goAgent(entry, button) }

    AppDot {
      visible: agentNumber.occupied
    }
  }

  Grid {
    id: strip
    anchors.left: parent.left
    anchors.top: parent.top
    flow: root.vertical ? Grid.TopToBottom : Grid.LeftToRight
    columns: root.vertical ? 1 : 0
    rows: root.vertical ? 0 : 1
    spacing: root.trayGap
    verticalItemAlignment: Grid.AlignVCenter
    horizontalItemAlignment: Grid.AlignHCenter

    // A tray for each display: its icon, a lock if it is locked, and the
    // numbers it holds. This bar's own display's tray is the brighter one
    // (2026-11-27: "an entire overhaul of workspace management and grouping",
    // to make "more clear which workspaces belong to which monitors").
    Repeater {
      model: root.stable.trays

      Item {
        id: trayItem
        required property var modelData
        readonly property var tray: modelData
        readonly property real pad: trayItem.tray.locked ? 0 : root.trayPad
        implicitWidth: root.vertical ? root.barSize : trayRow.implicitWidth + 2 * pad
        implicitHeight: root.vertical ? trayRow.implicitHeight + 2 * pad : root.barSize
        width: implicitWidth
        height: implicitHeight

        // A locked display's tray is only its little display (2026-11-27:
        // "when locking a workspace it should make the entire row/section
        // become just an icon").
        TrayBack {
          visible: !trayItem.tray.locked
        }

        Grid {
          id: trayRow
          anchors.centerIn: parent
          flow: root.vertical ? Grid.TopToBottom : Grid.LeftToRight
          columns: root.vertical ? 1 : 0
          rows: root.vertical ? 0 : 1
          spacing: root.gap
          verticalItemAlignment: Grid.AlignVCenter
          horizontalItemAlignment: Grid.AlignHCenter

          // The display: a click goes to it.
          WidgetButton {
            visible: !trayItem.tray.locked
            bar: root.bar
            text: root.displayGlyph(trayItem.tray.kind)
            centerFigures: false
            foreground: trayItem.tray.monitor === root.mouseMonitor ? root.mouseInk : root.ink
            opacity: trayItem.tray.monitor === root.mouseMonitor ? 1 : 0.45
            horizontalMargin: 4
            verticalPadding: 6
            fixedHeight: root.vertical ? -1 : root.barSize
            tooltipText: trayItem.tray.monitor + (trayItem.tray.here ? " (this display)" : "")
                         + (trayItem.tray.locked ? ", locked to what it shows" : "")
            onPressed: function(button) {
              root.dispatch("hl.dsp.focus({ monitor = " + JSON.stringify(trayItem.tray.monitor) + " })")
            }
          }

          Repeater {
            model: trayItem.tray.ids
            WorkspaceNumber {
              required property int modelData
              workspaceId: modelData
              tray: trayItem.tray
            }
          }

          // The agents' workspace (or a desk) this display shows, as a robot
          // where its number would be (2026-11-27: "the monitor looking at it
          // should get a robot workspace icon as its number").
          Repeater {
            model: trayItem.tray.agent !== "" ? [trayItem.tray.agent] : []
            ShownAgent {
              required property string modelData
              name: modelData
            }
          }
        }
      }
    }

    // The agents' tray.
    Item {
      id: agentTray
      visible: root.agentEntries.length > 0
      implicitWidth: root.vertical ? root.barSize : agentRow.implicitWidth + 2 * root.trayPad
      implicitHeight: root.vertical ? agentRow.implicitHeight + 2 * root.trayPad : root.barSize
      width: visible ? implicitWidth : 0
      height: visible ? implicitHeight : 0

      TrayBack {
      }

      Grid {
        id: agentRow
        anchors.centerIn: parent
        flow: root.vertical ? Grid.TopToBottom : Grid.LeftToRight
        columns: root.vertical ? 1 : 0
        rows: root.vertical ? 0 : 1
        spacing: root.gap
        verticalItemAlignment: Grid.AlignVCenter
        horizontalItemAlignment: Grid.AlignHCenter

        // nf-md-robot: goes to the agents' workspace, and back from it.
        WidgetButton {
          bar: root.bar
          text: "\u{f06a9}"
          centerFigures: false
          foreground: root.mouseOnAgents ? root.mouseInk : root.ink
          opacity: root.mouseOnAgents ? 1 : 0.45
          horizontalMargin: 4
          verticalPadding: 6
          fixedHeight: root.vertical ? -1 : root.barSize
          tooltipText: root.agentEntries.length + (root.agentEntries.length === 1 ? " agent workspace open, " : " agent workspaces open, ")
                       + root.agentsRunning + (root.agentsRunning === 1 ? " agent running" : " agents running")
          onPressed: function(button) {
            root.goAgent({ name: root.agentsWorkspace, kind: "agents" }, button)
          }
        }

        Repeater {
          model: root.agentEntries
          AgentNumber {
            required property var modelData
            entry: modelData
          }
        }
      }
    }

    // The numbers no display holds, apart from the trays.
    Grid {
      visible: root.stable.free.length > 0
      flow: root.vertical ? Grid.TopToBottom : Grid.LeftToRight
      columns: root.vertical ? 1 : 0
      rows: root.vertical ? 0 : 1
      spacing: root.gap
      Repeater {
        model: root.stable.free
        WorkspaceNumber {
          required property int modelData
          workspaceId: modelData
        }
      }
    }
  }
}

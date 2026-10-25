pragma Singleton
pragma ComponentBehavior: Bound

import QtQuick
import Quickshell
import Quickshell.Io

// The notifications, as vetro-notifyd keeps them. The daemon holds the
// notification name on the session bus and every rule (how long a toast
// stays, Do Not Disturb, what an action does); this starts it while the
// notifications plugin is loaded, tells it what the shell's buttons do, and
// keeps what it says as records the panels draw.
//
// Lines of JSON both ways: commands in (`{"cmd": "dismiss", "id": 4}`),
// events out (`{"event": "note", ...}`). See shell/notifyd/src/lines.rs.
Singleton {
  id: root

  // Set by the notifications plugin: the daemon runs while it is true.
  property bool serving: false
  // Set by the centre while it is loaded, so the bell and the history key
  // know there is one to open.
  property bool centerAvailable: false

  // What the daemon says.
  readonly property bool running: daemon.running
  property bool owner: false
  property bool dnd: false
  // Every notification not dismissed, the newest first, and the ones up as
  // toasts.
  property var all: []
  readonly property var toasts: all.filter(note => note.up)

  // Seconds since 1970, a little behind: what "2m ago" is counted from.
  property real now: Date.now() / 1000

  // Says how long ago, the way a phone does: now, 5m, 2h, then the date.
  function ago(time) {
    const seconds = Math.max(0, now - time);
    if (seconds < 60)
      return "now";
    if (seconds < 3600)
      return Math.floor(seconds / 60) + "m";
    if (seconds < 86400)
      return Math.floor(seconds / 3600) + "h";
    return new Date(time * 1000).toLocaleDateString(Qt.locale(), "d MMM");
  }

  function dismiss(id) {
    send({ cmd: "dismiss", id: id });
  }
  function hide(id) {
    send({ cmd: "hide", id: id });
  }
  // The default action, or the one named.
  function invoke(id, action) {
    send(action ? { cmd: "invoke", id: id, action: action } : { cmd: "invoke", id: id });
  }
  function hideAll() {
    send({ cmd: "hideAll" });
  }
  function clear() {
    send({ cmd: "clear" });
  }
  function reshow() {
    send({ cmd: "reshow" });
  }
  // The pointer is over the toasts of one screen, or has left them: a toast
  // keeps its time while any screen holds it.
  function hold() {
    send({ cmd: "hold" });
  }
  function release() {
    send({ cmd: "release" });
  }
  function setDnd(on) {
    send({ cmd: "dnd", on: !!on });
  }

  // Runs what a notification asked to be run when it is clicked, as Omarchy's
  // own notifications do: an argument list, never a shell line.
  function launch(note) {
    if (note.exec.length > 0)
      Quickshell.execDetached(Array.from(note.exec));
  }

  function send(command) {
    if (daemon.running)
      daemon.write(JSON.stringify(command) + "\n");
  }

  function find(id) {
    return all.find(note => note.id === id) ?? null;
  }

  function receive(line) {
    let event = null;
    try {
      event = JSON.parse(line);
    } catch (error) {
      console.warn("vetro-notifyd said something unreadable:", line.slice(0, 80));
      return;
    }
    switch (event.event) {
    case "note":
      take(event);
      break;
    case "toast": {
      const note = find(event.id);
      if (note)
        note.up = !!event.up;
      break;
    }
    case "closed":
      all = all.filter(note => note.id !== event.id);
      break;
    case "dnd":
      dnd = !!event.on;
      break;
    case "bus":
      owner = !!event.owner;
      if (owner)
        again.interval = 2000;
      break;
    }
  }

  // A new or replaced notification goes to the front.
  function take(event) {
    const fields = {
      id: event.id,
      app: String(event.app ?? ""),
      summary: String(event.summary ?? ""),
      body: String(event.body ?? ""),
      icon: String(event.icon ?? ""),
      image: String(event.image ?? ""),
      picture: String(event.picture ?? ""),
      desktopEntry: String(event.desktopEntry ?? ""),
      urgency: Number(event.urgency ?? 1),
      actions: Array.isArray(event.actions) ? event.actions : [],
      exec: Array.isArray(event.exec) ? event.exec.map(String) : [],
      resident: !!event.resident,
      passing: !!event.transient,
      time: Number(event.time ?? 0),
      up: !!(event.toast && event.toast.up)
    };
    let note = find(event.id);
    if (note) {
      for (const key in fields)
        note[key] = fields[key];
    } else {
      // Without a parent, so it is let go once nothing shows it.
      note = record.createObject(null, fields);
    }
    all = [note].concat(all.filter(other => other !== note));
  }

  component Record: QtObject {
    property int id: 0
    property string app: ""
    property string summary: ""
    property string body: ""
    property string icon: ""
    property string image: ""
    // A picture sent in memory, as a file the daemon wrote.
    property string picture: ""
    property string desktopEntry: ""
    // 0 low, 1 normal, 2 critical
    property int urgency: 1
    // [{ key, label }], "default" among them when it has one
    property var actions: []
    property list<string> exec: []
    property bool resident: false
    // Transient: gone once its toast is (QML keeps the word for itself).
    property bool passing: false
    // seconds since 1970
    property real time: 0
    property bool up: false

    readonly property bool critical: urgency === 2
    readonly property var buttons: actions.filter(action => action.key !== "default" && action.label !== "")
    readonly property bool clickable: exec.length > 0 || actions.some(action => action.key === "default")
  }

  Component {
    id: record
    Record {}
  }

  Process {
    id: daemon

    command: [Quickshell.env("HOME") + "/.local/bin/vetro-notifyd"]
    stdinEnabled: true
    stdout: SplitParser {
      onRead: line => root.receive(line)
    }
    stderr: SplitParser {
      onRead: line => console.warn("vetro-notifyd:", line)
    }
    // A fresh daemon knows nothing of the last one's notifications.
    onStarted: {
      root.all = [];
      root.owner = false;
    }
    onExited: (code, status) => {
      root.owner = false;
      if (root.serving)
        again.restart();
    }
  }

  onServingChanged: daemon.running = serving

  Timer {
    interval: 20000
    repeat: true
    running: root.all.length > 0
    triggeredOnStart: true
    onTriggered: root.now = Date.now() / 1000
  }

  // Started again after it stops, while it is wanted: two seconds later, and
  // twice as long each time it stops again without having held the name, up
  // to a minute.
  Timer {
    id: again
    interval: 2000
    onTriggered: {
      daemon.running = root.serving;
      interval = Math.min(60000, interval * 2);
    }
  }
}

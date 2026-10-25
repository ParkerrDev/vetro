import QtQuick
import Quickshell
import Quickshell.Io

// What vetro-launchd knows: upcoming SpaceX launches, and the cards' own
// settings from its state file (the video wallpaper it reported on is gone,
// so `live` stays false). The login screen uses this too, pointed at
// /var/lib/vetro since it can't read $HOME.
Scope {
  id: root

  property string launchesPath: Quickshell.env("HOME") + "/.cache/vetro/launches.json"
  property string statePath: Quickshell.env("XDG_RUNTIME_DIR") + "/vetro/state.json"
  // Only tick the countdown while something shows it.
  property bool active: true

  property var launches: ({})
  property var wallpaper: ({})
  property real now: Date.now() / 1000
  // Coarse clock for filtering lists, so they don't rebuild every second.
  property real minuteNow: Date.now() / 1000

  readonly property var upcoming: {
    const list = launches.upcoming || [];
    const t = minuteNow;
    return list.filter(l => l.net_epoch && l.net_epoch > t - postroll(l));
  }
  readonly property var next: upcoming.length > 0 ? upcoming[0] : null
  readonly property var later: upcoming.slice(1)
  readonly property var last: (launches.previous || []).find(l => l.net_epoch && l.net_epoch < minuteNow) || null
  readonly property bool live: wallpaper.mode === "live" && !!wallpaper.live
  readonly property bool muted: wallpaper.muted !== false
  readonly property var overlay: wallpaper.overlay || ({})
  readonly property var weatherCard: wallpaper.weather || ({})
  readonly property string error: launches.error || ""

  function postroll(l) {
    return (l.rocket || "").toLowerCase().indexOf("starship") >= 0 ? 75 * 60 : 20 * 60;
  }

  function parse(file) {
    try {
      return JSON.parse(file.text());
    } catch (e) {
      return null;
    }
  }

  Timer {
    interval: 1000
    repeat: true
    running: root.active
    triggeredOnStart: true
    onTriggered: {
      root.now = Date.now() / 1000;
      if (root.now - root.minuteNow >= 30)
        root.minuteNow = root.now;
    }
  }

  FileView {
    id: launchesFile

    path: root.launchesPath
    watchChanges: true
    printErrors: false
    onFileChanged: reload()
    onLoaded: {
      const data = root.parse(launchesFile);
      if (data)
        root.launches = data;
    }
  }

  FileView {
    id: stateFile

    path: root.statePath
    watchChanges: true
    printErrors: false
    onFileChanged: reload()
    onLoaded: {
      const data = root.parse(stateFile);
      if (data)
        root.wallpaper = data;
    }
  }

  // The daemon replaces files with a rename, which a file watch can lose
  // track of. A slow re-read covers that.
  Timer {
    interval: 20000
    repeat: true
    running: true
    onTriggered: {
      launchesFile.reload();
      stateFile.reload();
    }
  }
}

import QtQuick

// What vetro-launchd knows, as the login screen gets it: the launch list
// the daemon copies to /var/lib/vetro (the sddm user can't read your home).
// Read with XMLHttpRequest, which needs QML_XHR_ALLOW_FILE_READ=1 in the
// greeter's environment (/etc/sddm.conf.d/99-zz-vetro.conf sets it). The
// launch card reads the same properties as on the desktop.
QtObject {
  id: root

  property string launchesPath: "/var/lib/vetro/launches.json"

  property var launches: ({})
  // The wallpaper's state lives in your session, not here
  readonly property var wallpaper: ({})
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
  readonly property bool live: false
  readonly property bool muted: true
  readonly property string error: launches.error || ""

  function postroll(l) {
    return (l.rocket || "").toLowerCase().indexOf("starship") >= 0 ? 75 * 60 : 20 * 60;
  }

  function load() {
    const xhr = new XMLHttpRequest();
    xhr.onreadystatechange = function () {
      if (xhr.readyState !== XMLHttpRequest.DONE || !xhr.responseText)
        return;
      try {
        root.launches = JSON.parse(xhr.responseText);
      } catch (e) {
        console.warn("launches.json: " + e);
      }
    };
    xhr.open("GET", "file://" + launchesPath);
    xhr.send();
  }

  property Timer clock: Timer {
    interval: 1000
    repeat: true
    running: true
    triggeredOnStart: true
    onTriggered: {
      root.now = Date.now() / 1000;
      if (root.now - root.minuteNow >= 30)
        root.minuteNow = root.now;
    }
  }

  // The daemon rewrites the list now and then
  property Timer reread: Timer {
    interval: 60000
    repeat: true
    running: true
    triggeredOnStart: true
    onTriggered: root.load()
  }
}

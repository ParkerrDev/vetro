pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Services.Pipewire
import Quickshell.Wayland
import "../vetro.lib"

// Vetro's on-screen display: a capsule of glass under the bar, with the
// volume or the brightness as a filling track, or a message. This plugin is
// omarchy.osd's clone, so Omarchy's scripts (`omarchy-osd`, the volume and
// brightness keys, mic mute, logout, ...) land here and show in the same
// capsule. The volume shows whenever it changes, by any means.
Item {
  id: root

  // Read by Omarchy, like omarchy.osd's.
  property bool opened: false

  // What the capsule shows: an icon (a Material Symbols name), a value from
  // 0 to 1 or -1 for none, and a message.
  property string icon: "volume_up"
  property real value: -1
  property string message: ""

  // The same payload omarchy.osd takes:
  // { icon, message, value, max, progressText, duration }
  function open(payloadJson) {
    let p = {};
    try {
      p = JSON.parse(payloadJson || "{}");
    } catch (e) {
      return;
    }
    const max = Math.max(1, parseInt(p.max ?? "100", 10) || 100);
    const raw = parseInt(p.value ?? "", 10);
    const hasValue = !isNaN(raw) && !p.message;
    const name = String(p.icon || "").toLowerCase();
    // The volume keys change the volume (which shows here already) and then
    // send it as well: keep to the live volume.
    if (hasValue && name.startsWith("volume")) {
      showVolume();
      return;
    }
    const value = hasValue ? Math.min(1, Math.max(0, raw / max)) : -1;
    const duration = parseInt(p.duration ?? "", 10);
    show(iconFor(name, value), value, String(p.message || ""), isNaN(duration) ? 0 : duration);
  }

  function close() {
    opened = false;
  }

  function show(icon, value, message, duration) {
    root.icon = icon;
    root.value = value;
    root.message = message;
    opened = true;
    closing.interval = duration > 0 ? duration : Settings.osdTimeout;
    closing.restart();
  }

  // The default output's volume, as it is now.
  readonly property PwNode sink: Pipewire.defaultAudioSink
  function showVolume() {
    const audio = sink?.audio;
    if (!audio)
      return;
    const level = Math.max(0, Math.min(1, audio.volume));
    const icon = audio.muted || level === 0 ? "volume_off" : (level > 0.5 ? "volume_up" : "volume_down");
    show(icon, audio.muted ? 0 : level, "", 0);
  }

  PwObjectTracker {
    objects: [root.sink]
  }

  // The volume changes by the keys, the mixer or an app: shown each time,
  // but not when the shell starts or the output changes.
  Connections {
    target: root.sink?.audio ?? null
    function onVolumeChanged() {
      if (root.sink.ready)
        root.showVolume();
    }
    function onMutedChanged() {
      if (root.sink.ready)
        root.showVolume();
    }
  }

  // Omarchy's icon names, as Material Symbols. A Material Symbols name
  // passes through; anything else (Omarchy also sends raw Nerd Font glyphs)
  // gets a neutral one.
  function iconFor(name, value) {
    switch (name) {
    case "volume-muted": case "volume-mute": case "muted": case "mute": return "volume_off";
    case "volume-low": case "volume-medium": return "volume_down";
    case "volume-high": case "volume": return "volume_up";
    case "microphone-muted": case "microphone-off": case "mic-muted": case "mic-off": return "mic_off";
    case "microphone": case "mic": return "mic";
    case "keyboard": return "keyboard";
    case "brightness": case "display": return value >= 0 && value < 0.5 ? "brightness_low" : "brightness_high";
    case "touchpad": return "touchpad_mouse";
    case "touch": case "touchscreen": return "touch_app";
    case "reboot": case "restart": return "restart_alt";
    case "shutdown": case "power": case "poweroff": return "power_settings_new";
    case "logout": case "sign-out": case "leave": return "logout";
    case "media": case "player": case "media-source": case "player-source": return "music_note";
    case "media-play": case "player-play": return "play_arrow";
    case "media-pause": case "player-pause": return "pause";
    case "media-next": case "player-next": return "skip_next";
    case "media-previous": case "player-previous": return "skip_previous";
    }
    if (/^[a-z0-9_]+$/.test(name))
      return name;
    return value >= 0 ? "tune" : "info";
  }

  Timer {
    id: closing
    onTriggered: root.opened = false
  }

  // One capsule on each screen the settings choose, its surface the capsule
  // and nothing more, so the glass engine sees one pane that keeps its shape.
  Variants {
    model: Settings.screens

    PanelWindow {
      id: window

      required property ShellScreen modelData

      screen: modelData
      visible: root.opened
      color: "transparent"
      WlrLayershell.namespace: "hyprland-shell:osd"
      WlrLayershell.layer: WlrLayer.Overlay
      WlrLayershell.keyboardFocus: WlrKeyboardFocus.None
      exclusiveZone: 0
      mask: Region {}
      implicitWidth: capsule.implicitWidth
      implicitHeight: capsule.implicitHeight

      anchors.top: true
      margins.top: Settings.sideGap

      Rectangle {
        id: capsule

        anchors.fill: parent
        implicitWidth: row.implicitWidth + 2 * 16
        implicitHeight: 44
        radius: height / 2
        color: Look.film
        border.width: Look.frameWidth
        border.color: Look.frame

        RowLayout {
          id: row

          anchors.centerIn: parent
          spacing: 12

          Icon {
            text: root.icon
            size: 20
            filled: true
          }

          // The level, as a track that fills.
          Rectangle {
            visible: root.value >= 0
            implicitWidth: 180
            implicitHeight: 8
            radius: 4
            color: Look.wellHover

            Rectangle {
              height: parent.height
              radius: parent.radius
              width: Math.max(height, parent.width * Math.max(0, root.value))
              opacity: root.value > 0 ? 1 : 0
              color: Look.ink
              Behavior on width {
                Glide {
                  length: Look.quick
                }
              }
            }
          }

          Label {
            visible: root.value >= 0
            Layout.preferredWidth: 30
            horizontalAlignment: Text.AlignRight
            text: Math.round(Math.max(0, root.value) * 100)
            font.weight: Font.DemiBold
            font.features: { "tnum": 1 }
          }

          Label {
            visible: root.value < 0
            Layout.maximumWidth: 420
            text: root.message
            font.weight: Font.DemiBold
          }
        }
      }
    }
  }

  IpcHandler {
    target: "osd"

    function show(payloadJson: string): string {
      root.open(payloadJson);
      return "ok";
    }
    function close(): string {
      root.close();
      return "ok";
    }
    function state(): string {
      return root.opened ? "open" : "closed";
    }
    function ping(): string {
      return "ok";
    }
  }
}

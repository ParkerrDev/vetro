pragma Singleton

import QtQuick
import Quickshell
import qs.Commons

// How Vetro's panels look and move. The glass under a panel does the frosting
// and the darkening; a panel is a film of black over it with white type, and
// every colour here is one of those two at some strength.
Singleton {
  id: root

  // The type and the film.
  readonly property color ink: "#ffffff"
  readonly property color paper: "#000000"
  readonly property color film: tint(paper, Settings.opacity)
  readonly property color text: ink
  readonly property color quiet: tint(ink, 0.55)
  readonly property color faint: tint(ink, 0.35)
  readonly property color hairline: tint(ink, 0.22)
  readonly property color frame: tint(ink, 0.22)
  readonly property real frameWidth: Settings.frame ? Settings.frameWidth : 0
  // A control on a panel: at rest, under the pointer, and switched on.
  readonly property color well: tint(ink, 0.07)
  readonly property color wellHover: tint(ink, 0.13)
  readonly property color wellLit: tint(ink, 0.22)
  readonly property color urgent: "#ff6961"

  // The shapes: the panels' corners from the settings, and everything on a
  // panel rounder than it is wide (pills and circles), so it reads soft.
  readonly property real radius: Settings.radius
  readonly property real inner: Math.max(6, Settings.radius - 8)

  // The type: Omarchy's own font (the `monospace` that `omarchy font set`
  // points at, JetBrainsMono Nerd Font as it ships), and its Nerd Font
  // glyphs for the icons, the ones Omarchy draws itself.
  readonly property string family: Style.font.family
  readonly property string iconFamily: Style.font.family

  // An icon's glyph by its name. The names are Vetro's (Material's, as it
  // had them); each glyph is the one Omarchy uses for that thing where it
  // has one (its OSD and notifications), otherwise the Material Design one
  // of the same name. A glyph given instead of a name passes through.
  readonly property var glyphs: ({
    "volume_off": "\u{eee8}", "volume_down": "\u{f027}", "volume_up": "\u{f028}",
    "mic_off": "\u{f036d}", "mic": "\u{f036c}", "keyboard": "\u{f030c}",
    "brightness_low": "\u{f0379}", "brightness_high": "\u{f0379}",
    "touchpad_mouse": "\u{f07f8}", "touch_app": "\u{f0741}",
    "restart_alt": "\u{f0709}", "power_settings_new": "\u{f0425}", "logout": "\u{f0343}",
    "music_note": "\u{f075a}", "play_arrow": "\u{f040a}", "pause": "\u{f03e4}",
    "skip_next": "\u{f04ad}", "skip_previous": "\u{f04ae}",
    "tune": "\u{f062e}", "close": "\u{f0156}", "clear_all": "\u{f039f}"
  })
  // Those with an outline form, filled and outlined.
  readonly property var pairs: ({
    "notifications": ["\u{f009a}", "\u{f009c}"],
    "do_not_disturb_on": ["\u{f009b}", "\u{f0a91}"],
    "info": ["\u{f02fc}", "\u{f02fd}"]
  })
  function glyph(name, filled) {
    var key = String(name || "")
    if (key === "") return ""
    if (/[^\x00-\x7f]/.test(key)) return key
    var pair = root.pairs[key]
    if (pair) return filled ? pair[0] : pair[1]
    return root.glyphs[key] || root.pairs["info"][1]
  }
  readonly property real small: Settings.textSize * 0.9
  readonly property real normal: Settings.textSize
  readonly property real large: Settings.textSize * 1.15
  readonly property real huge: Settings.textSize * 1.6
  readonly property bool nativeRendering: Settings.nativeRendering

  // The motion. Every length is Vetro's own at the glass's pace (2 has it
  // all twice as quick), times motion.scale in the settings (0 turns the
  // animations off, 2 has them twice as long). Settle starts gently and comes
  // to rest softly, as iOS's sheets do; spring goes past and comes back, as a
  // bubble does, by motion.bounce (a third of a tenth at Vetro's own); leave
  // gathers speed and goes. Moving is false when there is no motion at all.
  readonly property list<real> settle: [0.32, 0.72, 0, 1, 1, 1]
  readonly property list<real> spring: [0.3, 1 + 0.64 * Settings.bounce, 0.5, 1, 1, 1]
  readonly property list<real> leave: [0.4, 0, 0.8, 0.2, 1, 1]
  readonly property int quick: ms(180)
  readonly property int moderate: ms(360)
  readonly property int slow: ms(560)
  readonly property bool moving: Settings.motion > 0

  function ms(length) {
    return Math.round(length * Settings.motion / Settings.pace);
  }

  function tint(colour, alpha) {
    return Qt.rgba(colour.r, colour.g, colour.b, Math.max(0, Math.min(1, alpha)));
  }
}

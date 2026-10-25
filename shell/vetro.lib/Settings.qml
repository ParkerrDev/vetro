pragma Singleton
pragma ComponentBehavior: Bound

import QtQuick
import Quickshell
import Quickshell.Io

// The shell's settings, from ~/.config/vetro/shell.json. Every key is
// optional: what the file leaves out keeps the value here. The file is read
// again whenever it changes, so an edit shows at once.
//
//   gaps.edge       between the bar and the screen edge it sits on
//   gaps.sides      between the bar's ends, the toasts and the centre, and
//                   the sides of the screen
//   bar.radius      the bar's corners
//   panels.radius   the corners of the toasts, the centre and the cards
//   panels.opacity  how much of a film lies over the glass, 0 to 1
//   panels.frame    a hairline round each panel, and its width
//   text.size       the base point size; text.nativeRendering for hinted type
//   clock.twelveHour
//   osd.timeout     how long the volume and brightness display stays, in ms
//   motion.scale    every animation's length times this: 0 turns them off
//   motion.bounce   how far what springs goes past where it rests and comes
//                   back, 0 (not at all) to 1; 0.5 is Vetro's own
//
// The pace is the glass's (`vetro tune`, Motion, Pace, in
// ~/.config/vetro/glass.conf): the panels move at it as the windows do, and
// follow it when it is applied.
//   screens.only    the toasts and the display go only to these screens, by
//                   name, when any are named; screens.except never to these
Singleton {
  id: root

  readonly property real edgeGap: Math.max(0, file.gaps.edge)
  readonly property real sideGap: Math.max(0, file.gaps.sides)
  readonly property real barRadius: Math.max(0, file.bar.radius)
  readonly property real radius: Math.max(0, file.panels.radius)
  readonly property real opacity: Math.max(0, Math.min(1, file.panels.opacity))
  readonly property bool frame: file.panels.frame
  readonly property real frameWidth: Math.max(0, file.panels.frameWidth)
  readonly property real textSize: Math.max(6, file.text.size)
  readonly property bool nativeRendering: file.text.nativeRendering
  readonly property bool twelveHour: file.clock.twelveHour
  readonly property int osdTimeout: Math.max(100, file.osd.timeout)
  readonly property real motion: Math.max(0, Math.min(4, file.motion.scale))
  readonly property real bounce: Math.max(0, Math.min(1, file.motion.bounce))
  readonly property real pace: tuning.pace

  // The screens the toasts and the display go to.
  readonly property var screens: {
    const only = Array.from(file.screens.only);
    const except = Array.from(file.screens.except);
    const all = Quickshell.screens;
    const chosen = all.filter(s => (only.length === 0 || only.includes(s.name)) && !except.includes(s.name));
    return chosen.length ? chosen : all.slice(0, 1);
  }

  FileView {
    path: Quickshell.env("HOME") + "/.config/vetro/shell.json"
    watchChanges: true
    printErrors: false
    onFileChanged: reload()

    JsonAdapter {
      id: file

      property JsonObject gaps: JsonObject {
        property real edge: 5
        property real sides: 5
      }
      property JsonObject bar: JsonObject {
        property real radius: 15
      }
      property JsonObject panels: JsonObject {
        property real radius: 20
        property real opacity: 0.2
        property bool frame: false
        property real frameWidth: 1
      }
      property JsonObject text: JsonObject {
        property real size: 10
        property bool nativeRendering: false
      }
      property JsonObject clock: JsonObject {
        property bool twelveHour: false
      }
      property JsonObject osd: JsonObject {
        property int timeout: 700
      }
      property JsonObject motion: JsonObject {
        property real scale: 1
        property real bounce: 0.5
      }
      property JsonObject screens: JsonObject {
        property list<string> only: []
        property list<string> except: []
      }
    }
  }

  // The glass's tuning, for its pace: 2 twice as quick, 0.5 half as quick, held
  // to the engine's range; 1 where the file or the line is missing.
  FileView {
    id: tuning

    property real pace: 1

    path: Quickshell.env("HOME") + "/.config/vetro/glass.conf"
    watchChanges: true
    printErrors: false
    onFileChanged: reload()
    onLoaded: {
      const found = /^[ \t]*pace[ \t]*=[ \t]*([0-9.]+)/m.exec(text());
      const value = found ? parseFloat(found[1]) : 1;
      pace = isFinite(value) ? Math.max(0.5, Math.min(2, value)) : 1;
    }
    onLoadFailed: pace = 1
  }
}

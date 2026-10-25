import QtQuick
import Quickshell
import Quickshell.Io

// Which of these cards a desktop widget layer has taken over.
//
// Homescreen, where it is installed, can place the launch card and the
// weather card as widgets, to be moved like any other. A card it has placed
// is drawn there, and the one in the corner stands down so that it is not on
// the desktop twice. Where Homescreen is not installed, or has neither card
// placed, nothing here changes anything.
Scope {
  id: root

  // The widgets placed on any screen, by id.
  property var ids: []

  function has(id) {
    return root.ids.indexOf(id) >= 0;
  }

  function take(text) {
    var found = [];
    try {
      var screens = JSON.parse(text).screens || {};
      for (var name in screens) {
        var placed = screens[name];
        for (var i = 0; i < placed.length && i < 256; i++) {
          if (placed[i] && typeof placed[i].widget === "string")
            found.push(placed[i].widget);
        }
      }
    } catch (error) {
      // No layout, or not one: nothing is placed.
    }
    root.ids = found;
  }

  FileView {
    id: layout

    path: Quickshell.env("HOME") + "/.config/homescreen/layout.json"
    printErrors: false
    watchChanges: true
    onFileChanged: reload()
    onLoaded: root.take(text())
    onLoadFailed: root.ids = []
  }
}

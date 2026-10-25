pragma ComponentBehavior: Bound

import QtQuick
import Quickshell
import Quickshell.Wayland

// The toasts on every screen the settings choose, at the top right under the
// bar, the newest at the top.
//
// Each toast is a surface of its own, so the glass engine sees a pane that
// keeps its shape: it comes in and goes by Hyprland's slide (the layer rule
// for hyprland-shell:notifs in hypr/vetro.lua), and when one above it goes
// the rest glide up by their surfaces' margins, the card standing still in
// each. A card moving inside a larger surface had the engine build its pane
// again on every frame it could, and the glass trailed the card.
Scope {
  id: layer

  // At most this many at once; the rest wait their turn.
  property int most: 5
  readonly property real gap: 8

  Variants {
    model: Settings.screens

    Scope {
      id: place

      required property ShellScreen modelData
      // The height of each toast up on this screen, by its id.
      property var heights: ({})
      // How many of the pointer's visits are on, so the toasts' time is held
      // once for this screen and let go once.
      property int over: 0
      onOverChanged: {
        if (over === 1)
          Notifications.hold();
        else if (over === 0)
          Notifications.release();
      }
      Component.onDestruction: {
        if (over > 0)
          Notifications.release();
      }

      readonly property var shown: Notifications.toasts.slice(0, layer.most)

      function above(note) {
        let total = 0;
        for (const other of shown) {
          if (other === note)
            break;
          total += (heights[other.id] ?? 0) + layer.gap;
        }
        return total;
      }

      Variants {
        model: place.shown

        PanelWindow {
          id: window

          required property var modelData

          screen: place.modelData
          color: "transparent"
          WlrLayershell.namespace: "hyprland-shell:notifs"
          WlrLayershell.layer: WlrLayer.Overlay
          WlrLayershell.keyboardFocus: WlrKeyboardFocus.None
          exclusiveZone: 0
          implicitWidth: toast.implicitWidth
          implicitHeight: toast.implicitHeight

          anchors {
            top: true
            right: true
          }
          margins {
            top: Settings.sideGap + window.offset
            right: Settings.sideGap
          }

          // Where it sits under the ones above it, glided to.
          property real offset: place.above(modelData)
          Behavior on offset {
            Glide {}
          }

          Toast {
            id: toast
            anchors.fill: parent
            note: window.modelData

            onImplicitHeightChanged: window.tell()
            onHoveredChanged: place.over += hovered ? 1 : -1
          }

          function tell() {
            const heights = Object.assign({}, place.heights);
            heights[modelData.id] = toast.implicitHeight;
            place.heights = heights;
          }
          Component.onCompleted: tell()
          Component.onDestruction: {
            if (toast.hovered)
              place.over -= 1;
            const heights = Object.assign({}, place.heights);
            delete heights[modelData.id];
            place.heights = heights;
          }
        }
      }
    }
  }
}

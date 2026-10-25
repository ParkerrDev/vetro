import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Wayland

// The lab's stand-in surfaces, built from a scene file so that the reference
// page can be given exactly the same shapes. Every length is in logical pixels.
ShellRoot {
  id: root
  property string scenePath: Quickshell.env("LAB_SCENE")
  property string dir: Quickshell.env("LAB_DIR")
  property var scene: ({ layers: [], bg: "", text: "#ffffff", pan: 0 })

  FileView {
    id: sceneFile
    path: root.scenePath
    blockLoading: true
    watchChanges: true
    onFileChanged: reload()
    onLoaded: root.scene = JSON.parse(text())
  }

  function rgba(c) { return Qt.rgba(c[0], c[1], c[2], c[3]); }

  // The wallpaper. "pan" slides it back and forth so that what lies behind
  // the glass changes on every frame, as a video wallpaper does.
  PanelWindow {
    color: "#000000"
    exclusionMode: ExclusionMode.Ignore
    WlrLayershell.namespace: "lab:wallpaper"
    WlrLayershell.layer: WlrLayer.Background
    anchors { top: true; bottom: true; left: true; right: true }
    Image {
      id: wall
      width: parent.width + 2 * (root.scene.pan || 0)
      height: parent.height
      x: -(root.scene.pan || 0)
      source: root.scene.bg ? "file://" + root.dir + "/bg/" + root.scene.bg : ""
      fillMode: Image.PreserveAspectCrop
      // nearest: a picture the size of the screen then lands on it pixel for pixel, whatever tiny
      // offset a session gives it, so that two runs of one build draw the same wallpaper
      smooth: false
      mipmap: false
      SequentialAnimation on x {
        running: (root.scene.pan || 0) > 0
        loops: Animation.Infinite
        NumberAnimation { from: -(root.scene.pan || 0); to: 0; duration: 4000 }
        NumberAnimation { from: 0; to: -(root.scene.pan || 0); duration: 4000 }
      }
    }
  }

  component Shape: Item {
    id: shape
    required property var modelData
    x: modelData.x; y: modelData.y; width: modelData.w; height: modelData.h
    // A soft shadow drawn by the client itself, fainter than the mask threshold.
    Rectangle {
      visible: (shape.modelData.shadow || 0) > 0
      anchors.fill: parent
      anchors.margins: -10
      radius: (shape.modelData.r || 0) + 10
      color: Qt.rgba(0, 0, 0, shape.modelData.shadow || 0)
    }
    Rectangle {
      visible: shape.modelData.kind !== "squircle"
      anchors.fill: parent
      radius: Math.min(shape.modelData.r || 0, width / 2, height / 2)
      color: root.rgba(shape.modelData.fill)
      border.width: shape.modelData.border ? 1 : 0
      border.color: shape.modelData.border ? root.rgba(shape.modelData.border) : "transparent"
      antialiasing: true
    }
    // A superellipse corner (Hyprland's rounding_power), which a Rectangle cannot draw.
    Canvas {
      visible: shape.modelData.kind === "squircle"
      anchors.fill: parent
      onPaint: {
        const ctx = getContext("2d");
        ctx.reset();
        const w = width, h = height, r = Math.min(shape.modelData.r || 0, w / 2, h / 2), n = shape.modelData.n || 4;
        const corner = (cx, cy, sx, sy, a0) => {
          for (let i = 0; i <= 24; i++) {
            const a = a0 + (i / 24) * Math.PI / 2;
            const c = Math.cos(a), s = Math.sin(a);
            const px = Math.sign(c) * Math.pow(Math.abs(c), 2 / n), py = Math.sign(s) * Math.pow(Math.abs(s), 2 / n);
            ctx.lineTo(cx + r * px, cy + r * py);
          }
        };
        ctx.beginPath();
        ctx.moveTo(w, h - r);
        corner(w - r, h - r, 1, 1, 0);
        corner(r, h - r, -1, 1, Math.PI / 2);
        corner(r, r, -1, -1, Math.PI);
        corner(w - r, r, 1, -1, 3 * Math.PI / 2);
        ctx.closePath();
        const f = shape.modelData.fill;
        ctx.fillStyle = Qt.rgba(f[0], f[1], f[2], f[3]);
        ctx.fill();
      }
    }
    Column {
      anchors.fill: parent
      anchors.margins: shape.modelData.pad || 18
      spacing: 8
      Text { visible: !!shape.modelData.title; text: shape.modelData.title || ""; color: root.scene.text; font.pixelSize: 13; font.letterSpacing: 2; opacity: 0.7 }
      Text { visible: !!shape.modelData.big; text: shape.modelData.big || ""; color: root.scene.text; font.pixelSize: 34; font.weight: Font.Light }
      Repeater {
        model: shape.modelData.rows || []
        Rectangle {
          required property var modelData
          required property int index
          width: parent.width; height: 34; radius: 9
          color: index === (shape.modelData.selected === undefined ? -1 : shape.modelData.selected) ? Qt.rgba(1, 1, 1, 0.16) : "transparent"
          Text { anchors.verticalCenter: parent.verticalCenter; x: 12; text: parent.modelData; color: root.scene.text; font.pixelSize: 15 }
        }
      }
    }
    Text { visible: !!shape.modelData.label; anchors.centerIn: parent; text: shape.modelData.label || ""; color: root.scene.text; font.pixelSize: 14; font.letterSpacing: 1.5 }
    // A counter that redraws a small part of the layer several times a second,
    // as a clock or a typed search would, while nothing behind the layer changes.
    Text {
      id: ticker
      property int count: 0
      visible: (shape.modelData.tick || 0) > 0
      anchors { right: parent.right; bottom: parent.bottom; margins: 14 }
      text: "tick " + count
      color: root.scene.text
      font.pixelSize: 16
      Timer { interval: shape.modelData.tick || 1000; running: (shape.modelData.tick || 0) > 0; repeat: true; onTriggered: ticker.count += 1 }
    }
  }

  Variants {
    model: root.scene.layers
    PanelWindow {
      id: win
      required property var modelData
      property bool full: !!modelData.full
      color: "transparent"
      exclusionMode: ExclusionMode.Ignore
      WlrLayershell.namespace: modelData.ns
      WlrLayershell.layer: modelData.overlay ? WlrLayer.Overlay : WlrLayer.Top
      anchors { top: true; left: true; bottom: win.full; right: win.full }
      margins { top: win.full ? 0 : win.modelData.y; left: win.full ? 0 : win.modelData.x }
      implicitWidth: win.full ? 0 : modelData.w
      implicitHeight: win.full ? 0 : modelData.h
      // A scrim over the whole surface, fainter than the mask threshold.
      Rectangle { anchors.fill: parent; color: Qt.rgba(0, 0, 0, win.modelData.scrim || 0); visible: (win.modelData.scrim || 0) > 0 }
      Repeater { model: win.modelData.shapes; Shape {} }
    }
  }
}

import QtQuick
import QtQuick.Effects

// Frosted glass: a blurred copy of `stage` behind this item, cut to a
// rounded rectangle. The login screen is a single surface, so Hyprland's
// own blur can't reach the video underneath; this does it in-scene.
Item {
  id: root

  required property Item stage
  property real radius: 16
  property real blurAmount: 1.0
  // Sample a margin around the item so the blur has real pixels at its edges.
  readonly property real pad: 40

  ShaderEffectSource {
    id: behind

    sourceItem: root.stage
    sourceRect: {
      // Re-evaluate when this item moves or resizes.
      root.x;
      root.y;
      root.width;
      root.height;
      const p = root.mapToItem(root.stage, 0, 0);
      return Qt.rect(p.x - root.pad, p.y - root.pad, root.width + 2 * root.pad, root.height + 2 * root.pad);
    }
    live: true
    visible: false
  }

  Item {
    id: mask

    width: root.width + 2 * root.pad
    height: root.height + 2 * root.pad
    visible: false
    layer.enabled: true

    Rectangle {
      x: root.pad
      y: root.pad
      width: root.width
      height: root.height
      radius: root.radius
    }
  }

  MultiEffect {
    x: -root.pad
    y: -root.pad
    width: root.width + 2 * root.pad
    height: root.height + 2 * root.pad
    source: behind
    autoPaddingEnabled: false
    blurEnabled: true
    blur: root.blurAmount
    blurMax: 48
    saturation: 0.15
    maskEnabled: true
    maskSource: mask
  }
}

import QtQuick
import QtQuick.Effects

// Liquid glass the shell draws itself (glass.frag), for a surface the glass
// engine never sees: the lock screen's. It lies over `backdrop`, an item that
// is a texture (a layer), with `blurred` a blurred copy of it in the same
// place, and it reads them where it is: keep it a child of something that does
// not move, beside the backdrop, so its place in it stays known.
//
// Without an `outline` it is a rounded box, a capsule by default; with one, it
// is that item's alpha (the clock's numerals, say), drawn the size of this one.
// Whatever it holds is drawn on the glass.
ShaderEffect {
  id: glass

  property Item backdrop: null
  property Item blurred: null
  property Item outline: null

  // The box: its corners, and room round it for the shadow.
  property real margin: 0
  property real radius: Math.max(0, Math.min(width, height) / 2 - margin)

  // The glass, clear with a little frost and white, lit from a little left of
  // straight above.
  property real bezel: 16
  property real refraction: 20
  property real dispersion: 0.3
  property real frost: 0.2
  property color tint: "#ffffff"
  property real tintAmount: 0.1
  property real saturation: 1.4
  property real lightAngle: -32 * Math.PI / 180
  property real glare: 0.65
  property real opposite: 0.45
  property real sharpness: 2.2
  property real hairline: 0.5
  property real shadow: 0.2

  // Anything else that moves it (a transform, say): bind it here, and the
  // glass reads from where it has gone.
  property real moved: 0

  // Where it is in the backdrop, worked out again as it moves or grows.
  readonly property rect place: {
    x; y; width; height; scale; moved; parent?.x; parent?.y;
    return backdrop ? mapToItem(backdrop, 0, 0, width, height) : Qt.rect(0, 0, 1, 1);
  }
  readonly property vector4d area: backdrop && backdrop.width > 0 && backdrop.height > 0
    ? Qt.vector4d(place.x / backdrop.width, place.y / backdrop.height, place.width / backdrop.width, place.height / backdrop.height)
    : Qt.vector4d(0, 0, 1, 1)
  readonly property size extent: Qt.size(width, height)
  readonly property vector2d texel: Qt.vector2d(1 / Math.max(1, width), 1 / Math.max(1, height))
  readonly property real shaped: outline ? 1 : 0
  readonly property real pixel: 1 / Math.max(1, Screen.devicePixelRatio)
  readonly property Item shape: outline ?? backdrop
  readonly property Item soft: outline ? softened : backdrop

  fragmentShader: Qt.resolvedUrl("glass.frag.qsb")

  // The outline blurred: its slope is the direction of the edge, its height
  // how deep into the glass a point is.
  MultiEffect {
    id: softened
    width: glass.width
    height: glass.height
    visible: false
    layer.enabled: glass.outline !== null
    source: glass.outline
    autoPaddingEnabled: false
    blurEnabled: true
    blur: 1
    blurMax: Math.max(2, Math.round(glass.bezel * 1.5))
  }
}

import QtQuick

// A Material Symbols icon by its name ("notifications", "volume_up"), in the
// rounded style, at a size in pixels.
Text {
  property real size: 18
  property bool filled: false

  color: Look.text
  font.family: Look.iconFamily
  font.pixelSize: size
  font.variableAxes: { "FILL": filled ? 1 : 0, "opsz": Math.max(20, Math.min(48, size)) }
  renderType: Text.QtRendering
  horizontalAlignment: Text.AlignHCenter
  verticalAlignment: Text.AlignVCenter
}

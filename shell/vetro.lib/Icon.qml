import QtQuick

// An icon by its name ("notifications", "volume_up"), drawn as the Nerd Font
// glyph Omarchy uses for it, in Omarchy's own font, at a size in pixels. A
// glyph given as the name is drawn as it is.
Text {
  property string name: ""
  property real size: 18
  property bool filled: false

  text: Look.glyph(name, filled)
  color: Look.text
  font.family: Look.iconFamily
  font.pixelSize: size
  renderType: Text.QtRendering
  horizontalAlignment: Text.AlignHCenter
  verticalAlignment: Text.AlignVCenter
}

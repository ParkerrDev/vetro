import QtQuick

// Type in Vetro's look: D-DIN in the panels' ink, at the shell's size.
Text {
  color: Look.text
  font.family: Look.family
  font.pointSize: Look.normal
  renderType: Look.nativeRendering ? Text.NativeRendering : Text.QtRendering
  elide: Text.ElideRight
  textFormat: Text.PlainText
}

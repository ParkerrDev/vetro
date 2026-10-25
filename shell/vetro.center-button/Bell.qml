import QtQuick
import qs.Commons
import qs.Ui
import "../vetro.lib"

// The bell for Vetro's notification center, as a widget for Omarchy's bar.
// Left click opens the center, right click is Do Not Disturb.
BarWidget {
  id: root
  moduleName: "vetro.center-button"

  readonly property int waiting: Notifications.all.length

  implicitWidth: button.implicitWidth
  implicitHeight: button.implicitHeight

  BarIconButton {
    id: button
    anchors.fill: parent
    bar: root.bar
    // nf-md-bell_off / bell_badge / bell_outline
    text: Notifications.dnd ? "󰂛" : (root.waiting > 0 ? "󱅫" : "󰂜")
    tooltipText: Notifications.dnd ? "Do Not Disturb" : (root.waiting === 1 ? "1 notification" : root.waiting + " notifications")
    onPressed: function (b) {
      if (b === Qt.RightButton)
        Notifications.setDnd(!Notifications.dnd);
      else if (root.bar)
        root.bar.run("omarchy-shell shell toggle vetro.center '{}'");
    }
  }
}

import QtQuick
import QtQuick.Effects
import QtQuick.Layouts

// The next-launch card, styled after SpaceX's webcast graphics: white type
// and hairlines on clear glass. Presentation only; the desktop overlay and
// the login screen hand it a LaunchData.
//
// The card draws a light tint of its own. Blur comes from whoever hosts it:
// a Hyprland layer rule on the desktop, Frosted.qml on the login screen.
Item {
  id: root

  required property var launchData
  property int rows: 3
  property real radius: 8
  property real basePointSize: 10
  property bool use12h: false
  property string muteHint: "Super+Shift+M"
  // The glass tint. Hosts over busier backdrops can make it darker.
  property color glassColor: Style.glass
  // A soft shadow under the type, for bright skies and smoke.
  property bool textShadow: true

  readonly property var launch: launchData.next
  readonly property bool live: launchData.live
  readonly property real now: launchData.now
  readonly property string precision: launch ? (launch.net_precision || "") : ""
  readonly property bool exactTime: !!launch && ["SEC", "MIN", ""].indexOf(precision) >= 0
  readonly property real pad: 18

  implicitWidth: 384
  implicitHeight: body.implicitHeight + 2 * pad

  function pt(k) {
    return basePointSize * k;
  }

  function rocketName(l) {
    return (l.rocket || "").replace(/\s+Block\s+\d+$/i, "");
  }

  function two(n) {
    return (n < 10 ? "0" : "") + n;
  }

  function clock(epoch) {
    return Qt.formatDateTime(new Date(epoch * 1000), use12h ? "h:mm AP" : "HH:mm");
  }

  function zone(epoch) {
    return Qt.formatDateTime(new Date(epoch * 1000), "t");
  }

  // "Thu Oct 1", or coarser when Launch Library only knows the month or
  // the quarter.
  function netDate(l) {
    const d = new Date(l.net_epoch * 1000);
    switch (l.net_precision || "") {
    case "MONTH":
      return Qt.formatDateTime(d, "MMMM yyyy");
    case "Q1":
    case "Q2":
    case "Q3":
    case "Q4":
      return l.net_precision + " " + Qt.formatDateTime(d, "yyyy");
    case "H1":
    case "H2":
    case "YEAR":
    case "FY":
      return (l.net_precision === "YEAR" ? "" : l.net_precision + " ") + Qt.formatDateTime(d, "yyyy");
    default:
      return Qt.formatDateTime(d, "ddd MMM d");
    }
  }

  // [{v: "04", l: "hrs"}, ...], days only when there are some.
  function countdown(epoch) {
    let d = Math.floor(epoch - now);
    const sign = d > 0 ? "T−" : "T+";
    d = Math.abs(d);
    const days = Math.floor(d / 86400);
    d %= 86400;
    const groups = [];
    if (days > 0)
      groups.push({
        v: two(days),
        l: days === 1 ? "day" : "days"
      });
    groups.push({
      v: two(Math.floor(d / 3600)),
      l: "hrs"
    });
    groups.push({
      v: two(Math.floor(d % 3600 / 60)),
      l: "min"
    });
    groups.push({
      v: two(d % 60),
      l: "sec"
    });
    return {
      sign: sign,
      groups: groups
    };
  }

  function ago(epoch) {
    const s = now - epoch;
    if (s < 3600)
      return Math.max(1, Math.round(s / 60)) + " min ago";
    if (s < 86400)
      return Math.round(s / 3600) + " h ago";
    const days = Math.round(s / 86400);
    return days + (days == 1 ? " day ago" : " days ago");
  }

  // Launch Library status -> the tag in the corner. Failures and holds get
  // a solid white outline; everything else a softer one.
  function statusTag(l) {
    if (!l)
      return "";
    const map = {
      "Go": "Go",
      "TBC": "TBC",
      "TBD": "TBD",
      "Hold": "Hold",
      "In Flight": "In flight",
      "Success": "Success",
      "Failure": "Failure",
      "Partial Failure": "Partial"
    };
    return map[l.status] || l.status || "";
  }

  readonly property var cd: launch ? countdown(launch.net_epoch) : null
  readonly property string tag: statusTag(launch)
  readonly property bool alarm: !!launch && ["Hold", "Failure", "Partial Failure"].indexOf(launch.status) >= 0

  // Small capitals in D-DIN, the label style used everywhere on the card.
  component Caps: Text {
    property real size: root.pt(0.82)

    color: Style.dim
    font.family: Style.family
    font.pointSize: size
    font.capitalization: Font.AllUppercase
    font.letterSpacing: size * Style.capsTracking
    elide: Text.ElideRight
  }

  // Glass
  Rectangle {
    anchors.fill: parent
    radius: root.radius
    color: root.glassColor
    border.width: 1
    border.color: Style.glassEdge
  }


  ColumnLayout {
    id: body

    spacing: 0
    layer.enabled: root.textShadow
    layer.effect: MultiEffect {
      shadowEnabled: true
      shadowColor: "black"
      shadowOpacity: 0.45
      shadowBlur: 0.35
      shadowVerticalOffset: 1
      shadowHorizontalOffset: 0
    }

    anchors {
      left: parent.left
      right: parent.right
      top: parent.top
      margins: root.pad
    }

    // Header: what this is, plus the Launch Library status.
    RowLayout {
      Layout.fillWidth: true
      spacing: 8

      Rectangle {
        visible: root.live
        implicitWidth: 7
        implicitHeight: 7
        radius: 3.5
        color: Style.live

        SequentialAnimation on opacity {
          running: root.live
          loops: Animation.Infinite

          NumberAnimation {
            from: 1
            to: 0.2
            duration: 800
            easing.type: Easing.InOutSine
          }

          NumberAnimation {
            from: 0.2
            to: 1
            duration: 800
            easing.type: Easing.InOutSine
          }
        }
      }

      Caps {
        Layout.fillWidth: true
        text: root.live ? "Live" : "Upcoming launch"
        color: root.live ? Style.text : Style.dim
        font.weight: Font.Bold
      }

      Rectangle {
        visible: root.tag !== "" && !root.live
        implicitWidth: tagText.implicitWidth + 14
        implicitHeight: tagText.implicitHeight + 6
        radius: 3
        color: "transparent"
        border.width: 1
        border.color: root.alarm ? Style.text : Qt.rgba(1, 1, 1, 0.55)

        Caps {
          id: tagText

          anchors.centerIn: parent
          text: root.tag
          color: Style.text
          size: root.pt(0.72)
          font.weight: Font.Bold
        }
      }
    }

    // Mission
    Text {
      Layout.fillWidth: true
      Layout.topMargin: 10
      text: root.launch ? root.launch.title : "Waiting for launch data"
      color: Style.text
      font.family: Style.family
      font.pointSize: root.pt(1.75)
      font.weight: Font.Bold
      font.capitalization: Font.AllUppercase
      font.letterSpacing: root.pt(1.75) * 0.08
      elide: Text.ElideRight
    }

    Caps {
      Layout.fillWidth: true
      Layout.topMargin: 3
      visible: !!root.launch
      text: root.launch ? [root.rocketName(root.launch), root.launch.pad_short || root.launch.pad, root.launch.location_short || root.launch.location].filter(s => !!s).join("  ·  ") : ""
      size: root.pt(0.78)
    }

    // The countdown, or the no-earlier-than date when the time isn't set.
    RowLayout {
      Layout.topMargin: 14
      visible: !!root.launch
      spacing: 0

      Text {
        Layout.alignment: Qt.AlignTop
        Layout.rightMargin: 10
        Layout.topMargin: countdownMetrics.height * 0.14
        text: root.exactTime && root.cd ? root.cd.sign : "NET"
        color: Style.dim
        font.family: Style.family
        font.pointSize: root.pt(1.25)
        font.weight: Font.Bold
      }

      Repeater {
        model: root.exactTime && root.cd ? root.cd.groups : []

        RowLayout {
          required property var modelData
          required property int index

          Layout.alignment: Qt.AlignTop
          spacing: 0

          Text {
            Layout.alignment: Qt.AlignTop
            visible: index > 0
            text: ":"
            color: Style.faint
            font.family: Style.wide
            font.pointSize: root.pt(2.5)
            leftPadding: 3
            rightPadding: 3
            // Nudge the colon up a hair so it sits between the digits' tops
            // and baselines rather than on the baseline.
            topPadding: -countdownMetrics.height * 0.04
          }

          ColumnLayout {
            Layout.alignment: Qt.AlignTop
            spacing: 1

            Text {
              Layout.preferredWidth: countdownMetrics.width
              horizontalAlignment: Text.AlignHCenter
              text: modelData.v
              color: Style.text
              font.family: Style.wide
              font.pointSize: root.pt(2.5)
              font.features: {
                "tnum": 1
              }
            }

            Caps {
              Layout.alignment: Qt.AlignHCenter
              text: modelData.l
              color: Style.dim
              size: root.pt(0.62)
            }
          }
        }
      }

      Text {
        visible: !root.exactTime
        text: root.launch ? root.netDate(root.launch) : ""
        color: Style.text
        font.family: Style.wide
        font.pointSize: root.pt(1.9)
        font.capitalization: Font.AllUppercase
        font.letterSpacing: root.pt(1.9) * 0.06
      }
    }

    // When, weather, orbit. Or during a launch: whose webcast, and sound.
    Caps {
      Layout.fillWidth: true
      Layout.topMargin: 12
      visible: !!root.launch
      text: {
        if (!root.launch)
          return "";
        if (root.live) {
          const w = root.launchData.wallpaper.live || {};
          const sound = root.launchData.muted ? (root.muteHint ? "muted, " + root.muteHint + " for sound" : "muted") : "sound on";
          return "Webcast" + (w.publisher ? " · " + w.publisher : "") + "  ·  " + sound;
        }
        const parts = [];
        if (root.exactTime)
          parts.push(root.netDate(root.launch) + "  " + root.clock(root.launch.net_epoch) + " " + root.zone(root.launch.net_epoch));
        if (root.launch.probability !== null && root.launch.probability !== undefined && root.launch.probability >= 0)
          parts.push(root.launch.probability + "% weather go");
        if (root.launch.orbit)
          parts.push(root.launch.orbit);
        return parts.join("  ·  ");
      }
    }

    Rectangle {
      Layout.fillWidth: true
      Layout.topMargin: 14
      Layout.bottomMargin: 8
      visible: laterRepeater.count > 0
      implicitHeight: 1
      color: Style.hairline
    }

    Repeater {
      id: laterRepeater

      model: root.launchData.later.slice(0, root.rows)

      RowLayout {
        required property var modelData

        Layout.fillWidth: true
        Layout.topMargin: 2
        Layout.bottomMargin: 2
        spacing: 12

        Caps {
          Layout.preferredWidth: dateMetrics.width
          text: Qt.formatDateTime(new Date(modelData.net_epoch * 1000), "MMM d") + "  " + root.clock(modelData.net_epoch)
          color: Style.faint
          size: root.pt(0.74)
          font.features: {
            "tnum": 1
          }
        }

        Caps {
          Layout.fillWidth: true
          text: modelData.title
          color: Style.text
          size: root.pt(0.74)
        }

        Caps {
          text: modelData.rocket_short
          color: Style.faint
          size: root.pt(0.74)
          font.weight: Font.Bold
        }
      }
    }

    RowLayout {
      Layout.fillWidth: true
      Layout.topMargin: laterRepeater.count > 0 ? 10 : 14
      visible: !!root.launchData.last && !root.live
      spacing: 0

      Caps {
        text: "Last  "
        color: Style.faint
        size: root.pt(0.68)
        font.weight: Font.Bold
      }

      Caps {
        Layout.fillWidth: true
        text: root.launchData.last ? root.launchData.last.title : ""
        color: Style.faint
        size: root.pt(0.68)
      }

      Caps {
        text: {
          const l = root.launchData.last;
          return l ? "  ·  " + (l.status || "").toLowerCase() + "  ·  " + root.ago(l.net_epoch) : "";
        }
        color: Style.faint
        size: root.pt(0.68)
      }
    }
  }

  TextMetrics {
    id: countdownMetrics

    font.family: Style.wide
    font.pointSize: root.pt(2.5)
    text: "00"
  }

  TextMetrics {
    id: dateMetrics

    font.family: Style.family
    font.pointSize: root.pt(0.74)
    font.capitalization: Font.AllUppercase
    font.letterSpacing: root.pt(0.74) * Style.capsTracking
    text: "SEP 30  " + (root.use12h ? "12:00 PM" : "00:00")
  }
}

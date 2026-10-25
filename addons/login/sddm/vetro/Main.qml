import QtQuick
import QtQuick.Effects

// Vetro's login screen as an SDDM theme, for Omarchy: the lock screen
// (shell/vetro.lock/LockView.qml), element for element, so logging in and
// unlocking look the same. SDDM loads it once per monitor: the primary one
// gets the name and the password field, the others the still and the clock.
//
// SDDM hands the theme `sddm` (log in, restart, shut down), `userModel`,
// `sessionModel` and `config` (theme.conf). The launch clip, the launch list
// and your avatar come from /var/lib/vetro, which vetro-launchd keeps
// fresh from your session.
Rectangle {
  id: win

  width: 1600
  height: 1000
  color: "black"

  readonly property bool primary: typeof primaryScreen === "undefined" ? true : primaryScreen
  readonly property string shared: config.shared || "/var/lib/vetro"
  readonly property bool use12h: String(config.use12h) === "true"
  property date now: new Date()

  Timer {
    interval: 1000
    running: true
    repeat: true
    onTriggered: win.now = new Date()
  }

  LaunchData {
    id: launchSource

    launchesPath: win.shared + "/launches.json"
  }

  // -- what LoginPanel talks to: greeter/shell's Auth and Sessions, on SDDM

  // The people who can log in, for the name over the password field
  Item {
    id: users

    property var list: []

    function realName(name) {
      const u = list.find(x => x && x.name === name);
      return u ? u.realName : "";
    }

    Repeater {
      model: userModel

      Item {
        Component.onCompleted: {
          const l = users.list.slice();
          l[index] = {
            name: model.name || "",
            realName: model.realName || ""
          };
          users.list = l;
        }
      }
    }
  }

  // Installed sessions; Omarchy's own (run by uwsm) first, as its theme does
  Item {
    id: sessionList

    property var list: []
    property int index: 0
    readonly property var current: list.length ? list[Math.min(index, list.length - 1)] : null

    function next() {
      if (list.length)
        index = (index + 1) % list.length;
    }

    Repeater {
      model: sessionModel

      Item {
        Component.onCompleted: {
          const l = sessionList.list.slice();
          l.push({
            name: model.name || "",
            sddmIndex: index
          });
          l.sort((a, b) => a.sddmIndex - b.sddmIndex);
          sessionList.list = l;
          const uwsm = l.findIndex(s => s.name.indexOf("uwsm") !== -1);
          const last = l.findIndex(s => s.sddmIndex === sessionModel.lastIndex);
          sessionList.index = uwsm >= 0 ? uwsm : Math.max(0, last);
        }
      }
    }
  }

  QtObject {
    id: authFlow

    // The last one to log in; on a machine nobody has logged in to through
    // this screen yet, the first (the only one, on this laptop)
    property string user: userModel.lastUser || (users.list.length && users.list[0] ? users.list[0].name : "")
    property bool busy: false
    property bool launching: false
    property string message: ""
    property bool isError: false
    // SDDM asks for the password and nothing else
    property string prompt: ""
    property bool promptEcho: false

    signal failed

    function submit(text) {
      if (busy || launching)
        return;
      if (user === "") {
        message = "Type a user name first";
        isError = true;
        failed();
        return;
      }
      message = "";
      isError = false;
      busy = true;
      sddm.login(user, text, sessionList.current ? sessionList.current.sddmIndex : sessionModel.lastIndex);
    }
  }

  Connections {
    target: sddm

    function onLoginFailed() {
      authFlow.busy = false;
      if (!(authFlow.isError && authFlow.message !== ""))
        authFlow.message = "That password didn't work";
      authFlow.isError = true;
      authFlow.failed();
    }

    function onLoginSucceeded() {
      authFlow.busy = false;
      authFlow.launching = true;
      authFlow.message = "";
    }

    function onInformationMessage(message) {
      authFlow.message = message;
    }
  }

  // -- the screen: LockView.qml's, with SDDM's data behind it

  // hyprlock sizes are Pango points: 4/3 px each
  function pt(points) {
    return Math.round(points * 4 / 3);
  }

  readonly property string nextLaunch: {
    const t = win.now.getTime() / 1000;
    const two = n => String(n).padStart(2, "0");
    for (const l of (launchSource.launches.upcoming || [])) {
      const net = l.net_epoch;
      if (!net || t - net > 20 * 60)
        continue;
      let when;
      if (["SEC", "MIN", ""].includes(l.net_precision || "")) {
        let left = Math.floor(Math.abs(net - t));
        const days = Math.floor(left / 86400);
        left %= 86400;
        const clock = two(Math.floor(left / 3600)) + ":" + two(Math.floor(left % 3600 / 60));
        when = (net > t ? "T− " : "T+ ") + (days ? days + "D " : "") + clock;
      } else {
        when = "NET " + Qt.formatDate(new Date(net * 1000), "MMM d").toUpperCase();
      }
      return (l.title || "").toUpperCase() + "   ·   " + when;
    }
    return "";
  }

  // The still of the launch the wallpaper plays, behind blur
  Image {
    id: still
    anchors.fill: parent
    visible: false
    source: "file://" + win.shared + "/poster.jpg"
    fillMode: Image.PreserveAspectCrop
    asynchronous: true
    cache: false
    sourceSize.width: width
    sourceSize.height: height
  }

  MultiEffect {
    anchors.fill: still
    source: still
    visible: still.status === Image.Ready
    autoPaddingEnabled: false
    blurEnabled: true
    blur: 1.0
    blurMax: 48
    saturation: 0.05
  }

  // brightness 0.55
  Rectangle {
    anchors.fill: parent
    color: "#000000"
    opacity: 0.45
  }

  MouseArea {
    anchors.fill: parent
    hoverEnabled: true
    cursorShape: Qt.BlankCursor
    onClicked: passwordInput.forceActiveFocus()
  }

  // Clock: big D-DIN numerals
  Text {
    anchors.horizontalCenter: parent.horizontalCenter
    anchors.verticalCenter: parent.verticalCenter
    anchors.verticalCenterOffset: -250
    text: Qt.formatTime(win.now, win.use12h ? "h:mm" : "HH:mm")
    color: "#ffffff"
    font.family: Style.wide
    font.pixelSize: win.pt(110)
    layer.enabled: true
    layer.effect: MultiEffect {
      shadowEnabled: true
      shadowColor: Qt.rgba(0, 0, 0, 0.45)
      shadowBlur: 0.6
      shadowVerticalOffset: 0
      shadowHorizontalOffset: 0
    }
  }

  // The date in spaced capitals
  Text {
    anchors.horizontalCenter: parent.horizontalCenter
    anchors.verticalCenter: parent.verticalCenter
    anchors.verticalCenterOffset: -160
    text: Qt.formatDate(win.now, "dddd, MMMM d").toUpperCase()
    color: Qt.rgba(1, 1, 1, 0.72)
    font.family: Style.family
    font.bold: true
    font.pixelSize: win.pt(14)
    font.letterSpacing: win.pt(4)
  }

  // Your picture
  Item {
    width: 84
    height: 84
    anchors.horizontalCenter: parent.horizontalCenter
    anchors.verticalCenter: parent.verticalCenter
    anchors.verticalCenterOffset: -30
    visible: win.primary && avatar.status === Image.Ready

    Image {
      id: avatar
      anchors.fill: parent
      visible: false
      source: "file://" + win.shared + "/avatar.jpg"
      fillMode: Image.PreserveAspectCrop
      sourceSize.width: 168
      sourceSize.height: 168
      cache: false
    }

    Rectangle {
      id: avatarMask
      anchors.fill: parent
      radius: width / 2
      visible: false
      layer.enabled: true
    }

    MultiEffect {
      anchors.fill: parent
      source: avatar
      maskEnabled: true
      maskSource: avatarMask
    }

    Rectangle {
      anchors.fill: parent
      radius: width / 2
      color: "transparent"
      border.width: 1
      border.color: Qt.rgba(1, 1, 1, 0.45)
    }
  }

  // Your name
  Text {
    visible: win.primary
    anchors.horizontalCenter: parent.horizontalCenter
    anchors.verticalCenter: parent.verticalCenter
    anchors.verticalCenterOffset: 45
    text: (users.realName(authFlow.user) || authFlow.user).toUpperCase()
    color: "#ffffff"
    font.family: Style.family
    font.bold: true
    font.pixelSize: win.pt(16)
    font.letterSpacing: win.pt(2)
  }

  // The hairline password field
  Rectangle {
    visible: win.primary
    width: 320
    height: 46
    anchors.horizontalCenter: parent.horizontalCenter
    anchors.verticalCenter: parent.verticalCenter
    anchors.verticalCenterOffset: 115
    radius: 4
    color: Qt.rgba(1, 1, 1, 0.07)
    border.width: 1
    border.color: authFlow.isError ? Qt.rgba(1, 1, 1, 0.95) : Qt.rgba(1, 1, 1, 0.35)
    clip: true

    HoverHandler {
      cursorShape: Qt.BlankCursor
    }

    TextInput {
      id: passwordInput
      anchors.fill: parent
      anchors.leftMargin: 16
      anchors.rightMargin: 16
      verticalAlignment: TextInput.AlignVCenter
      horizontalAlignment: TextInput.AlignHCenter
      activeFocusOnPress: true
      focus: win.primary
      clip: true
      enabled: !authFlow.busy && !authFlow.launching
      echoMode: TextInput.Password
      passwordCharacter: "●"
      passwordMaskDelay: 0
      color: "#ffffff"
      selectionColor: Qt.rgba(1, 1, 1, 0.3)
      selectedTextColor: "#ffffff"
      font.family: Style.family
      font.pixelSize: 11
      font.letterSpacing: 7
      cursorDelegate: Item {}

      onTextChanged: {
        if (text.length > 0 && authFlow.isError) {
          authFlow.isError = false;
          authFlow.message = "";
        }
      }

      onAccepted: {
        const submitted = text;
        text = "";
        if (submitted.length > 0)
          authFlow.submit(submitted);
      }

      Keys.onPressed: function (event) {
        if (event.key === Qt.Key_Escape || (event.modifiers & Qt.ControlModifier && event.key === Qt.Key_U)) {
          text = "";
          event.accepted = true;
        }
      }

      Component.onCompleted: if (win.primary)
        forceActiveFocus()
    }

    // PASSWORD, then Checking... or why it failed
    Text {
      anchors.fill: parent
      anchors.leftMargin: 12
      anchors.rightMargin: 12
      visible: passwordInput.text.length === 0
      text: authFlow.busy || authFlow.launching ? "CHECKING" : (authFlow.isError ? authFlow.message : "PASSWORD")
      color: authFlow.isError ? Qt.rgba(1, 1, 1, 0.95) : Qt.rgba(1, 1, 1, authFlow.busy ? 0.85 : 0.5)
      font.family: Style.family
      font.pixelSize: win.pt(11)
      font.letterSpacing: authFlow.isError ? 0 : win.pt(2)
      horizontalAlignment: Text.AlignHCenter
      verticalAlignment: Text.AlignVCenter
      elide: Text.ElideRight
    }
  }

  Connections {
    target: authFlow

    function onFailed() {
      passwordInput.forceActiveFocus();
    }
  }

  // The next SpaceX launch
  Text {
    visible: win.primary
    anchors.horizontalCenter: parent.horizontalCenter
    anchors.bottom: parent.bottom
    anchors.bottomMargin: 70
    text: win.nextLaunch
    color: Qt.rgba(1, 1, 1, 0.72)
    font.family: Style.family
    font.pixelSize: win.pt(12)
    font.letterSpacing: win.pt(3)
  }

  Text {
    visible: win.primary
    anchors.left: parent.left
    anchors.bottom: parent.bottom
    anchors.leftMargin: 32
    anchors.bottomMargin: 30
    text: "VETRO   ·   " + (sddm.hostName || "").toUpperCase()
    color: Qt.rgba(1, 1, 1, 0.5)
    font.family: Style.family
    font.bold: true
    font.pixelSize: win.pt(10)
    font.letterSpacing: win.pt(3)
  }
}

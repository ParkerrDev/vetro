import QtQuick
import QtQuick.Effects
import Quickshell
import Quickshell.Io
import "../vetro.lib"

// Vetro's lock screen on Omarchy's lock: the wallpaper, sharp, under a clock of
// clear glass; your picture in a ring of glass and your name; a capsule of
// glass for the password; and the next launch, when the launch addon has one.
// The glass is the shell's own (vetro.lib/Glass.qml): the glass engine never
// sees a lock surface. Everything is round and moves like a bubble, every
// length times motion.scale in the settings. Service.qml is Omarchy's own,
// unchanged; this view keeps the interface of its LockView.
Item {
  id: root

  // The interface Service.qml drives, as Omarchy's LockView has it.
  property string backgroundPath: ""
  property int backgroundVersion: 0
  property bool fingerprintConfigured: false
  property bool authenticatingPassword: false
  property string failureMessage: ""
  property int failedAttempts: 0
  property bool inputEnabled: true
  property bool loadBackground: true
  property string passwordText: ""
  property bool syncingPasswordText: false

  signal submitPassword(string password)
  signal passwordTextEdited(string password)
  signal clearFailureRequested()
  signal wakeRequested()

  readonly property string home: Quickshell.env("HOME")
  readonly property bool errorState: failureMessage.length > 0
  readonly property string rounded: "Nunito"

  function forcePasswordFocus() {
    passwordInput.forceActiveFocus();
  }

  function clearPassword() {
    passwordTextEdited("");
  }

  function syncPasswordText() {
    if (passwordInput.text === passwordText)
      return;
    syncingPasswordText = true;
    passwordInput.text = passwordText;
    syncingPasswordText = false;
  }

  function send() {
    const submitted = passwordText;
    passwordTextEdited("");
    if (submitted.length > 0)
      submitPassword(submitted);
  }

  // As Omarchy's lock does it: a query string makes Image load the background
  // again when another is picked while locked.
  function fileUrl(path) {
    if (!path)
      return "";
    return "file://" + String(path).split("/").map(encodeURIComponent).join("/") + "?v=" + backgroundVersion;
  }

  onPasswordTextChanged: syncPasswordText()
  onInputEnabledChanged: {
    if (inputEnabled)
      Qt.callLater(forcePasswordFocus);
  }
  Component.onCompleted: {
    syncPasswordText();
    if (inputEnabled)
      Qt.callLater(forcePasswordFocus);
    shown = true;
  }

  // Everything comes in once the view is up, on a spring.
  property bool shown: false

  // ------------------------------------------------------------ what it shows

  property var now: new Date()
  Timer {
    interval: 1000
    repeat: true
    running: root.loadBackground
    triggeredOnStart: true
    onTriggered: root.now = new Date()
  }

  // With the launch addon on, the desktop plays a launch: its still is what
  // was on the screen. Without it, the wallpaper.
  property string poster: ""
  FileView {
    path: root.home + "/.local/share/vetro/clips/current.json"
    printErrors: false
    watchChanges: true
    onFileChanged: reload()
    onLoaded: {
      try {
        root.poster = JSON.parse(text()).poster || "";
      } catch (e) {
        root.poster = "";
      }
    }
    onLoadFailed: root.poster = ""
  }

  // Your name from the passwd entry.
  property string fullName: ""
  Process {
    running: true
    command: ["getent", "passwd", Quickshell.env("USER") || ""]
    stdout: StdioCollector {
      onStreamFinished: root.fullName = (this.text.split(":")[4] || "").split(",")[0].trim()
    }
  }

  // The next launch, from the launch addon's cache.
  property var upcoming: []
  FileView {
    path: root.home + "/.cache/vetro/launches.json"
    printErrors: false
    watchChanges: true
    onFileChanged: reload()
    onLoaded: {
      try {
        root.upcoming = JSON.parse(text()).upcoming || [];
      } catch (e) {
        root.upcoming = [];
      }
    }
  }
  readonly property string nextLaunch: {
    const t = now.getTime() / 1000;
    const two = n => String(n).padStart(2, "0");
    for (const l of upcoming) {
      const net = l.net_epoch;
      if (!net || t - net > 20 * 60)
        continue;
      let when;
      if (["SEC", "MIN", ""].includes(l.net_precision || "")) {
        let left = Math.floor(Math.abs(net - t));
        const days = Math.floor(left / 86400);
        left %= 86400;
        const clock = two(Math.floor(left / 3600)) + ":" + two(Math.floor(left % 3600 / 60));
        when = (net > t ? "T− " : "T+ ") + (days ? days + "d " : "") + clock;
      } else {
        when = Qt.formatDate(new Date(net * 1000), "MMM d");
      }
      return (l.title || "") + "  ·  " + when;
    }
    return "";
  }

  // ------------------------------------------------------------- the glass

  // What the glass lies over and reads: the picture, sharp, a little darker
  // at the top and the bottom where the type is.
  Item {
    id: backdrop
    anchors.fill: parent
    layer.enabled: true

    Rectangle {
      anchors.fill: parent
      color: "#000000"
    }

    Image {
      id: picture
      anchors.fill: parent
      source: root.loadBackground ? root.fileUrl(root.poster || root.backgroundPath) : ""
      fillMode: Image.PreserveAspectCrop
      asynchronous: true
      cache: false
      sourceSize.width: width
      sourceSize.height: height
      opacity: status === Image.Ready ? 1 : 0
      Behavior on opacity {
        Fade {}
      }
    }

    Rectangle {
      anchors.fill: parent
      gradient: Gradient {
        GradientStop { position: 0; color: Qt.rgba(0, 0, 0, 0.26) }
        GradientStop { position: 0.4; color: Qt.rgba(0, 0, 0, 0.04) }
        GradientStop { position: 0.7; color: Qt.rgba(0, 0, 0, 0.04) }
        GradientStop { position: 1; color: Qt.rgba(0, 0, 0, 0.3) }
      }
    }
  }

  // The same, frosted, for the glass's frost.
  MultiEffect {
    id: frosted
    anchors.fill: backdrop
    source: backdrop
    visible: false
    layer.enabled: true
    autoPaddingEnabled: false
    blurEnabled: true
    blur: 1
    blurMax: 48
    saturation: 0.15
  }

  MouseArea {
    anchors.fill: parent
    hoverEnabled: true
    cursorShape: Qt.BlankCursor
    onClicked: {
      root.wakeRequested();
      root.forcePasswordFocus();
    }
    onPositionChanged: root.wakeRequested()
  }

  // ----------------------------------------------------------- the clock

  readonly property real head: Math.round(height * 0.06)

  // The date, small and white, over the clock.
  Text {
    id: date
    anchors.horizontalCenter: parent.horizontalCenter
    y: root.head + (root.shown ? 0 : -24)
    opacity: root.shown ? 1 : 0
    text: Qt.formatDate(root.now, "dddd, MMMM d")
    color: Qt.rgba(1, 1, 1, 0.92)
    font.family: root.rounded
    font.weight: Font.Bold
    font.pixelSize: Math.max(16, Math.round(root.height * 0.024))
    Behavior on y { Spring {} }
    Behavior on opacity { Fade {} }
    layer.enabled: true
    layer.effect: Lift {}
  }

  // The clock: big round numerals of clear glass.
  Glass {
    id: clock
    backdrop: backdrop
    blurred: frosted
    outline: digits
    anchors.horizontalCenter: parent.horizontalCenter
    y: root.head + date.implicitHeight - pad * 0.6 + (root.shown ? 0 : -40)
    width: digits.implicitWidth + 2 * pad
    height: digits.implicitHeight + 2 * pad
    scale: root.shown ? 1 : 0.9
    opacity: root.shown ? 1 : 0
    Behavior on y { Spring {} }
    Behavior on scale { Spring {} }
    Behavior on opacity { Fade {} }

    readonly property real pad: Math.round(digits.font.pixelSize * 0.12)
    bezel: Math.round(digits.font.pixelSize * 0.09)
    refraction: Math.round(digits.font.pixelSize * 0.14)
    frost: 0.2
    tintAmount: 0.14
    glare: 1
    hairline: 0.8
    shadow: 0.28

    Text {
      id: digits
      anchors.fill: parent
      visible: false
      layer.enabled: true
      horizontalAlignment: Text.AlignHCenter
      verticalAlignment: Text.AlignVCenter
      text: Qt.formatTime(root.now, Settings.twelveHour ? "h:mm" : "HH:mm")
      color: "#ffffff"
      font.family: root.rounded
      font.weight: Font.Black
      font.pixelSize: Math.round(Math.min(root.height * 0.22, root.width * 0.17))
      font.features: { "tnum": 1 }
    }
  }

  // ------------------------------------------------------------- you

  // You and the password sit low, as on a Mac, with the picture open between.
  readonly property real low: Math.round(height * 0.76)

  Item {
    id: you
    anchors.horizontalCenter: parent.horizontalCenter
    y: root.low - height + (root.shown ? 0 : 36)
    width: 300
    height: ring.height + 10 + name.implicitHeight
    opacity: root.shown ? 1 : 0
    Behavior on y { Spring {} }
    Behavior on opacity { Fade {} }

    // Your picture in a ring of glass, or a figure without one.
    Glass {
      id: ring
      backdrop: backdrop
      blurred: frosted
      anchors.horizontalCenter: parent.horizontalCenter
      width: 116
      height: 116
      margin: 8
      bezel: 9
      refraction: 11
      frost: 0.3
      tintAmount: 0.16

      Image {
        id: face
        anchors.centerIn: parent
        width: 86
        height: 86
        visible: false
        source: "file://" + root.home + "/avatar.jpg"
        fillMode: Image.PreserveAspectCrop
        sourceSize.width: 172
        sourceSize.height: 172
        cache: false
      }

      Rectangle {
        id: round
        anchors.fill: face
        radius: width / 2
        visible: false
        layer.enabled: true
      }

      MultiEffect {
        anchors.fill: face
        visible: face.status === Image.Ready
        source: face
        maskEnabled: true
        maskSource: round
      }

      Text {
        anchors.centerIn: parent
        visible: face.status !== Image.Ready
        text: "person"
        color: Qt.rgba(1, 1, 1, 0.9)
        font.family: Look.iconFamily
        font.pixelSize: 52
      }
    }

    Text {
      id: name
      anchors.horizontalCenter: parent.horizontalCenter
      anchors.bottom: parent.bottom
      text: root.fullName || Quickshell.env("USER") || ""
      color: "#ffffff"
      font.family: root.rounded
      font.weight: Font.Bold
      font.pixelSize: 20
      layer.enabled: true
      layer.effect: Lift {}
    }
  }

  // ------------------------------------------------------ the password

  Glass {
    id: field
    backdrop: backdrop
    blurred: frosted
    anchors.horizontalCenter: parent.horizontalCenter
    anchors.horizontalCenterOffset: shake
    y: root.low + 22 + (root.shown ? 0 : 48)
    width: 300 + 2 * margin
    height: 54 + 2 * margin
    margin: 10
    bezel: 14
    refraction: 16
    frost: 0.35
    tint: root.errorState ? "#ffd2d2" : "#ffffff"
    tintAmount: root.errorState ? 0.2 : 0.12
    shadow: 0.22
    moved: swell
    opacity: root.shown ? 1 : 0
    Behavior on y { Spring {} }
    Behavior on opacity { Fade {} }
    Behavior on tintAmount { Fade {} }

    // A key pressed swells it a little, as a touched bubble does; a password
    // that was not taken shakes it, the way jelly does.
    property real shake: 0
    property real swell: 0
    transform: Scale {
      origin.x: field.width / 2
      origin.y: field.height / 2
      xScale: 1 + field.swell
      yScale: 1 - field.swell * 0.6
    }
    SequentialAnimation {
      id: press
      NumberAnimation { target: field; property: "swell"; to: 0.018; duration: Look.ms(60); easing.type: Easing.OutQuad }
      NumberAnimation { target: field; property: "swell"; to: 0; duration: Look.ms(460); easing.type: Easing.OutElastic; easing.amplitude: 1.2; easing.period: 0.35 }
    }
    SequentialAnimation {
      id: refusal
      NumberAnimation { target: field; property: "shake"; to: -14; duration: Look.ms(70); easing.type: Easing.OutQuad }
      NumberAnimation { target: field; property: "shake"; to: 0; duration: Look.ms(760); easing.type: Easing.OutElastic; easing.amplitude: 1.6; easing.period: 0.22 }
    }
    Connections {
      target: root
      function onErrorStateChanged() {
        if (root.errorState)
          refusal.restart();
      }
    }

    // No pointer anywhere (the text field would show a text cursor).
    HoverHandler {
      cursorShape: Qt.BlankCursor
    }

    Item {
      anchors.fill: parent
      anchors.margins: field.margin

      TextInput {
        id: passwordInput
        anchors.fill: parent
        anchors.leftMargin: 22 + 40
        anchors.rightMargin: 22 + 40
        verticalAlignment: TextInput.AlignVCenter
        horizontalAlignment: TextInput.AlignHCenter
        activeFocusOnPress: true
        clip: true
        enabled: root.inputEnabled && !root.authenticatingPassword
        readOnly: root.authenticatingPassword
        echoMode: TextInput.Password
        passwordCharacter: "●"
        passwordMaskDelay: 0
        // The field takes the keys and shows nothing of its own: the dots are
        // drawn over it, below, where each one can arrive and leave.
        color: "transparent"
        selectionColor: "transparent"
        selectedTextColor: "transparent"
        font.pixelSize: 12
        cursorDelegate: Item {}

        onTextChanged: {
          typed.follow(text.length);
          if (!root.syncingPasswordText)
            root.passwordTextEdited(text);
          if (text.length > 0)
            root.wakeRequested();
          if (text.length > 0 && root.failureMessage.length > 0)
            root.clearFailureRequested();
        }

        onAccepted: root.send()

        Keys.onPressed: function (event) {
          root.wakeRequested();
          if (event.key === Qt.Key_Escape || (event.modifiers & Qt.ControlModifier && event.key === Qt.Key_U)) {
            root.passwordTextEdited("");
            event.accepted = true;
            return;
          }
          if (event.text.length > 0 || event.key === Qt.Key_Backspace)
            press.restart();
        }
      }

      // What has been typed, a dot for each key. A dot pops into its place
      // as its key is pressed and shrinks away when it is taken back, and the
      // row stays centred by gliding. How many there are is all that is kept:
      // nothing here knows what was typed.
      ListModel {
        id: typed

        // The most dots drawn. A longer password goes on being typed, and the
        // row shows its end.
        readonly property int most: 64

        function follow(length) {
          const wanted = Math.min(length, most);
          for (let i = 0; i < most && typed.count < wanted; i++)
            typed.append({});
          for (let i = 0; i < most && typed.count > wanted; i++)
            typed.remove(typed.count - 1);
        }
      }

      ListView {
        id: dots
        anchors.centerIn: passwordInput
        width: Math.min(contentWidth, passwordInput.width)
        height: parent.height
        orientation: ListView.Horizontal
        interactive: false
        clip: true
        spacing: 8
        model: typed
        onCountChanged: positionViewAtEnd()

        Behavior on width {
          Fade {}
        }

        delegate: Item {
          width: 11
          height: dots.height

          Rectangle {
            anchors.centerIn: parent
            width: 11
            height: 11
            radius: 5.5
            color: "#ffffff"
          }
        }

        add: Transition {
          NumberAnimation { property: "scale"; from: 0; to: 1; duration: Look.ms(340); easing.type: Easing.OutBack; easing.overshoot: 6 * Settings.bounce }
          NumberAnimation { property: "opacity"; from: 0; to: 1; duration: Look.ms(120) }
        }
        remove: Transition {
          NumberAnimation { property: "scale"; to: 0; duration: Look.ms(160); easing.type: Easing.InBack }
          NumberAnimation { property: "opacity"; to: 0; duration: Look.ms(160) }
        }
        displaced: Transition {
          NumberAnimation { properties: "x"; duration: Look.moderate; easing.type: Easing.BezierSpline; easing.bezierCurve: Look.spring }
        }
      }

      // Enter Password, then Checking, or why it failed.
      Text {
        anchors.fill: parent
        anchors.leftMargin: 22
        anchors.rightMargin: 22
        visible: passwordInput.text.length === 0

        // While the password is being checked the word breathes.
        SequentialAnimation on opacity {
          running: root.authenticatingPassword && Look.moving
          loops: Animation.Infinite
          alwaysRunToEnd: true
          NumberAnimation { to: 0.35; duration: Look.ms(560); easing.type: Easing.InOutSine }
          NumberAnimation { to: 1; duration: Look.ms(560); easing.type: Easing.InOutSine }
        }
        text: root.authenticatingPassword ? "Checking" : (root.errorState ? root.failureMessage : "Enter Password")
        color: Qt.rgba(1, 1, 1, root.errorState || root.authenticatingPassword ? 0.97 : 0.82)
        font.family: root.rounded
        font.weight: Font.DemiBold
        font.pixelSize: 16
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
        elide: Text.ElideRight
        layer.enabled: true
        layer.effect: Lift {}
      }

      // The way in: a round button of glass that grows in once there is
      // something to send, and a finger to touch while there is not.
      Item {
        id: go
        anchors.right: parent.right
        anchors.rightMargin: 8
        anchors.verticalCenter: parent.verticalCenter
        width: 38
        height: 38

        readonly property bool ready: passwordInput.text.length > 0 && !root.authenticatingPassword

        Rectangle {
          anchors.fill: parent
          radius: width / 2
          color: Qt.rgba(1, 1, 1, tap.pressed ? 0.42 : (hover.hovered ? 0.32 : 0.24))
          border.width: 1
          border.color: Qt.rgba(1, 1, 1, 0.45)
          scale: go.ready ? (tap.pressed ? 0.9 : 1) : 0
          opacity: go.ready ? 1 : 0
          Behavior on scale { Spring {} }
          Behavior on opacity { Fade {} }
          Behavior on color { ColorAnimation { duration: Look.quick } }

          Text {
            anchors.centerIn: parent
            text: "arrow_forward"
            color: "#ffffff"
            font.family: Look.iconFamily
            font.pixelSize: 22
          }

          HoverHandler {
            id: hover
          }
          TapHandler {
            id: tap
            enabled: go.ready
            gesturePolicy: TapHandler.ReleaseWithinBounds
            onTapped: root.send()
          }
        }

        Text {
          anchors.centerIn: parent
          visible: root.fingerprintConfigured
          opacity: go.ready ? 0 : 0.7
          scale: go.ready ? 0.6 : 1
          Behavior on opacity { Fade {} }
          Behavior on scale { Spring {} }
          text: "fingerprint"
          color: "#ffffff"
          font.family: Look.iconFamily
          font.pixelSize: 24
        }
      }
    }
  }

  // --------------------------------------------------------- the launch

  Glass {
    id: launch
    backdrop: backdrop
    blurred: frosted
    visible: root.nextLaunch.length > 0
    anchors.horizontalCenter: parent.horizontalCenter
    y: root.height - height - 36 + (root.shown ? 0 : 40)
    width: row.implicitWidth + 2 * 18 + 2 * margin
    height: 40 + 2 * margin
    margin: 8
    bezel: 10
    refraction: 12
    frost: 0.4
    tintAmount: 0.1
    shadow: 0.18
    opacity: root.shown ? 1 : 0
    Behavior on y { Spring {} }
    Behavior on opacity { Fade {} }

    Row {
      id: row
      anchors.centerIn: parent
      spacing: 8

      Text {
        anchors.verticalCenter: parent.verticalCenter
        text: "rocket_launch"
        color: "#ffffff"
        font.family: Look.iconFamily
        font.pixelSize: 18
      }
      Text {
        anchors.verticalCenter: parent.verticalCenter
        text: root.nextLaunch
        color: "#ffffff"
        font.family: root.rounded
        font.weight: Font.Bold
        font.pixelSize: 14
        font.features: { "tnum": 1 }
      }
    }
  }

  // How things move here: on a spring that goes a little past and comes
  // back, or by a soft fade. And the shadow that keeps white type legible on
  // a light picture.
  component Spring: NumberAnimation {
    duration: Look.slow
    easing.type: Easing.BezierSpline
    easing.bezierCurve: Look.spring
  }
  component Fade: NumberAnimation {
    duration: Look.moderate
    easing.type: Easing.BezierSpline
    easing.bezierCurve: Look.settle
  }
  component Lift: MultiEffect {
    shadowEnabled: true
    shadowColor: Qt.rgba(0, 0, 0, 0.35)
    shadowBlur: 0.5
    shadowVerticalOffset: 1
  }
}

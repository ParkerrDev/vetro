import QtQuick
import QtQuick.Effects
import QtQuick.Layouts

// Avatar, name, password field, messages and the session picker, on the
// same black glass as the launch card.
Item {
  id: root

  required property var auth
  required property var sessions
  property string avatar: ""
  property string displayName: ""
  property bool interactive: true
  property real radius: 10

  implicitWidth: 360
  implicitHeight: column.implicitHeight + 60

  function focusField() {
    if (editingUser)
      userField.forceActiveFocus();
    else
      passwordField.forceActiveFocus();
  }

  property bool editingUser: auth.user === ""

  Connections {
    target: root.auth

    function onFailed() {
      passwordField.text = "";
      shake.restart();
      root.focusField();
    }
  }

  transform: Translate {
    id: shakeOffset
  }

  SequentialAnimation {
    id: shake

    NumberAnimation { target: shakeOffset; property: "x"; to: -14; duration: 50 }
    NumberAnimation { target: shakeOffset; property: "x"; to: 12; duration: 70 }
    NumberAnimation { target: shakeOffset; property: "x"; to: -8; duration: 70 }
    NumberAnimation { target: shakeOffset; property: "x"; to: 5; duration: 70 }
    NumberAnimation { target: shakeOffset; property: "x"; to: 0; duration: 60 }
  }

  // Small capitals, as on the launch card.
  component Caps: Text {
    property real size: 12

    color: Style.dim
    font.family: Style.family
    font.pixelSize: size
    font.capitalization: Font.AllUppercase
    font.letterSpacing: size * 0.28
  }

  // Glass
  Rectangle {
    anchors.fill: parent
    radius: root.radius
    color: Style.glass
    border.width: 1
    border.color: Style.glassEdge
  }


  ColumnLayout {
    id: column

    spacing: 16

    anchors {
      left: parent.left
      right: parent.right
      top: parent.top
      margins: 30
    }

    // Avatar, or an initial when there is no picture.
    Item {
      Layout.alignment: Qt.AlignHCenter
      implicitWidth: 84
      implicitHeight: 84

      Rectangle {
        anchors.fill: parent
        radius: width / 2
        color: Qt.rgba(1, 1, 1, 0.08)
        visible: avatarImage.status !== Image.Ready

        Text {
          anchors.centerIn: parent
          text: (root.displayName || root.auth.user || "?").charAt(0).toUpperCase()
          color: Style.text
          font.family: Style.family
          font.pixelSize: 36
          font.weight: Font.Bold
        }
      }

      Image {
        id: avatarImage

        anchors.fill: parent
        source: root.avatar
        fillMode: Image.PreserveAspectCrop
        visible: false
        cache: false
        asynchronous: true
      }

      Item {
        id: avatarMask

        anchors.fill: parent
        visible: false
        layer.enabled: true

        Rectangle {
          anchors.fill: parent
          radius: width / 2
        }
      }

      MultiEffect {
        anchors.fill: parent
        source: avatarImage
        visible: avatarImage.status === Image.Ready
        maskEnabled: true
        maskSource: avatarMask
      }

      Rectangle {
        anchors.fill: parent
        anchors.margins: -4
        radius: width / 2
        color: "transparent"
        border.width: 1
        border.color: Qt.rgba(1, 1, 1, 0.45)
      }
    }

    // Who is logging in. Click the name to type another user.
    Item {
      Layout.fillWidth: true
      implicitHeight: 30

      Text {
        anchors.centerIn: parent
        visible: !root.editingUser
        text: root.displayName || root.auth.user
        color: Style.text
        font.family: Style.family
        font.pixelSize: 19
        font.weight: Font.Bold
        font.capitalization: Font.AllUppercase
        font.letterSpacing: 19 * 0.12

        MouseArea {
          anchors.fill: parent
          anchors.margins: -6
          cursorShape: Qt.PointingHandCursor
          enabled: root.interactive
          onClicked: {
            root.editingUser = true;
            userField.text = root.auth.user;
            userField.selectAll();
            userField.forceActiveFocus();
          }
        }
      }

      Rectangle {
        anchors.fill: parent
        visible: root.editingUser
        radius: 4
        color: Qt.rgba(1, 1, 1, userField.activeFocus ? 0.1 : 0.06)
        border.width: 1
        border.color: userField.activeFocus ? Qt.rgba(1, 1, 1, 0.85) : Style.hairline

        TextInput {
          id: userField

          anchors.fill: parent
          anchors.leftMargin: 12
          anchors.rightMargin: 12
          verticalAlignment: TextInput.AlignVCenter
          horizontalAlignment: TextInput.AlignHCenter
          color: Style.text
          selectionColor: Qt.rgba(1, 1, 1, 0.35)
          font.family: Style.family
          font.pixelSize: 16
          enabled: root.interactive
          onAccepted: {
            if (text.trim() === "")
              return;
            root.auth.user = text.trim();
            root.editingUser = false;
            passwordField.forceActiveFocus();
          }
          Keys.onEscapePressed: {
            if (root.auth.user !== "") {
              root.editingUser = false;
              passwordField.forceActiveFocus();
            }
          }
        }

        Caps {
          anchors.centerIn: parent
          visible: userField.text === ""
          text: "User name"
          color: Style.faint
        }
      }
    }

    // Password, or whatever PAM is asking for next.
    Rectangle {
      Layout.fillWidth: true
      Layout.topMargin: 2
      implicitHeight: 46
      radius: 4
      color: Qt.rgba(1, 1, 1, passwordField.activeFocus ? 0.1 : 0.06)
      border.width: 1
      border.color: passwordField.activeFocus ? Qt.rgba(1, 1, 1, 0.85) : Style.hairline

      Behavior on border.color {
        ColorAnimation {
          duration: 150
        }
      }

      TextInput {
        id: passwordField

        anchors {
          left: parent.left
          right: submit.left
          top: parent.top
          bottom: parent.bottom
          leftMargin: 16
          rightMargin: 8
        }
        verticalAlignment: TextInput.AlignVCenter
        echoMode: root.auth.prompt !== "" && root.auth.promptEcho ? TextInput.Normal : TextInput.Password
        passwordCharacter: "•"
        color: Style.text
        selectionColor: Qt.rgba(1, 1, 1, 0.35)
        font.family: Style.family
        font.pixelSize: 18
        font.letterSpacing: echoMode === TextInput.Password ? 3 : 0
        enabled: root.interactive && !root.auth.busy && !root.auth.launching
        focus: !root.editingUser
        clip: true
        onAccepted: {
          if (text === "" && root.auth.prompt === "")
            return;
          const answeringPrompt = root.auth.prompt !== "";
          root.auth.submit(text);
          // A one time code or similar shouldn't linger in the field. The
          // password stays (as dots) while PAM checks it.
          if (answeringPrompt)
            text = "";
        }
        Keys.onEscapePressed: text = ""
      }

      Caps {
        anchors.left: passwordField.left
        anchors.verticalCenter: parent.verticalCenter
        visible: passwordField.text === ""
        text: root.auth.prompt !== "" ? root.auth.prompt.replace(/:\s*$/, "") : "Password"
        color: Style.faint
      }

      // Submit arrow, which turns into a spinner while PAM thinks.
      Rectangle {
        id: submit

        readonly property bool lit: passwordField.text !== "" || root.auth.busy || root.auth.launching

        anchors {
          right: parent.right
          verticalCenter: parent.verticalCenter
          rightMargin: 6
        }
        width: 34
        height: 34
        radius: 3
        color: lit ? Style.text : "transparent"
        border.width: 1
        border.color: lit ? Style.text : Style.hairline

        Behavior on color {
          ColorAnimation {
            duration: 150
          }
        }

        Text {
          anchors.centerIn: parent
          text: root.auth.busy || root.auth.launching ? "progress_activity" : "arrow_forward"
          color: submit.lit ? "black" : Style.faint
          font.family: "Material Symbols Rounded"
          font.pixelSize: 20

          RotationAnimation on rotation {
            running: root.auth.busy || root.auth.launching
            loops: Animation.Infinite
            from: 0
            to: 360
            duration: 900
          }
        }

        MouseArea {
          anchors.fill: parent
          cursorShape: Qt.PointingHandCursor
          enabled: root.interactive
          onClicked: passwordField.accepted()
        }
      }
    }

    Text {
      Layout.fillWidth: true
      visible: text !== ""
      text: root.auth.launching ? "Starting " + (root.sessions.current ? root.sessions.current.name : "your session") + "..." : root.auth.message
      color: root.auth.isError ? Style.live : Style.dim
      font.family: Style.family
      font.pixelSize: 14
      horizontalAlignment: Text.AlignHCenter
      wrapMode: Text.WordWrap
    }

    // Session picker: click to cycle through installed Wayland sessions.
    Item {
      Layout.fillWidth: true
      implicitHeight: 24

      Row {
        anchors.centerIn: parent
        spacing: 6
        opacity: sessionMouse.containsMouse ? 1 : 0.75

        Text {
          text: "desktop_windows"
          color: Style.dim
          font.family: "Material Symbols Rounded"
          font.pixelSize: 16
          anchors.verticalCenter: parent.verticalCenter
        }

        Caps {
          text: root.sessions.current ? root.sessions.current.name : "Hyprland"
          size: 11
          anchors.verticalCenter: parent.verticalCenter
        }

        Text {
          visible: root.sessions.list.length > 1
          text: "unfold_more"
          color: Style.dim
          font.family: "Material Symbols Rounded"
          font.pixelSize: 15
          anchors.verticalCenter: parent.verticalCenter
        }
      }

      MouseArea {
        id: sessionMouse

        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        enabled: root.interactive && root.sessions.list.length > 1
        onClicked: {
          root.sessions.next();
          root.focusField();
        }
      }
    }
  }
}

import QtQuick
import Quickshell
import Quickshell.Io
import "../vetro.lib"

// Vetro's notifications on Omarchy, in place of omarchy.notifications: the
// toasts on the glass, and the list the centre (vetro.center) shows. The
// server is vetro-notifyd, started here; this plugin is
// omarchy.notifications' clone, so enabling it switches Omarchy's off, and
// Omarchy's keybinds (`omarchy-shell notifications ...`) and the Do Not
// Disturb of its widgets reach this instead.
Item {
  id: service

  // Injected by Omarchy's shell.
  property var shell: null
  property var manifest: null

  // The part of omarchy.notifications that Omarchy's widgets use.
  readonly property bool doNotDisturb: Notifications.dnd
  function setDoNotDisturb(value) {
    Notifications.setDnd(!!value);
  }

  Component.onCompleted: Notifications.serving = true
  Component.onDestruction: Notifications.serving = false

  ToastLayer {}

  // omarchy.notifications' IPC, so Omarchy's keybinds and scripts work:
  // Super+comma and friends, `omarchy-shell notifications ...`.
  IpcHandler {
    target: "notifications"

    function dndState(): string {
      return Notifications.dnd ? "on" : "off";
    }

    // The daemon answers with the new state a moment later; this says what
    // it will be.
    function toggleDnd(): string {
      const on = !Notifications.dnd;
      Notifications.setDnd(on);
      return on ? "on" : "off";
    }

    function setDnd(value: string): string {
      const on = ["true", "1", "on", "yes"].includes(String(value || "").toLowerCase());
      Notifications.setDnd(on);
      return on ? "on" : "off";
    }

    function isDnd(): string {
      return dndState();
    }

    // Every notification stays until it is dismissed. With the centre
    // loaded, open that; without it, bring the newest back up as toasts.
    function showHistory(): string {
      if (Notifications.centerAvailable) {
        Quickshell.execDetached(["omarchy-shell", "shell", "toggle", "vetro.center", "{}"]);
        return "ok";
      }
      Notifications.reshow();
      return Notifications.all.length ? "ok" : "none";
    }

    // Forget them all (the centre's clear button).
    function clear(): string {
      Notifications.clear();
      return "ok";
    }

    // Take the toasts off the screen; they stay in the centre.
    function dismissAll(): string {
      Notifications.hideAll();
      return "ok";
    }

    function dismissOne(): string {
      const note = Notifications.toasts[0];
      if (!note)
        return "none";
      Notifications.hide(note.id);
      return "ok";
    }

    // Do what the newest toast offers, as a click on it would.
    function invokeLast(): string {
      const note = Notifications.toasts[0];
      if (!note)
        return "none";
      Notifications.launch(note);
      if (note.actions.length > 0)
        Notifications.invoke(note.id);
      else
        Notifications.dismiss(note.id);
      return "ok";
    }

    // Omarchy's first-run notifications take themselves away by summary
    // once their action has run.
    function dismiss(summary: string): string {
      const needle = String(summary || "");
      if (!needle)
        return "none";
      const hits = Notifications.all.filter(note => note.summary.includes(needle));
      for (const note of hits)
        Notifications.dismiss(note.id);
      return hits.length ? "ok" : "none";
    }

    function ping(): string {
      return "ok";
    }
  }
}

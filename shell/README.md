# shell: the Omarchy shell plugins

Seven plugins, each a folder with a `manifest.json` that `install.sh` links into
`~/.config/omarchy/plugins` and `vetro on` enables. Omarchy keeps its own bar,
widgets and keybinds; these draw what is on or behind them.

| Plugin | What it is | Clone of |
|---|---|---|
| `vetro.barglass` | The rounded pane behind Omarchy's bar, which the engine turns to glass. It is a surface of exactly the pane's size, so the glass has its rim where the pane ends. | |
| `vetro.menu` | Omarchy's menu, unchanged but for a calculator: type a sum and the answer is the first row. Also its bar button. | `omarchy.menu` |
| `vetro.notifications` | The notifications: the toasts on the glass, and the list the centre shows. It starts `vetro-notifyd` (`notifyd/`), which holds the notification name on the session bus and keeps every rule. | `omarchy.notifications` |
| `vetro.osd` | A capsule under the bar with the volume, the brightness or a message. | `omarchy.osd` |
| `vetro.lock` | Omarchy's lock (its own password and fingerprint flows) in Vetro's design. | `omarchy.lock` |
| `vetro.center` | The notification centre: everything still waiting, Do Not Disturb, clear all. Needs `vetro.notifications`. | |
| `vetro.center-button` | The bell in the bar that opens the centre, with a dot while something waits. | |

A clone replaces the Omarchy plugin it is cloned from while it is enabled, keeping
its keybinds and scripts, and gives it back when disabled; that is Omarchy's own
mechanism (`omarchy plugin clone`).

The launch addon has a plugin too, `vetro.cards` (`addons/launch/plugin`).

## The library they share

`vetro.lib` is not a plugin: it is the QML the plugins import by a path beside
their own (`../vetro.lib`), linked in beside them by `install.sh`. It has to be
one folder, because Quickshell keys a singleton by its URL and the settings, the
look and the notifications are singletons several plugins share.

| File | What it is |
|---|---|
| `Settings.qml` | `~/.config/vetro/shell.json`, read again whenever it changes |
| `Look.qml` | the colours (white type on a film of black), the type, the shapes and the motion |
| `Notifications.qml` | the bridge to `vetro-notifyd`: it starts the daemon, sends what the buttons do and keeps what it says |
| `Toast.qml`, `ToastLayer.qml` | one notification as a card, and the toasts on each screen, each a surface of its own |
| `Icon.qml`, `Label.qml`, `Hairline.qml` | an icon, type and a line in the look |

## The notification daemon

`notifyd/` is `vetro-notifyd`, in Rust: the `org.freedesktop.Notifications`
server. A library that never prints (the notifications, their toasts' time, Do
Not Disturb, the pictures sent in memory) and a thin binary over it (the bus, the
files, the lines it trades with the plugin). `install.sh` builds it with cargo
into `~/.local/state/vetro/bin` and links it onto the PATH; without it Vetro
leaves Omarchy's notifications in place.

## Settings

`install.sh` copies `shell.json` to `~/.config/vetro/shell.json` once; the copy
is yours to change, and the shell reads it again whenever it changes. Every key
is optional:

```
gaps.edge, gaps.sides    from the bar to the screen edge, and at its ends
bar.radius               the bar's corners (15)
panels.radius            the corners of the toasts, the centre and the cards (20)
panels.opacity           how much film lies over the glass, 0 to 1 (0.2)
panels.frame, .frameWidth  a hairline round each panel
text.size, .nativeRendering  the base point size (10), hinted type
clock.twelveHour
osd.timeout              how long the capsule stays, in ms (700)
motion.scale             every animation's length times this; 0 turns them off
motion.bounce            how far what springs goes past and comes back, 0 to 1 (0.5)
screens.only, .except    the screens the toasts and the capsule go to, by name
```

The panels move at the glass's pace, as the windows do: `vetro tune`, Motion,
Pace, once it is applied.

`fonts/` is D-DIN, which the whole shell is set in, and the Material Symbols icon
fonts; `install.sh` links it to `~/.local/share/fonts/vetro`.

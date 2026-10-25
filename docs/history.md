# History: where Vetro came from

Vetro is one project in place of three. This file says which, how their
histories are in this repository, what was left behind, and what every old name
is now. It is the one place in the tree, with `migrate.sh` and its tests, where
the old names are written.

## The three

| Project | What it was | Taken at | Folder it came in under |
|---|---|---|---|
| liftoff | A Hyprland desktop themed around SpaceX, and then, on Omarchy, "the look": a bar in glass, cards, a launch wallpaper, black and white apps. | 2026-10-11 | `liftoff/` |
| meniscus | The glass for Omarchy as a theme: two themes, a pane behind the bar, the script that builds the engine, its pins and its update hook. | 2026-10-11 | `meniscus/` |
| hyprglass (the `meniscus` branch of the fork) | The glass engine, a Hyprland plugin: upstream `hyprnux/hyprglass` v0.10.0 and 17 commits of the fork on top. | 2026-10-11 | `hyprglass/` |

## Reading the history

This repository's history starts where it was published. The three projects'
own histories, and Vetro's before publishing, are kept apart from it.

## What was left behind

These are the parts that no installed file referred to, or that belong to
another project now:

- the Windows VM and its bar button (the passthru project now);
- the backup and restore scripts from before the Omarchy reinstall;
- the greetd login screen and the rest of the desktop liftoff began as
  (`home/`), which Omarchy replaces;
- the whole port, which put liftoff's own keybinds, monitors and bar on top of
  Omarchy, and the boot setup;
- liftoff's own bar as a replacement for Omarchy's, never enabled, and the
  library its plugins shared, which `shell/vetro.lib` and `vetro-notifyd`
  replace;
- meniscus's two themes and the glass that followed its theme;
- the engine's release machinery and its hyprpm manifest.

## What the names were, and are

| Was | Is |
|---|---|
| command `liftoff-look on`, `off`, `toggle`, `status`, `glass load`, `glass unload`; `meniscus on`, `off`, `glass ...` | `vetro on`, `off`, `toggle`, `status`, `glass load`, `glass unload`; and new: `vetro build`, `vetro doctor` |
| `liftoff` (the repository manager and wallpaper tool) | `vetro-launch` (the wallpaper and next-launch part; the rest went) |
| `liftoff-launchd`, `liftoff-launchd.service` | `vetro-launchd`, `vetro-launchd.service` |
| `~/.config/hypr/liftoff-look.lua`, `meniscus.lua` | `~/.config/hypr/vetro.lua` |
| plugin ids `liftoff.barglass`, `.cards`, `.center`, `.center-button`, `.lock`, `.menu`, `.notifications`, `.osd` | `vetro.barglass`, ... (`vetro.cards` is the launch addon's) |
| `liftoff.windows-button`; `liftoff` (its bar and library) | gone (passthru's button); the library is `vetro.lib`, written afresh |
| layer namespaces `liftoff:barglass`, `liftoff:launch`, `liftoff:weather` | `vetro:barglass`, `vetro:launch`, `vetro:weather` |
| marked blocks `liftoff look`, `liftoff glass` (shell.toml, kitty.conf, foot.ini); `meniscus` (hyprland.lua) | `vetro look`, `vetro glass`; `vetro` |
| hooks `50-liftoff.sh`, `50-meniscus-glass.sh` | `50-vetro.sh`, `50-vetro-glass.sh` |
| `~/.local/state/liftoff` (flag `look-off`) | `~/.local/state/vetro` (flag `off`; the remembered theme is dropped) |
| `~/.local/lib/meniscus` | `~/.local/lib/vetro` |
| `~/.cache/meniscus/hyprglass` | `~/.cache/vetro/engine` |
| `~/.config/liftoff/shell.json`, `wallpaper.toml` | `~/.config/vetro/shell.json`, `wallpaper.toml` |
| `~/.cache/liftoff/launches.json`, `~/.local/share/liftoff/clips` | `~/.cache/vetro/launches.json`, `~/.local/share/vetro/clips` |
| `$XDG_RUNTIME_DIR/liftoff-glass-stood-down`, `liftoff-glass-readings`, `liftoff/` | `vetro-glass-stood-down`, `vetro-glass-readings`, `vetro/` |
| `~/.local/share/fonts/liftoff` | `~/.local/share/fonts/vetro` |
| Kvantum theme `liftoff`; GTK `matugen.css`; Qt `matugen-kde.colors` | `vetro`; `vetro.css`; `vetro.colors` |
| theme `liftoff` | theme `launch`, in the `theme` addon; the two meniscus themes are gone |
| `/usr/share/sddm/themes/liftoff`, `99-zz-liftoff.conf`, `/var/lib/liftoff` | `.../vetro`, `99-zz-vetro.conf`, `/var/lib/vetro` (the `login` addon, as root) |
| `glass/build.sh`, `glass/pins`, `glass/post-update.sh` | `engine/build.sh`, `engine/pins`, `engine/post-update.sh` |
| `MENISCUS_ENGINE` (where to fetch the engine) | `VETRO_ENGINE_SOURCE`; by default this repository |
| the engine inside Hyprland: `hyprglass`, `hl.plugin.hyprglass`, `hyprctl hyprglass` | unchanged |

## What `migrate.sh` does with a home folder that has them

`install.sh` runs it. It takes over the links, the marked blocks, the plugin ids
in the shell's own `shell.json` (renamed where they stand, so the bar keeps its
order), the key bound to `liftoff-look` in `~/.config/hypr/bindings.lua`, the
launch service, the state, the cache, the clips and the built engine, and saves
what it replaces under `~/.local/state/vetro-migrated`. `migrate.sh --dry-run`
says what it would do, `--check` looks for leftovers, and `--undo` takes Vetro out
and puts the home folder back as it was. Files that merely sit beside the old
ones (the Windows VM's state and tools) are left where they are.

## Switching a machine over, and going back

On a machine that has the two earlier projects installed and switched on:

```sh
cd ~/Dev/GitHub/vetro
./migrate.sh --dry-run     # read what it will do; nothing changes
./install.sh               # in a terminal inside the session
vetro status               # on / engine: loaded / guard: ready
vetro doctor               # all well
```

`install.sh` links Vetro in, runs `migrate.sh`, installs the addons, finds the
engine the old build left in `~/.local/lib/meniscus` (it is moved, so nothing is
compiled), and, because the look was on, runs `vetro on`: the shell restarts once
and Hyprland reloads once. The old engine may be left loaded in Hyprland; it keeps
working, but `vetro off` cannot unload it (Hyprland unloads by the path a plugin
was loaded from, and that has changed) until the next login, and says so.

What needs root is separate, and is best done last: `addons/login/install-system.sh`
(as root) gives the login screen its new names. The move cannot
undo that. Afterwards, anything that refers to the old project by name outside
Vetro needs the same change.

Going back, as long as the root step has not been done:

```sh
./migrate.sh --undo        # uninstalls Vetro, puts back every link, block, file, state folder
omarchy-restart-shell && hyprctl reload
```

The record is `~/.local/state/vetro-migrated/`; an undone record is renamed
`.undone`. What an undo cannot put back is the engine's old build folder (it is
rebuilt) and the runtime marks.

## How Vetro differs from the look as it ran

The aim was none; these are the places where it does, and why.

- `vetro on` no longer sets the theme (`omarchy theme set liftoff`), and
  `vetro off` no longer sets back a remembered one or resets GTK's font and
  cursor. Vetro is not a theme. The launch theme is an addon you choose like
  any other, and its hook takes back what it set when you choose another.
- The bar's own background colour is in the colour override (black), where it
  came from the theme before. With a theme whose bar is not black, the pane
  behind the bar is a shade different from what it was; it no longer follows
  the theme.
- `vetro off` removes the Qt links as well as the GTK ones; the old `off` left
  Qt's.
- `vetro on` clears the guard against a config read in a loop before it reloads
  Hyprland, not after, so a reload cannot read the config as still in a loop.
- The watermark on the lock screen and the login screen says VETRO, not
  LIFTOFF.
- The notification when Vetro goes on names `vetro off`, not a key.
- The bar block in `shell.toml` no longer carries its three comment lines, and the
  blocks are separated by a blank line; the shell reads neither.
- liftoff's own bar, which was never enabled, is no longer in `omarchy plugin
  list`, and neither is the Windows button copy (passthru's stays).
- The launch addon is not installed on a machine without `mpvpaper` and `mpv`.
- A fresh install is off until `vetro on`. A home folder that was on stays on.
- The commands `liftoff-look`, `liftoff` and `meniscus` no longer exist. The key
  bound to `liftoff-look toggle` in `bindings.lua` is changed by the move to run
  `vetro toggle`.

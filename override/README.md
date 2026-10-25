# override: what Vetro lays over any theme

Vetro is not a theme. Choosing a theme or a wallpaper never touches what is in
this folder, and nothing here chooses a theme. `vetro on` lays these files over
whatever theme is current; `vetro off` takes every one of them out again. While
they are laid, the glass looks the same under any theme, because the theme's
own colours for the same things are overridden.

| File | Laid where | By |
|---|---|---|
| `glass.toml` | a marked block in `~/.config/omarchy/shell.toml` | `vetro glass load`, and only while the engine draws glass |
| `bar.toml` | a marked block in the same file | `vetro on` |
| `terminal/foot.ini`, `terminal/kitty.conf` | a marked block in `~/.config/foot/foot.ini` and `~/.config/kitty/kitty.conf` (the terminals that are installed): a fifth of black behind the text | `vetro on` |
| `gtk-3.0/`, `gtk-4.0/` | links in `~/.config/gtk-3.0` and `gtk-4.0` | `vetro on` |
| `qt6ct/`, `kvantum/` | links in `~/.config/qt6ct` and `~/.config/Kvantum` (`qt6ct.conf` is written from `qt6ct.conf.in`, because qt6ct wants an absolute path to the colours) | `vetro on` |

## Apps of glass

A window is glass when its own background is clear and the compositor keeps a
pane behind it (the engine's window glass, `hypr/vetro.lua`). So what is laid
over an app is not a colour for its surfaces but next to none:

- **GTK apps** (the files app, and whatever else uses GTK 3 or 4): the window
  draws a fifth of black, a list a little more, a sidebar a faint step of
  white, and a header bar nothing of its own. Menus and other popups stay
  solid, since no pane is behind those.
- **Terminals**: the same fifth of black behind the text (`background_opacity`
  in kitty, `alpha` in foot).
- **Qt apps**: Kvantum draws the window a fifth as solid as it did
  (`translucent_windows`, `reduce_window_opacity`); the frames inside a window
  keep their own.
- **Browsers** cannot draw themselves see-through, and since 2026-10-18 they
  are solid, as every window is whose app does not draw a clear background
  (`hypr/vetro.lua` makes Omarchy's few percent of see-through whole): glass
  is only behind the apps this folder makes clear. `touch
  ~/.config/vetro/browser-glass` makes browsers glass the only way they can
  be: the whole window a little see-through over its pane. Their frame (tabs,
  toolbar) is the glass's own dark grey, `#161616`, where the theme would give
  it its colour: Omarchy keeps that colour in the browsers' policy, and
  Chromium makes a whole palette of it, so a theme's blue-grey came out
  purple. Chromium still takes one accent of its own from the grey (a teal,
  for buttons and the focused address bar). Vetro sets the grey through
  Omarchy's own helper and its rule, as a change of theme does, and gives the
  theme's colour back when it is switched off (`lib/lay.sh`, "browser").
  `touch ~/.config/vetro/browser-themed` keeps the theme's colour.

The fifth of black is the same film as over every other surface (the bar,
the panels, the cards, the menus: `glass.toml`), so that every pane of glass
looks the same. It is also there for the times no pane is drawn (the engine not
built, or stood down): Hyprland's own blur is behind the window then, and white
text still holds on it. With the pane, the two tones come to about half.

Every window, glass or not, has its outline made of glass: Hyprland's coloured
border is made clear, and the engine fills the band it keeps
(`windows.border` in `hypr/vetro.lua`).

The shell reads `~/.config/omarchy/shell.toml` over the theme's own
`shell.toml`, key by key, and keeps it through a change of theme; that is why
the glass colours stay where they are when the theme changes. `glass.toml`
sets every colour that Omarchy's bar, cards, menus, controls and password
prompt take from the theme, so nothing of the theme's shows through the glass.

Without the engine the panes are plain blur and the cards go back to the
theme's own dark slabs, so `glass.toml` is not laid then. The bar block is
laid either way: the pane behind the bar needs the bar to be see-through.

`kvantum/make-theme.py` is how `kvantum/vetro/` was made (KvLibadwaita,
recoloured; GPL-3.0, see `kvantum/vetro/COPYING`). Nothing runs it at install.

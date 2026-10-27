-- Vetro on Omarchy, and nothing but the look: the blur behind the glass, the
-- panes that get the engine's glass, the corners, the cursor and the Qt theme.
-- Keybinds, monitors, window rules and everything else stay Omarchy's own.
-- ~/.config/hypr/hyprland.lua loads this last, through the one marked block
-- that install.sh adds.
--
-- Vetro is not a theme. Nothing here reads which theme is chosen, so a change
-- of theme or wallpaper leaves all of it as it is.

local home = os.getenv("HOME") or ""

local function exists(path)
  local file = io.open(path, "r")
  if file then
    file:close()
    return true
  end
  return false
end

-- `vetro off` leaves this flag, and Omarchy is exactly its own again until
-- `vetro on`. A fresh install leaves it too, so that nothing changes on screen
-- before the first `vetro on`.
if exists((os.getenv("XDG_STATE_HOME") or (home .. "/.local/state")) .. "/vetro/off") then
  return
end

-- A theme installed by a package, or by hand in your home folder.
local function installed(kind, name)
  return exists("/usr/share/" .. kind .. "/" .. name) or exists(home .. "/.local/share/" .. kind .. "/" .. name)
end

------------------------------------------------------------------- blur -----

-- Black glass needs blur behind it (Omarchy ships blur off). These are the
-- values the look has always run with.
hl.config({
  decoration = {
    blur = {
      enabled = true,
      size = 5,
      passes = 3,
      noise = 0.0117,
      vibrancy = 0.1696,
      new_optimizations = true,
      popups = true,
    },
  },
  misc = {
    force_default_wallpaper = 0,
    background_color = "rgb(000000)",
  },
})

----------------------------------------------------------- liquid glass -----

-- A config that is read again and again within seconds is in a loop, and a
-- compositor in a loop is a frozen screen. The glass below is the one part of
-- this file that talks to a plugin, so it counts the readings of this file,
-- and past twelve in ten seconds it is left out until `vetro on`.
--
-- Only Hyprland's own readings count. This file can be run outside it (a test,
-- a linter, anyone trying it with plain lua), and such runs once stood the
-- glass down on a working desktop for an hour by filling the tally from
-- outside. So whoever reads it must be the compositor, by the name the kernel
-- has for this process; the tests say so themselves with VETRO_GUARD_TEST.
local runtime = os.getenv("XDG_RUNTIME_DIR") or ""
local function read_by_hyprland()
  if os.getenv("VETRO_GUARD_TEST") == "1" then
    return true
  end
  local stat = io.open("/proc/self/stat", "r")
  if not stat then
    return false
  end
  local line = stat:read("l") or ""
  stat:close()
  return line:match("^%d+ %((.-)%)") == "Hyprland"
end
local function read_in_a_loop()
  if runtime == "" or not read_by_hyprland() then
    return false
  end
  local mark = io.open(runtime .. "/vetro-glass-stood-down", "r")
  if mark then
    mark:close()
    return true
  end
  local now, since, count = os.time(), 0, 0
  local tally = io.open(runtime .. "/vetro-glass-readings", "r")
  if tally then
    since = tonumber(tally:read("l") or "") or 0
    count = tonumber(tally:read("l") or "") or 0
    tally:close()
  end
  if now - since > 10 then
    since, count = now, 0
  end
  count = count + 1
  tally = io.open(runtime .. "/vetro-glass-readings", "w")
  if tally then
    tally:write(since, "\n", count, "\n")
    tally:close()
  end
  if count <= 12 then
    return false
  end
  mark = io.open(runtime .. "/vetro-glass-stood-down", "w")
  if mark then
    mark:write("read ", count, " times in ten seconds\n")
    mark:close()
  end
  return true
end

-- What gets glass: each surface named here, and in it every patch that is
-- more solid than the cut-off beside its name. The engine (a fork of
-- hyprglass, in engine/) takes each such patch for a pane of its own, whatever
-- its outline, and gives it the whole material: the frost, the light bent in a
-- narrow band inside the edge, a lit rim that follows the pane's own corners,
-- and a soft shadow. So a card inside a larger see-through surface is glass to
-- its own edge, and the dim a menu draws behind its card is not glass at all.
--
-- The cut-off sits midway between what is beside a pane and the pane itself:
--   0.10   a pane filled at 0.2 with clear air beside it (the bar's strip, the
--          cards and panels at panels.baseOpacity, and Omarchy's panels while
--          the glass draws)
--   0.155  Omarchy's cards on a dim, the password prompt's among them: the dim
--          is 0.1 and a card over it comes to 0.208, the same fifth of black
--          as every other surface (override/glass.toml, which `vetro glass
--          load` puts in force)
local panes = {
  -- The bar's strip: in Vetro's own bar, where the strip is part of the bar's
  -- surface and so above windows, and under Omarchy's own bar where Vetro's
  -- could not be built (a surface of its own, under windows). Only one of
  -- the two is ever on screen.
  ["vetro:bar"] = 0.10,
  ["vetro:barglass"] = 0.10,
  -- The launch addon's cards. Where the addon is not installed no surface has
  -- these names, and the entries do nothing.
  ["vetro:launch"] = 0.10,
  ["vetro:weather"] = 0.10,
  -- Homescreen's cards, the launch card and the weather card among them once
  -- they are placed as widgets: each card is filled as Vetro's panels are
  -- (panels.baseOpacity, 0.2), with clear air between them. Where Homescreen
  -- is not installed no surface has this name.
  ["homescreen"] = 0.10,
  -- Vetro's notifications, volume display, and the panels that drop from the
  -- bar (the notification centre among them)
  ["hyprland-shell:notifs"] = 0.10,
  ["hyprland-shell:osd"] = 0.10,
  ["hyprland-shell:bar"] = 0.10,
  ["vetro:center"] = 0.10,
  -- Omarchy's cards on a dim
  ["omarchy-menu"] = 0.155,
  ["omarchy-clipboard"] = 0.155,
  ["omarchy-emojis"] = 0.155,
  ["omarchy-reminders"] = 0.155,
  ["omarchy-polkit"] = 0.155,
  -- Omarchy's cards on clear air (every panel that drops from its bar is an
  -- omarchy-keyboard-panel)
  ["omarchy-keyboard-panel"] = 0.10,
  ["omarchy-notifications"] = 0.10,
  ["omarchy-osd"] = 0.10,
  ["omarchy-network-qr"] = 0.10,
  -- Whole-screen surfaces: the faint film the menu (Super+Space) puts under
  -- itself (shell/vetro.menu), and the picture chooser's dim. They were
  -- Hyprland's blur, which blurred the desktop's widgets in with the rest;
  -- as glass they show the wallpaper alone, as every other pane does.
  ["vetro:backdrop"] = 0.01,
  ["omarchy-image-selector"] = 0.05,
}

-- Real glass, when the engine is there: `vetro glass load` puts it into
-- Hyprland from outside (at login, when Vetro is switched on, and after a
-- change of theme). This file never loads it. Hyprland reads its config again
-- each time a plugin is loaded, so a config that loads one is read again,
-- loads it again, and never stops.
--
-- The glass as it comes is Apple's own, on the panes (the bar, the widgets, the
-- cards and panels) and on windows: the engine's "apple27", the shader of macOS 27 as it was
-- transcribed from QuartzCore, with Apple's own material recipe and the light
-- of macOS 27 as it was decoded (2026-10-18: "implement authentic apple
-- liquid glass", then "refresh all windows": windows too, each pane its own
-- rounded rectangle). Until then the panes were Apple's current material as
-- measured, the engine's "regular27": a rim of a dark hairline and two lines of light that
-- bends what it shows, what is behind coming through blurred, in its own
-- darks and colours, and bright things behind pulled down (2026-10-15: "make them liquid glass", then "still cloudy in some
-- areas"). Windows are the engine's "regular" kind: frosted enough
-- that nothing can be read through it while shapes and colours still come
-- through, and toned (2026-10-13: "it was perfect then"); on a pane as large
-- as a window the current material is fog.
--
-- `vetro glass look` chooses another by leaving a file in ~/.config/vetro:
-- `liquid-material`, the panes as they were until 2026-10-18 (regular27);
-- `regular-material`, the regular kind on the panes too (dark smoked cards);
-- `cloud-material`, the current material on windows too, as it was measured;
-- `clear-material`, where what is behind keeps its own darks and colours and
-- is only put a little out of focus, panes and windows; and
-- `sharp-material`, the same with nothing out of focus at all.
local liquid = false
local hg = hl.plugin.hyprglass
if hg and not read_in_a_loop() then
  liquid = pcall(function()
    local chosen = function(look)
      return exists(home .. "/.config/vetro/" .. look .. "-material")
    end
    local cloud, clear, sharp = chosen("cloud"), chosen("clear"), chosen("sharp")
    local current = cloud or clear or sharp
    local regular = chosen("regular") and not current
    -- Apple's own glass is the one with no file. Since the evening of
    -- 2026-10-18 it is the glass the user tunes (the engine's "tuned": the
    -- values of ~/.config/vetro/glass.conf, which `vetro tune` writes, and
    -- the Author's pick until it does): "Make a application so I can tune it
    -- myself ... the clear/tinted glass/frost sliders", and "reach an iOS 27
    -- liquid glass, rather than a liquid glass 26 look". A lens-material file
    -- gives the lens of earlier that evening back (`vetro glass look lens`), a
    -- macos-material file macOS 27's recipe (`vetro glass look macos`).
    local apple = not (current or chosen("regular") or chosen("liquid"))
    local apple_glass = chosen("macos") and "apple27" or chosen("lens") and "lens" or "tuned"
    -- The current material without its cloud and its white: one soft copy
    -- of what is behind, or none.
    local function lucid(body_shade)
      return {
        body_shade = body_shade,
        frost_wide = 4,
        frost_cloud = clear and 1 or 0,
        tint_over_dark = 0,
        tint_over_bright = 0,
      }
    end
    -- A pane's glass is the current material with two of its numbers not
    -- Apple's. Bright things behind are pulled down further (body_shade 0.45
    -- where Apple's dark glass has 0.28), because everything written on
    -- these panes is white and stays white. And there is no white over it:
    -- an eighth of white made the blur look milky and a dark wallpaper grey.
    local pane_glass = {
      body_shade = 0.45,
      tint_over_dark = 0,
      tint_over_bright = 0,
    }
    local pane = apple and { inherits = apple_glass } or regular and { inherits = "regular" } or {
      inherits = "regular27",
      dark = cloud and { body_shade = 0.45 } or current and lucid(0.45) or pane_glass,
    }
    -- Every pane reads the engine's backdrop: the wallpaper alone, drawn once
    -- and again only when it changes, and over a window the window too, from
    -- a clean copy of the frame (the engine's Beneath). The desktop's widgets
    -- never show through any glass (2026-10-18: "You should not be able to see
    -- the widgets through other liquid glass effects"; "you should be able to
    -- see other windows through other windows when they overlap, just not for
    -- the widgets in the wallpaper"). see_windows = off in the tuning gives
    -- the wallpaper alone again.
    pane.xray = true
    hg.preset("pane", pane)
    -- The glass of a window: the regular material, toned far less heavily. A
    -- pane as large as a window, toned as a small one is, is a dark slab that
    -- nothing comes through, which is a colour again. A window's own
    -- background adds a quarter of black to this (the override, for the apps
    -- it can reach), and together they come to about half: the shapes and
    -- colours behind a window still show, and the frost, and what is bright
    -- behind being dimmed nearly all the way, keep white text on it readable
    -- over a launch in full daylight too.
    -- Its rim is lit a little more than a panel's: round a window the rim
    -- is all there is of the glass's edge, and on the window that has the
    -- focus it has to be seen at a glance.
    -- Windows read the backdrop too (2026-10-18: "I don't think you should be
    -- able to see widgets in the render behind windows, only where the window
    -- doesn't exist"), and the windows under them: never the desktop's cards,
    -- and a clock that ticks behind a window costs no new copy of the screen.
    hg.preset("window", apple and { inherits = apple_glass, xray = true } or not current and {
      inherits = "regular",
      xray = true,
      blur_strength = 1.3,
      bevel_strength = 0.9,
      specular_strength = 0.8,
      dark = { tint_color = 0x16161659, adaptive_dim = 0.9 },
    } or {
      inherits = "regular27",
      xray = true,
      dark = cloud and { body_shade = 0.6, frost_cloud = 0.92 } or lucid(0.4),
    })
    hg.config({
      -- Windows are glass too. A window that is see-through (a terminal, the
      -- files app, anything the override makes clear) has a pane behind the
      -- whole of it. Every window, solid or not, has its outline made of
      -- glass: the band Hyprland keeps for the border (windows.border; the
      -- border's own colours are made clear below). A window without the
      -- focus has its rim lit less, which is what tells the focused one
      -- from the others now that no colour does.
      enabled = true,
      default_theme = "dark",
      default_preset = "window",
      layers = { enabled = true, live_resample_fps = 60 },
      windows = { border = true, border_dim = 0.4 },
    })
    for namespace, cut in pairs(panes) do
      hg.layer(namespace, { preset = "pane", mask_threshold = cut, mask_feather = 0 })
    end
  end)
end
-- Windows that pour into the bottom of the screen as they close, land like
-- jelly as they open and wobble as they are dragged, and every animation at
-- one pace (2026-10-24: "add more animations into the desktop like the genie
-- in a bottle animation effect ... and maybe window wiggle/jiggle jelly
-- effects as well, all configurable in the settings"). Which of them run, and
-- how quickly, is the tuning's: the Motion group of Vetro Glass, or
-- ~/.config/vetro/glass.conf. Only an engine that has them has hg.motion.
local moving = liquid and hg.motion ~= nil and pcall(hg.motion, { enabled = true })

-- Blur behind every translucent surface: the glass under Omarchy's bar and
-- the panels that drop from the bar, Vetro's notifications and volume display,
-- the launch addon's cards, and Omarchy's own glass. ignore_alpha keeps the
-- blur to the panel itself, not its shadow.
-- With the real glass drawing, the bar's own strip is glass from the
-- backdrop, and only what hangs from the bar (tooltips, tray menus) keeps
-- the compositor's blur: blurred twice, glass is frosted, not clear. A blur
-- on the bar itself also had Hyprland blur the whole wallpaper again each
-- time anything on the desktop changed (the launch countdown, every second).
local glass = {
  ["^omarchy-bar$"] = 0.3,
  ["^vetro:bar$"] = 0.3,
}
if not liquid then
  glass["^omarchy-image-selector$"] = 0.3
  glass["^omarchy-(menu|notifications|osd|polkit|clipboard|emojis|keyboard-panel|reminders|network-qr)$"] = 0.3
  glass["^vetro:barglass$"] = 0.41
  glass["^hyprland-shell:(bar|notifs|osd)$"] = 0.41
  glass["^vetro:center$"] = 0.41
  glass["^vetro:(launch|weather)$"] = 0.01
end
local popups_only = { ["^omarchy-bar$"] = true, ["^vetro:bar$"] = true }
for namespace, alpha in pairs(glass) do
  local own = not (liquid and popups_only[namespace])
  hl.layer_rule({ match = { namespace = namespace }, blur = own, blur_popups = true, ignore_alpha = alpha })
end
-- Behind the menu (Super+Space) the whole screen is blurred, by a surface of
-- its own that the menu puts under itself (shell/vetro.menu). With the
-- engine it is glass (the panes above); without it, the compositor's blur.
if not liquid then
  hl.layer_rule({ match = { namespace = "^vetro:backdrop$" }, blur = true, ignore_alpha = 0 })
end
-- The bar's glass is part of the bar, which Omarchy keeps instant
hl.layer_rule({ match = { namespace = "^vetro:barglass$" }, no_anim = true, animation = "none" })
hl.layer_rule({ match = { namespace = "^vetro:bar$" }, no_anim = true, animation = "none" })
-- Each toast is a surface of its own (shell/vetro.lib/ToastLayer.qml), slid in
-- from the right edge and back out whole, as the centre is: its card stands
-- still in it, so the glass engine builds its pane once
hl.layer_rule({ match = { namespace = "^hyprland-shell:notifs$" }, animation = "slide right" })
-- The notification centre is slid in whole from the right edge, at the speed
-- of layersIn and layersOut below: its card stays still in its surface, so
-- the glass engine builds its pane once and not on every frame of the slide
-- (shell/vetro.center)
hl.layer_rule({ match = { namespace = "^vetro:center$" }, animation = "slide right" })
-- The screen of glass behind the menu comes at once, as it always has, and
-- only the menu on it fades in. Fading the whole screen of glass as well had
-- the engine draw its lens again for every frame of the fade: three times
-- the work of opening the menu at all (2026-10-22).
hl.layer_rule({ match = { namespace = "^vetro:backdrop$" }, no_anim = true, animation = "none" })

-- Fast, snappy and still fluid (2026-10-18: "make everything feel fast,
-- snappy and yet fluid, so very responsive"): Omarchy's animations about a
-- third shorter, on a curve that leaves at once and lands softly, and windows
-- opening with a hint of spring. Speeds are tenths of a second. Workspaces
-- keep sliding, as the dotfiles' feel.lua asks, only quicker.
-- Menus, panels and pickers fade in over 0.18 s on vetroGlide, which leaves
-- at once and lands softly (2026-10-22, of Super+K: "there is not animations
-- and it feels slow and clunky"): on vetroSnap the fade was over within a
-- frame, which read as no animation at all.
-- Every curve named here is defined here: a missing one is a config error,
-- and a config error leaves Hyprland with no keybinds.
-- With the engine's motion, windows move on springs instead (2026-10-24: "we
-- need smoother and silkier animations"): a spring keeps its speed when it is
-- sent somewhere new halfway, where a curve starts again from rest, so a
-- window dragged, tiled and tiled again flows from one place to the next.
-- vetroMove all but stops dead and vetroPop overshoots a little, as a bubble
-- does. Each settles in about a quarter of a second, at the pace of the
-- tuning. Hyprland's springs take no speed, but the field is still asked for.
-- Workspaces stay on vetroSnap: on a spring they were slow to switch
-- (2026-10-27: "make the workspace switching faster (like it was before)").
if hl.curve and hl.animation then
  hl.curve("vetroSnap", { type = "bezier", points = { { 0.05, 0.9 }, { 0.1, 1.0 } } })
  hl.curve("vetroSpring", { type = "bezier", points = { { 0.05, 0.9 }, { 0.1, 1.05 } } })
  hl.curve("vetroGlide", { type = "bezier", points = { { 0.25, 1.0 }, { 0.5, 1.0 } } })
  local sprung = function(each)
    return each
  end
  if moving then
    hl.curve("vetroMove", { type = "spring", stiffness = 360, dampening = 35, mass = 1 })
    hl.curve("vetroPop", { type = "spring", stiffness = 300, dampening = 26, mass = 1 })
    local springs = { windows = "vetroMove", windowsIn = "vetroPop" }
    sprung = function(each)
      if springs[each.leaf] then
        each.bezier, each.spring = nil, springs[each.leaf]
      end
      return each
    end
  end
  local motion = {
    { leaf = "windows", speed = 2.6, bezier = "vetroSnap" },
    { leaf = "windowsIn", speed = 2.6, bezier = "vetroSpring", style = "popin 90%" },
    { leaf = "windowsOut", speed = 1.3, bezier = "vetroSnap", style = "popin 90%" },
    { leaf = "border", speed = 4, bezier = "vetroSnap" },
    { leaf = "fade", speed = 2.2, bezier = "vetroSnap" },
    { leaf = "fadeIn", speed = 1.4, bezier = "vetroSnap" },
    { leaf = "fadeOut", speed = 1.2, bezier = "vetroSnap" },
    { leaf = "layers", speed = 2.6, bezier = "vetroSnap" },
    { leaf = "layersIn", speed = 1.8, bezier = "vetroGlide", style = "fade" },
    { leaf = "layersOut", speed = 1.2, bezier = "vetroGlide", style = "fade" },
    { leaf = "fadeLayersIn", speed = 1.8, bezier = "vetroGlide" },
    { leaf = "fadeLayersOut", speed = 1.2, bezier = "vetroGlide" },
    { leaf = "workspaces", speed = 2.8, bezier = "vetroSnap", style = "slide" },
    { leaf = "specialWorkspace", speed = 2.6, bezier = "vetroSnap", style = "slidevert" },
  }
  for _, each in ipairs(motion) do
    each.enabled = true
    hl.animation(sprung(each))
  end
end

-- Super+K, Omarchy's list of keybindings, opens in Vetro's menu the moment
-- the key goes down: the menu answers this global shortcut
-- (shell/vetro.menu), where Omarchy's command started bash, hyprctl, perl
-- and a second Quickshell before the menu could open, 130 ms or more
-- (2026-10-22: "takes a super long time to open"). The list, and what a
-- choice from it runs, are still Omarchy's own. With Vetro off this file
-- stands down, and the key runs Omarchy's command again.
hl.unbind("SUPER + K")
hl.bind("SUPER + K", hl.dsp.global("vetro:keybindings"), { description = "Keybindings" })

-- Squircles over squares: every window corner rounded, on the superellipse
-- curve (rounding_power 3.5) rather than a plain quarter circle. Omarchy's
-- shell takes its own corner radius from this value, so its menu, panels,
-- tooltips and polkit dialog round to match; Vetro's own glass is bar.radius
-- and panels.radius in ~/.config/vetro/shell.json (the bar's strip a full
-- capsule at 15, the cards at 20, like the windows).
-- Not while Omarchy's "window gaps" toggle is off: that asks for square,
-- edge-to-edge windows.
if not exists((os.getenv("XDG_STATE_HOME") or (home .. "/.local/state")) .. "/omarchy/toggles/hypr/window-no-gaps.lua") then
  hl.config({
    decoration = {
      rounding = 20,
      rounding_power = 3.5,
    },
  })
end

-- The outline of a window is glass, and not a colour. Hyprland's own border
-- is made clear and a pixel wider, and the engine fills the band it keeps
-- with glass (windows.border above): a rim that catches the light, brighter
-- on the window that has the focus. Only while the engine draws: without it
-- the theme's coloured border stays, or a window would have no edge at all.
-- Not while Omarchy's "window gaps" toggle is off either, which asks for no
-- border.
-- A window's shadow is soft and wide, as a Mac's is, not Omarchy's tight
-- ring of near black (range 2), which round the glass read as a dark outline
-- (2026-10-18: "they look weird and kinda have like a dark outline").
if liquid then
  local clear = "rgba(00000000)"
  hl.config({ decoration = { shadow = { range = 18, render_power = 2, color = "rgba(00000040)", offset = "0 3" } } })
  hl.config({ general = { col = { active_border = clear, inactive_border = clear } } })
  if not exists((os.getenv("XDG_STATE_HOME") or (home .. "/.local/state")) .. "/omarchy/toggles/hypr/window-no-gaps.lua") then
    hl.config({ general = { border_size = 2 } })
  end
end

-- Glass shows only behind the apps that draw a clear background themselves:
-- the terminal, GTK apps (the files app among them) and Qt apps, which the
-- override makes see-through (override/README.md). Every other window is
-- solid (2026-10-18: "i don't want it for browsers or apps like that, just
-- terminal files, and other apps that use liquid glass"): Omarchy's few
-- percent of see-through for every window (its default-opacity tag) and the
-- browsers' own are made whole, so the compositor draws nothing under them
-- and the engine gives them their outline of glass alone. A solid window
-- costs nothing beneath it; under a see-through one the wallpaper, the cards
-- and the pane were drawn again for every frame the window changed.
-- Super+Backspace still toggles a window solid, through its opaque property.
--
-- `touch ~/.config/vetro/browser-glass` (then `hyprctl reload`) makes browsers
-- glass again, the only way a browser can be: the whole window a little
-- see-through over its pane, pages and all.
local browser_glass = exists(home .. "/.config/vetro/browser-glass")
if liquid and type(o) == "table" and o.window then
  o.window({ tag = "default-opacity" }, { opacity = "1 1" })
  local browser = browser_glass and "0.92 0.88" or "1 1"
  o.window({ tag = "chromium-based-browser" }, { opacity = browser })
  o.window({ tag = "firefox-based-browser" }, { opacity = browser })
end
-- Browsers and Electron apps draw their whole window opaque, but into a
-- buffer that has alpha all the same, so the engine took them for
-- see-through: it drew a pane of glass under every one of them on every frame
-- they changed, never seen, and Hyprland drew everything under them as well
-- (2026-10-23: 3.5 ms of every frame's GPU with Brave on screen, the most of
-- why Omarchy's panels opened choppily over it). Tagged solid, they get their
-- outline of glass alone, and what is under them is not drawn. Needs an
-- engine that knows the tag; an older one leaves the tag be.
if liquid and type(o) == "table" and o.window then
  if not browser_glass then
    o.window({ tag = "chromium-based-browser" }, { tag = "+hyprglass_solid" })
    o.window({ tag = "firefox-based-browser" }, { tag = "+hyprglass_solid" })
  end
  o.window({ class = "^(discord|vesktop|chatgpt|Notesnook|Slack|signal|Signal|obsidian|Spotify|spotify|Code|code-oss|Cursor|Bitwarden|Element|Typora|zoom|teams-for-linux)$" }, { tag = "+hyprglass_solid" })
end

------------------------------------------------------------- cursor, Qt -----

-- Each only once it is installed: naming a theme that is not there would lose
-- Omarchy's own instead.
local cursor = installed("icons", "XCursor-Pro-Dark")
local hyprcursor = installed("icons", "XCursor-Pro-Hyprcursor-Dark")

if cursor then
  hl.env("XCURSOR_THEME", "XCursor-Pro-Dark")
  hl.env("XCURSOR_SIZE", "24")
end
if hyprcursor then
  hl.env("HYPRCURSOR_THEME", "XCursor-Pro-Hyprcursor-Dark")
  hl.env("HYPRCURSOR_SIZE", "24")
end

-- Qt apps in Vetro's Kvantum theme through qt6ct, instead of Omarchy's
-- GTK-following Qt
if o.cmd_present("qt6ct") then
  hl.env("QT_QPA_PLATFORMTHEME", "qt6ct")
end

--------------------------------------------------------------- autostart ----

hl.on("hyprland.start", function()
  -- the glass engine, from outside this file (see "liquid glass" above)
  hl.exec_cmd("vetro glass load")
  if hyprcursor then
    hl.exec_cmd("hyprctl setcursor XCursor-Pro-Hyprcursor-Dark 24")
  elseif cursor then
    hl.exec_cmd("hyprctl setcursor XCursor-Pro-Dark 24")
  end
  -- The launch addon's wallpaper and the data behind its cards, where the
  -- addon is installed.
  if exists(home .. "/.config/systemd/user/vetro-launchd.service") then
    hl.exec_cmd("systemctl --user restart vetro-launchd.service")
  end
end)

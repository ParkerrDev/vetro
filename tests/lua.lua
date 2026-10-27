-- Runs hypr/vetro.lua against a stand-in for Hyprland's Lua API, in plain Lua,
-- and checks what it asks of it.
--
--   bash tests/lint.sh runs it, twice, each time in a home folder it makes.
--
-- It writes into HOME and XDG_RUNTIME_DIR, and removes whole folders there
-- (~/.config/systemd, ~/.local/share/icons, ~/.config/vetro and Omarchy's
-- toggles) to put its scratch home back as it was. Run in a real home folder
-- it would delete those, so it refuses to run anywhere but in two folders
-- made for it, each holding the mark `.vetro-test-scratch`:
--
--   d=$(mktemp -d); mkdir "$d/home" "$d/run"
--   touch "$d/home/.vetro-test-scratch" "$d/run/.vetro-test-scratch"
--   env -i PATH="$PATH" HOME="$d/home" XDG_RUNTIME_DIR="$d/run" lua tests/lua.lua .
--
-- It cannot show that the glass looks right. It does show that every path
-- through the file runs without an error, that the file never loads the engine
-- (Hyprland reads its config again for each plugin loaded, so a config that
-- loads one never stops being read), that the guard against a config read in a
-- loop stands down when it should, that no theme is looked at, and that every
-- option and rule field is one this machine's Hyprland knows (when its API
-- description is here).

local repo = arg[1] or "."
local home = assert(os.getenv("HOME"), "HOME must point at an empty directory")
local runtime = assert(os.getenv("XDG_RUNTIME_DIR"), "XDG_RUNTIME_DIR must point at an empty directory")
-- The file looks for its state where XDG_STATE_HOME says, so the test writes
-- there, and only if that is inside the scratch home.
local state_home = os.getenv("XDG_STATE_HOME") or (home .. "/.local/state")
assert(state_home:sub(1, #home) == home, "XDG_STATE_HOME must be inside HOME")

-- Nothing below this runs in a folder that was not made for it.
local function made_for_this(dir)
  local mark = io.open(dir .. "/.vetro-test-scratch", "r")
  if mark then
    mark:close()
  end
  return mark ~= nil
end
if not (made_for_this(home) and made_for_this(runtime)) then
  io.stderr:write("tests/lua.lua removes folders in HOME and XDG_RUNTIME_DIR, and runs only in two\n"
    .. "made for it (each holding .vetro-test-scratch). bash tests/lint.sh makes them.\n")
  os.exit(2)
end
local failures = 0
local checks = 0

local function check(what, ok)
  checks = checks + 1
  if not ok then
    failures = failures + 1
    print("FAIL  " .. what)
  end
end

local function write(path, text)
  os.execute("mkdir -p '" .. path:match("^(.*)/[^/]*$") .. "'")
  local file = assert(io.open(path, "w"))
  file:write(text)
  file:close()
end

local function remove(path)
  os.execute("rm -rf '" .. path .. "'")
end

local function read(path)
  local file = io.open(path, "r")
  if not file then
    return ""
  end
  local text = file:read("*a")
  file:close()
  return text
end

-- A fresh stand-in for `hl`, recording every call. `opts.plugin` puts a
-- hyprglass in place, as it is once the engine has been loaded from outside;
-- `opts.motion` makes it an engine with the motion.
-- Loading is recorded so that the tests can hold the file to never doing it.
local function fake(opts)
  local seen = {
    config = {}, rules = {}, env = {}, hooks = {}, commands = {}, loads = {}, windows = {}, curves = {}, animations = {}, binds = {},
    glass = { config = {}, layers = {}, presets = {} },
  }
  local glass = {
    config = function(t) seen.glass.config[#seen.glass.config + 1] = t end,
    layer = function(ns, t) seen.glass.layers[ns] = t or {} end,
    preset = function(name, t) seen.glass.presets[name] = t or {} end,
  }
  if opts.motion then
    glass.motion = function(t) seen.glass.motion = t end
  end
  local plugins = { hyprglass = opts.plugin and glass or nil }
  plugins.load = function(path)
    seen.loads[#seen.loads + 1] = path
    plugins.hyprglass = glass
  end
  hl = {
    plugin = plugins,
    on = function(event, handler) seen.hooks[event] = handler end,
    exec_cmd = function(command) seen.commands[#seen.commands + 1] = command end,
    config = function(t) seen.config[#seen.config + 1] = t end,
    layer_rule = function(t) seen.rules[#seen.rules + 1] = t end,
    env = function(name, value) seen.env[name] = value end,
    curve = function(name, t) seen.curves[name] = t end,
    animation = function(t) seen.animations[t.leaf] = t end,
    bind = function(keys, action, t) seen.binds[keys] = { action = action, opts = t } end,
    unbind = function(keys) seen.binds[keys] = false end,
    dsp = { global = function(name) return { global = name } end },
  }
  -- Omarchy's own helper table, as the Lua file finds it.
  o = {
    cmd_present = function(name) return opts.qt6ct and name == "qt6ct" end,
    window = function(match, rule) seen.windows[#seen.windows + 1] = { match = match, rule = rule } end,
  }
  return seen
end

local vetro = assert(loadfile(repo .. "/hypr/vetro.lua"))
local function run(opts)
  opts = opts or {}
  local seen = fake(opts)
  -- Each run stands for a reading on a quiet day, unless the test is about
  -- many readings in a row.
  if not opts.keep_tally then
    remove(runtime .. "/vetro-glass-readings")
    remove(runtime .. "/vetro-glass-stood-down")
  end
  local ok, err = pcall(vetro)
  check("hypr/vetro.lua runs: " .. tostring(err), ok)
  return seen
end

-- rule_for(seen, namespace, field): the first rule for the namespace, that has
-- the field if one is named.
local function rule_for(seen, namespace, field)
  for _, rule in ipairs(seen.rules) do
    if rule.match.namespace == namespace and (field == nil or rule[field] ~= nil) then
      return rule
    end
  end
end

local function exists(path)
  local file = io.open(path, "r")
  if file then
    file:close()
    return true
  end
  return false
end

-- The cursor theme may be installed on the machine the test runs on, which
-- the file also looks at.
local system_hyprcursor = exists("/usr/share/icons/XCursor-Pro-Hyprcursor-Dark")
local system_cursor = exists("/usr/share/icons/XCursor-Pro-Dark")

local function sorted_keys(t)
  local keys = {}
  for key in pairs(t) do
    keys[#keys + 1] = key
  end
  table.sort(keys)
  return keys
end

local function dump(value)
  if type(value) == "function" then
    return "function"
  end
  if type(value) ~= "table" then
    return tostring(value)
  end
  local parts = {}
  for _, key in ipairs(sorted_keys(value)) do
    parts[#parts + 1] = tostring(key) .. "=" .. dump(value[key])
  end
  return "{" .. table.concat(parts, ",") .. "}"
end

local state = state_home .. "/vetro"
local seen

-- Switched off: it touches nothing, and asks for nothing.
write(state .. "/off", "")
seen = run({ plugin = true, qt6ct = true })
check("off: no options set", #seen.config == 0)
check("off: no layer rules", #seen.rules == 0)
check("off: no environment", next(seen.env) == nil)
check("off: no start hook", next(seen.hooks) == nil)
check("off: Omarchy's animations left alone", next(seen.animations) == nil)
check("off: the glass is not configured", #seen.glass.config == 0 and next(seen.glass.layers) == nil)
remove(state .. "/off")

-- On, with no engine in Hyprland: blur stands in for the glass everywhere, and
-- the file loads nothing.
seen = run()
check("on, no engine: blur is set", seen.config[1] and seen.config[1].decoration.blur.enabled == true)
check("on, no engine: the file loads no plugin", #seen.loads == 0)
check("on, no engine: the glass is not configured", #seen.glass.config == 0)
local barglass = rule_for(seen, "^vetro:barglass$", "blur")
check("on, no engine: the bar's pane has blur", barglass and barglass.blur == true and barglass.ignore_alpha == 0.41)
local nomotion = rule_for(seen, "^vetro:barglass$", "no_anim")
check("on, no engine: the pane has no animation", nomotion and nomotion.no_anim == true and nomotion.animation == "none")
check("on, no engine: the launch cards have blur",
  rule_for(seen, "^vetro:(launch|weather)$") and rule_for(seen, "^vetro:(launch|weather)$").ignore_alpha == 0.01)
check("on, no engine: Omarchy's cards have blur",
  rule_for(seen, "^omarchy-(menu|notifications|osd|polkit|clipboard|emojis|keyboard-panel|reminders|network-qr)$") ~= nil)
check("on, no engine: the bar keeps its blur", rule_for(seen, "^omarchy-bar$") ~= nil)

-- The corners.
local rounded
for _, group in ipairs(seen.config) do
  if group.decoration and group.decoration.rounding then
    rounded = group.decoration
  end
end
check("on: windows are rounded at 20 on the superellipse", rounded and rounded.rounding == 20 and rounded.rounding_power == 3.5)
local opening = seen.animations.windowsIn
check("on: windows open quicker than Omarchy's 4.1, on the spring", opening and opening.enabled and opening.speed < 4.1 and opening.bezier == "vetroSpring")
check("on: the spring curve is defined", seen.curves.vetroSpring ~= nil and seen.curves.vetroSnap ~= nil)
check("on: workspaces still slide", seen.animations.workspaces and seen.animations.workspaces.style == "slide")
check("on, an engine without the motion: no spring is asked of Hyprland", seen.curves.vetroMove == nil and seen.animations.windows.spring == nil)
write(state_home .. "/omarchy/toggles/hypr/window-no-gaps.lua", "")
seen = run()
local square = true
for _, group in ipairs(seen.config) do
  if group.decoration and group.decoration.rounding then
    square = false
  end
end
check("on: Omarchy's no-gaps toggle keeps the windows square", square)
remove(state_home .. "/omarchy/toggles")

-- At login the engine is asked for through a command, after Hyprland is up,
-- and never by the file itself.
check("login: a start hook is set", type(seen.hooks["hyprland.start"]) == "function")
seen.hooks["hyprland.start"]()
check("login: the hook runs vetro glass load", seen.commands[1] == "vetro glass load")
check("login: and loads nothing itself", #seen.loads == 0)
local without_addon = #seen.commands
check("login: without the launch addon no service is restarted", seen.commands[#seen.commands] ~= "systemctl --user restart vetro-launchd.service")
write(home .. "/.config/systemd/user/vetro-launchd.service", "[Service]\n")
seen = run()
seen.hooks["hyprland.start"]()
check("login: with the launch addon its service is restarted", seen.commands[#seen.commands] == "systemctl --user restart vetro-launchd.service" and #seen.commands == without_addon + 1)
remove(home .. "/.config/systemd")
write(home .. "/.local/share/icons/XCursor-Pro-Hyprcursor-Dark/index.theme", "")
seen = run()
seen.hooks["hyprland.start"]()
check("login: the cursor is set when its theme is installed", seen.commands[2] == "hyprctl setcursor XCursor-Pro-Hyprcursor-Dark 24")
check("login: and its environment", seen.env.HYPRCURSOR_THEME == "XCursor-Pro-Hyprcursor-Dark")
remove(home .. "/.local/share/icons")
seen = run()
check("no cursor theme, no cursor environment (unless the system has one)",
  (seen.env.HYPRCURSOR_THEME ~= nil) == system_hyprcursor and (seen.env.XCURSOR_THEME ~= nil) == system_cursor)
seen = run({ qt6ct = true })
check("Qt apps go through qt6ct where it is installed", seen.env.QT_QPA_PLATFORMTHEME == "qt6ct")
seen = run()
check("and not where it is not", seen.env.QT_QPA_PLATFORMTHEME == nil)

-- An engine with the motion: it is turned on, and windows move on springs;
-- workspaces, fades and layers keep their curves.
seen = run({ plugin = true, motion = true })
check("motion: turned on", seen.glass.motion and seen.glass.motion.enabled == true)
local animations = seen.animations
check("motion: windows move on a spring", animations.windows.spring == "vetroMove" and animations.windows.bezier == nil)
check("motion: and open on one that overshoots a little, still popping in",
  animations.windowsIn.spring == "vetroPop" and animations.windowsIn.style == "popin 90%")
check("motion: workspaces slide on the quick curve, not a spring",
  animations.workspaces.bezier == "vetroSnap" and animations.workspaces.spring == nil and animations.workspaces.style == "slide")
check("motion: every spring asked for is defined as one", (function()
  for _, each in pairs(animations) do
    local curve = each.spring and seen.curves[each.spring]
    if each.spring and not (curve and curve.type == "spring" and curve.stiffness > 0.5 and curve.dampening > 0.5 and curve.mass > 0.5) then
      return false
    end
  end
  return true
end)())
check("motion: fades keep their curve", animations.fadeIn.bezier == "vetroSnap" and animations.fadeIn.spring == nil)
check("motion: and so does a closing window", animations.windowsOut.bezier == "vetroSnap")

-- The engine loaded from outside: set up for the panes, and for nothing else.
seen = run({ plugin = true })
check("loaded: not loaded again", #seen.loads == 0)
local set = seen.glass.config[1] or { layers = {}, windows = {} }
check("loaded: the engine is set up once", #seen.glass.config == 1)
check("loaded: windows are glass", set.enabled == true)
check("loaded: layers are", set.layers.enabled == true)
check("loaded: dark", set.default_theme == "dark")
local window = seen.glass.presets.window or { dark = {} }
check("loaded: a window's glass is the tuned glass on Apple's shader too", window.inherits == "tuned" and window.dark == nil)
check("loaded: and it reads the backdrop, so no widget or window behind shows through", window.xray == true)
check("loaded: and it is the glass a window gets", set.default_preset == "window")
check("loaded: a window's outline is glass", set.windows.border == true)
check("loaded: lit less on a window without the focus", set.windows.border_dim == 0.4)
local expected = {
  "homescreen", "hyprland-shell:bar", "hyprland-shell:notifs", "hyprland-shell:osd", "omarchy-clipboard",
  "omarchy-emojis", "omarchy-image-selector", "omarchy-keyboard-panel", "omarchy-menu", "omarchy-network-qr",
  "omarchy-notifications", "omarchy-osd", "omarchy-polkit", "omarchy-reminders", "vetro:backdrop", "vetro:bar",
  "vetro:barglass", "vetro:center", "vetro:launch", "vetro:weather",
}
check("loaded: exactly these panes get glass", dump(sorted_keys(seen.glass.layers)) == dump(expected))
local glass = seen.glass.presets.pane or { dark = {} }
check("loaded: a pane's glass is the tuned glass on Apple's shader", glass.inherits == "tuned" and glass.dark == nil)
for pane, options in pairs(seen.glass.layers) do
  check("loaded: " .. pane .. " gets a pane's glass", options.preset == "pane" and options.mask_feather == 0)
end
check("loaded: every pane reads the backdrop, so no widget shows through any glass", glass.xray == true)
check("loaded: the menu's backdrop is glass, not the compositor's blur", rule_for(seen, "^vetro:backdrop$", "blur") == nil and seen.glass.layers["vetro:backdrop"].mask_threshold == 0.01)
check("loaded: so is the picture chooser's dim", rule_for(seen, "^omarchy-image-selector$", "blur") == nil)
check("loaded: a pane on clear air is cut at 0.10", seen.glass.layers["vetro:barglass"].mask_threshold == 0.10)
check("loaded: the strip in Vetro's own bar is cut the same", seen.glass.layers["vetro:bar"].mask_threshold == 0.10)
local own_bar = rule_for(seen, "^vetro:bar$", "blur")
check("loaded: Vetro's bar keeps the blur for what hangs from it, as Omarchy's does",
  own_bar and own_bar.blur_popups == true and own_bar.ignore_alpha == 0.3)
check("loaded: but not on itself, which would blur the wallpaper again each time the desktop changes", own_bar and own_bar.blur == false)
local still = rule_for(seen, "^vetro:bar$", "no_anim")
check("loaded: and it is not animated", still and still.no_anim == true and still.animation == "none")
check("loaded: a card on a dim is cut between the dim and the card", seen.glass.layers["omarchy-menu"].mask_threshold == 0.155)
check("loaded: the password prompt is cut as the other cards on a dim are", seen.glass.layers["omarchy-polkit"].mask_threshold == 0.155)
check("loaded: the bar keeps its blur for what hangs from it", rule_for(seen, "^omarchy-bar$") ~= nil)
check("loaded: the cards do not get the blur too", rule_for(seen, "^vetro:barglass$", "blur") == nil)
check("loaded: the desktop's cards are cut as the panels are", seen.glass.layers["homescreen"].mask_threshold == 0.10)

-- The outline of a window: Hyprland's own border made clear and two wide,
-- for the engine to fill, and only while the engine draws.
local function general(run_seen, field)
  for _, group in ipairs(run_seen.config) do
    if group.general and group.general[field] ~= nil then
      return group.general[field]
    end
  end
end
local colours = general(seen, "col") or {}
check("loaded: the border of the focused window has no colour", colours.active_border == "rgba(00000000)")
check("loaded: nor the others'", colours.inactive_border == "rgba(00000000)")
check("loaded: the border is two wide, a lit rim", general(seen, "border_size") == 2)
local function decoration(run_seen, field)
  for _, group in ipairs(run_seen.config) do
    if group.decoration and group.decoration[field] ~= nil then
      return group.decoration[field]
    end
  end
end
local shadow = decoration(seen, "shadow") or {}
check("loaded: a window's shadow is soft and wide, not a dark ring", shadow.range == 18 and shadow.color == "rgba(00000040)")
check("no engine: Omarchy's shadow stays", decoration(run(), "shadow") == nil)
local plain = run()
check("no engine: the theme's border keeps its colour", general(plain, "col") == nil)
check("no engine: and its width", general(plain, "border_size") == nil)
write(state_home .. "/omarchy/toggles/hypr/window-no-gaps.lua", "")
local gapless = run({ plugin = true })
check("loaded, with Omarchy's no-gaps toggle: no border is asked for", general(gapless, "border_size") == nil)
remove(state_home .. "/omarchy/toggles")

-- Only apps that draw a clear background are glass: every other window, the
-- browsers among them, is made solid, unless its owner wants browsers glass.
local function opacity_of(run_seen, tag)
  for _, entry in ipairs(run_seen.windows) do
    if entry.match.tag == tag then
      return entry.rule.opacity
    end
  end
end
check("loaded: Omarchy's few percent of see-through is made whole", opacity_of(seen, "default-opacity") == "1 1")
check("loaded: browsers are solid", opacity_of(seen, "chromium-based-browser") == "1 1" and opacity_of(seen, "firefox-based-browser") == "1 1")
local opacities, solid = 0, {}
for _, entry in ipairs(seen.windows) do
  if entry.rule.opacity then
    opacities = opacities + 1
  end
  if entry.rule.tag == "+hyprglass_solid" then
    solid[#solid + 1] = entry.match.tag or entry.match.class
  end
end
check("loaded: and no other window is given an opacity", opacities == 3)
check("loaded: browsers and Electron apps are glass at their outline alone", #solid == 3 and solid[1] == "chromium-based-browser" and solid[2] == "firefox-based-browser")
check("no engine: windows are left as Omarchy has them", #plain.windows == 0)
write(home .. "/.config/vetro/browser-glass", "")
local glassy = run({ plugin = true })
check("browser-glass: browsers are a little see-through", opacity_of(glassy, "chromium-based-browser") == "0.92 0.88" and opacity_of(glassy, "firefox-based-browser") == "0.92 0.88")
check("browser-glass: the other windows stay solid", opacity_of(glassy, "default-opacity") == "1 1")
check("browser-glass: and every window keeps its outline of glass", (glassy.glass.config[1] or { windows = {} }).windows.border == true)
remove(home .. "/.config/vetro")
-- macOS 27's recipe as transcribed, the default until the lens.
write(home .. "/.config/vetro/macos-material", "")
local macos = run({ plugin = true })
check("macos-material: panes are macOS 27's recipe", (macos.glass.presets.pane or {}).inherits == "apple27")
check("macos-material: and windows too, through the backdrop", (macos.glass.presets.window or {}).inherits == "apple27" and macos.glass.presets.window.xray == true)
remove(home .. "/.config/vetro")
-- The panes as they were until 2026-10-18: Apple's current material as measured.
write(home .. "/.config/vetro/liquid-material", "")
local liquid = run({ plugin = true }).glass.presets.pane or { dark = {} }
check("liquid-material: a pane's glass is Apple's current material", liquid.inherits == "regular27")
check("liquid-material: with no white over it", liquid.dark.tint_over_dark == 0 and liquid.dark.tint_over_bright == 0)
check("liquid-material: and its blur as it was measured", liquid.dark.frost_cloud == nil and liquid.dark.frost_wide == nil)
check("liquid-material: and bright things pulled down for white text", liquid.dark.body_shade == 0.45)
local liquidWindow = run({ plugin = true }).glass.presets.window or { dark = {} }
check("liquid-material: a window's glass is the regular kind", liquidWindow.inherits == "regular")
check("liquid-material: toned far less than a panel's, a quarter of black", liquidWindow.dark.tint_color == 0x16161659)
check("liquid-material: with what is bright behind it dimmed", liquidWindow.dark.adaptive_dim == 0.9)
check("liquid-material: and nothing of the current material's", liquidWindow.dark.frost_cloud == nil and liquidWindow.dark.body_shade == nil)
remove(home .. "/.config/vetro")
-- The other glasses that `vetro glass look` chooses: the regular kind on the
-- panes too, and three that are the current material on windows too.
write(home .. "/.config/vetro/regular-material", "")
local regular = run({ plugin = true }).glass.presets
check("regular-material: a pane is the regular kind", (regular.pane or {}).inherits == "regular" and regular.pane.dark == nil)
check("regular-material: and a window is as it comes", (regular.window or {}).inherits == "regular" and regular.window.dark.tint_color == 0x16161659)
write(home .. "/.config/vetro/cloud-material", "")
check("cloud-material: comes before the regular one", run({ plugin = true }).glass.presets.pane.inherits == "regular27")
remove(home .. "/.config/vetro")
write(home .. "/.config/vetro/cloud-material", "")
local clouded = run({ plugin = true }).glass.presets
check("cloud-material: a pane is the current material as it was measured", (clouded.pane or {}).inherits == "regular27")
check("cloud-material: its cloud and its white left as they are", clouded.pane.dark.frost_cloud == nil and clouded.pane.dark.tint_over_dark == nil)
check("cloud-material: with bright things pulled down for white text", clouded.pane.dark.body_shade == 0.45)
check("cloud-material: a window shows less of what is behind it", clouded.window.dark.frost_cloud == 0.92 and clouded.window.dark.body_shade == 0.6)
remove(home .. "/.config/vetro")
write(home .. "/.config/vetro/clear-material", "")
local cleared = run({ plugin = true }).glass.presets
check("clear-material: what is behind a window is one soft copy", cleared.window.inherits == "regular27" and cleared.window.dark.frost_cloud == 1)
check("clear-material: only a little out of focus", cleared.window.dark.frost_wide == 4 and cleared.pane.dark.frost_wide == 4)
check("clear-material: with no white over it", cleared.window.dark.tint_over_dark == 0 and cleared.pane.dark.tint_over_bright == 0)
check("clear-material: bright things pulled down, less behind a window", cleared.pane.dark.body_shade == 0.45 and cleared.window.dark.body_shade == 0.4)
write(home .. "/.config/vetro/sharp-material", "")
check("clear-material: comes before the sharp one", run({ plugin = true }).glass.presets.pane.dark.frost_cloud == 1)
remove(home .. "/.config/vetro")
write(home .. "/.config/vetro/sharp-material", "")
local sharpened = run({ plugin = true }).glass.presets
check("sharp-material: nothing behind a pane is out of focus", sharpened.pane.dark.frost_cloud == 0)
check("sharp-material: nor behind a window", sharpened.window.dark.frost_cloud == 0)
check("sharp-material: and there is no white over it either", sharpened.window.dark.tint_over_dark == 0)
write(home .. "/.config/vetro/cloud-material", "")
check("cloud-material: comes before the others", run({ plugin = true }).glass.presets.pane.dark.frost_cloud == nil)
remove(home .. "/.config/vetro")
write(home .. "/.config/vetro/first-material", "")
check("a file an earlier version left changes nothing", (run({ plugin = true }).glass.presets.pane or {}).inherits == "tuned")
remove(home .. "/.config/vetro")

-- Nothing is named for an earlier project, and no theme is looked at: the
-- same run gives the same result whichever theme is chosen.
-- (The names are spelled in two halves so that this file does not have them.)
local earlier = { "lift" .. "off", "menis" .. "cus" }
for pane in pairs(seen.glass.layers) do
  check("loaded: " .. pane .. " has no old name", not pane:lower():find(earlier[1]) and not pane:lower():find(earlier[2]))
end
local source = read(repo .. "/hypr/vetro.lua")
check("the file never asks which theme is chosen", not source:find("theme%.name"))
local code = source:gsub("%-%-[^\n]*", "")
check("the file never loads a plugin", not code:find("plugin load") and not code:find("plugin%.load") and not code:find("plugin_load"))
write(state_home .. "/omarchy/current/theme.name", "tokyo-night\n")
local a = dump(run({ plugin = true }))
write(state_home .. "/omarchy/current/theme.name", "launch\n")
local b = dump(run({ plugin = true }))
write(state_home .. "/omarchy/current/theme.name", "wallhaven-k8j6w1\n")
local c = dump(run({ plugin = true }))
check("the same under three themes", a == b and b == c)

-- The guard counts only Hyprland's own readings. This test is not Hyprland,
-- so it is run twice: once saying it stands in for it (VETRO_GUARD_TEST=1),
-- and once as the stranger it is, whose readings must count for nothing.
if os.getenv("VETRO_GUARD_TEST") == "1" then
  -- Read in a loop, it stands down: twelve readings in ten seconds are let
  -- through, the thirteenth and every one after it leave the glass alone.
  run({ plugin = true })
  for _ = 2, 12 do
    seen = run({ plugin = true, keep_tally = true })
  end
  check("twelve readings: still at work", #seen.glass.config == 1 and next(seen.glass.layers) ~= nil)
  seen = run({ plugin = true, keep_tally = true })
  check("thirteenth reading: stands down", #seen.glass.config == 0 and next(seen.glass.layers) == nil)
  check("stood down: the mark says why", read(runtime .. "/vetro-glass-stood-down"):find("read 13 times") ~= nil)
  check("stood down: the blur stands in", rule_for(seen, "^vetro:barglass$", "blur") ~= nil and rule_for(seen, "^vetro:barglass$", "blur").ignore_alpha == 0.41)
  check("stood down: and nothing is loaded", #seen.loads == 0)
  seen = run({ plugin = true, keep_tally = true })
  check("stood down: and stays down", #seen.glass.config == 0)
  seen = run({ plugin = true })
  check("cleared (vetro on): at work again", #seen.glass.config == 1)
else
  run({ plugin = true })
  for _ = 2, 15 do
    seen = run({ plugin = true, keep_tally = true })
  end
  check("read from outside Hyprland: fifteen readings change nothing", #seen.glass.config == 1 and next(seen.glass.layers) ~= nil)
  check("read from outside Hyprland: no tally is kept", read(runtime .. "/vetro-glass-readings") == "")
  check("read from outside Hyprland: and no mark is left", read(runtime .. "/vetro-glass-stood-down") == "")
end

-- Every option and rule field is one Hyprland has.
local known
local stubs = io.open("/usr/share/hypr/stubs/hl.meta.lua", "r")
if stubs then
  known = { options = {}, rule = {} }
  local in_rule = false
  for line in stubs:lines() do
    local key = line:match("^%-%-%-@field %['([%w_.]+)'%]")
    if key then
      known.options[key] = true
    end
    if line:match("^%-%-%-@class HL%.LayerRuleSpec") then
      in_rule = true
    elseif in_rule then
      local field = line:match("^%-%-%-@field ([%w_]+)%??")
      if field then
        known.rule[field] = true
      else
        in_rule = false
      end
    end
  end
  stubs:close()
end

-- A table of sides or a pair of numbers is one value, not a group of options.
local function is_value(t)
  if #t > 0 then
    return true
  end
  for key in pairs(t) do
    if key ~= "top" and key ~= "right" and key ~= "bottom" and key ~= "left" then
      return false
    end
  end
  return true
end

local function each_option(t, prefix, visit)
  for key, value in pairs(t) do
    local path = prefix == "" and key or (prefix .. "." .. key)
    if type(value) == "table" and not is_value(value) then
      each_option(value, path, visit)
    else
      visit(path)
    end
  end
end

seen = run()
check("on: options are set", #seen.config > 0)
if known then
  for _, group in ipairs(seen.config) do
    each_option(group, "", function(path)
      check("Hyprland has no option " .. path, known.options[path])
    end)
  end
  for _, rule in ipairs(seen.rules) do
    for field in pairs(rule) do
      check("Hyprland has no layer rule field " .. field, known.rule[field])
    end
  end
else
  print("note  Hyprland's API description is not installed: option names were not checked")
end

if failures > 0 then
  print(failures .. " of " .. checks .. " check(s) failed")
  os.exit(1)
end
print("lua.lua: " .. checks .. " checks passed")

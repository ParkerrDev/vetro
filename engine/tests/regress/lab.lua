-- The nested compositor's config: an output of the size of the owner's screen at
-- its scale, no animations, and the glass of glass-config.lua when the plugin is
-- loaded. The plugin is loaded from the shell, never from here.
local W = "@WORK@"

hl.config({
  general = { gaps_in = 5, gaps_out = 15, border_size = 0 },
  decoration = { rounding = 10, rounding_power = 4.0, blur = { enabled = false } },
  animations = { enabled = false },
  misc = { disable_hyprland_logo = true, disable_splash_rendering = true, disable_watchdog_warning = true },
})

hl.monitor({ output = "", mode = "2560x1600@60", position = "0x0", scale = 1.6 })

if hl.plugin.hyprglass then
  local ok, glass = pcall(dofile, W .. "/glass-config.lua")
  if ok and type(glass) == "table" then
    local hg = hl.plugin.hyprglass
    hg.config({
      enabled = false,
      default_theme = glass.theme,
      default_preset = glass.preset,
      layers = { enabled = glass.only ~= "", live_resample_fps = 60 },
      debug = { timers = glass.timers },
    })
    for ns in glass.only:gmatch("%S+") do
      hg.layer(ns, { preset = glass.preset, mask_threshold = glass.threshold, mask_feather = 0.04 })
    end
  end
end

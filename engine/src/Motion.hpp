#pragma once

#include <hyprland/src/desktop/DesktopTypes.hpp>
#include <hyprland/src/render/pass/PassElement.hpp>
#include <hyprland/src/SharedDefs.hpp>

#include <string>

// The engine's own window animations: a window that closes pours itself into
// the bottom of the screen, as a genie into its bottle, and a new one may pour
// out of it; a new window lands like jelly; a window dragged around bends
// behind the pointer and jiggles as it settles. Which of these run, and how
// quickly, is the tuning's (glass.conf, the Motion group of `vetro tune`), and
// none runs unless plugin:hyprglass:motion:enabled is on.
//
// The shapes are the model's (glass_genie_mesh, glass_wobble_*). This side
// keeps Hyprland's: the events, a window transformer that draws the window over
// the model's mesh, the closing window's snapshot, the framebuffers and the GL.
// The pace also scales Hyprland's own animations, as the config set them.
namespace Motion {

    void init();
    // Takes every transformer off its window and frees what the GL holds.
    void exit();

    void onOpen(PHLWINDOW window);
    void onClose(PHLWINDOW window);
    // Before a monitor's frame: every animation of it moved on to now.
    void onRenderPre(PHLMONITOR monitor);
    void onRenderStage(eRenderStage stage);
    void onMonitorGone(MONITORID monitor);

    // Before a config reload: Hyprland's animations back as the config set them.
    void beforeReload();
    // After it, and after the tuning changed: Hyprland's animations at the pace.
    void applyPace();

    // Whether a pass element is the fadeout of a window the genie closes, which
    // the genie draws in its place. Cheap when no window is closing.
    [[nodiscard]] bool swallows(IPassElement& element);

    // What to draw after a window's surface, when it is the main surface of
    // the window the genie would pour if it closed: an element that keeps a
    // copy of the window as it looks. Null for any other surface.
    [[nodiscard]] UP<IPassElement> keeper(IPassElement& surface);

    // `hyprctl hyprglass motion`: what runs, and how often each part has.
    [[nodiscard]] std::string describe();

    // Whether a window's glass may say it covers what is under it: never while
    // the window is bent, since its glass is lifted off and the frame put back.
    [[nodiscard]] bool bends(const PHLWINDOW& window);

    // Around the glass of a window being bent: what the glass draws is lifted
    // into a framebuffer of the window's, bent with it by the transformer, and
    // the frame is left as it was before the glass. Does nothing for a window
    // that is not bent.
    class CLift {
      public:
        CLift(const PHLWINDOW& window, const PHLMONITOR& monitor);
        ~CLift();

        CLift(const CLift&)            = delete;
        CLift& operator=(const CLift&) = delete;

      private:
        PHLWINDOWREF m_window;
        bool         m_active = false;
    };

} // namespace Motion

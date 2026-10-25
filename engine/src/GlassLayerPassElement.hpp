#pragma once

#include "Beneath.hpp"

#include <hyprland/src/render/pass/PassElement.hpp>
#include <hyprutils/math/Box.hpp>
#include <hyprutils/math/Region.hpp>
#include <memory>

class CGlassLayerSurface;

class CGlassLayerPassElement : public IPassElement {
  public:
    struct SGlassLayerPassData {
        std::shared_ptr<CGlassLayerSurface> layerState;
        float                               alpha = 1.0f;
        // Resolved once where the element is queued: needsLiveBlur() and draw() must agree on it.
        bool                                xray  = false;
        // The lens model draws this layer: the glass goes under the layer's own
        // pixels here, and no composite element follows.
        bool                                model = false;
        // The windows drawn before the layer that meet it (Beneath.hpp), taken
        // where the element is queued, when the frame's windows before it are known.
        std::vector<Beneath::SRect>         under;
    };

    explicit CGlassLayerPassElement(const SGlassLayerPassData& data);
    ~CGlassLayerPassElement() override = default;

    std::vector<UP<IPassElement>> draw() override;
    [[nodiscard]] bool                needsLiveBlur() override;
    [[nodiscard]] bool                needsPrecomputeBlur() override;
    [[nodiscard]] std::optional<CBox> boundingBox() override;
    [[nodiscard]] bool                disableSimplification() override;

    [[nodiscard]] const char* passName() override { return "CGlassLayerPassElement"; }
    [[nodiscard]] ePassElementType type() override { return EK_CUSTOM; }

  private:
    // Shared by boundingBox() and needsLiveBlur() so they can never disagree
    // about whether a box exists — see needsLiveBlur()'s comment.
    [[nodiscard]] std::optional<CBox> paddedLogicalBox() const;

    SGlassLayerPassData m_data;
};

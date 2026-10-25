use super::*;
use crate::options::resolve::{Request, resolve};
use crate::options::{FloatOption as F, IntOption as I, Levels, Store, StoreView};

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

fn preset(name: &str, dark: bool) -> crate::options::Values {
    let bytes = Store::builtin().to_bytes().unwrap();
    resolve(&Request {
        store: StoreView::new(&bytes).unwrap(),
        preset: name,
        dark,
        levels: &Levels::default(),
    })
}

fn colour(value: i64) -> [u8; 3] {
    [24_u32, 16, 8].map(|shift| u8::try_from((value >> shift) & 0xff).unwrap())
}

#[test]
fn g26_is_what_the_regular_preset_resolves_to() {
    for dark in [true, false] {
        let regular = preset("regular", dark);
        let appearance = G26.appearance(dark);
        assert_eq!(colour(regular.int(I::TintColor)), appearance.tone.tint);
        assert!((regular.float(F::TintOverDark) - appearance.tone.tint_over_dark).abs() < 1e-6);
        assert!((regular.float(F::TintOverBright) - appearance.tone.tint_over_bright).abs() < 1e-6);
        assert!((regular.float(F::Saturation) - appearance.tone.saturation).abs() < 1e-6);
        assert!(near(
            f64::from(regular.float(F::Frost)),
            appearance.frost.narrow_sigma
        ));
        let shadow = appearance.shadow.unwrap();
        assert!(near(
            f64::from(regular.float(F::ShadowOpacity)),
            shadow.opacity
        ));
        assert!(
            near(f64::from(regular.float(F::ShadowRadius)), shadow.blur)
                && near(f64::from(regular.float(F::ShadowOffset)), shadow.offset)
        );
        assert!(near(
            f64::from(regular.float(F::LightAngle)),
            appearance.rim.light_angle
        ));
        assert!(near(
            f64::from(regular.float(F::LightIntensity)),
            appearance.rim.strength
        ));
        assert!(near(
            f64::from(regular.float(F::RimGlow)),
            appearance.rim.glow_and_inner_shadow.unwrap()[0]
        ));
        assert!(near(
            f64::from(regular.float(F::InnerShadow)),
            appearance.rim.glow_and_inner_shadow.unwrap()[1]
        ));
        assert!(near(
            f64::from(regular.float(F::Dispersion)),
            G26.dispersion
        ));
    }
}

#[test]
fn g26_has_the_lens_of_the_regular_preset() {
    let regular = preset("regular", true);
    let LensLaw::Snell {
        bezel_width,
        thickness,
        refractive_index,
        profile_exponent,
    } = G26.lens
    else {
        panic!("G26 is Snell")
    };
    assert!(
        near(f64::from(regular.float(F::BezelWidth)), bezel_width)
            && near(f64::from(regular.float(F::GlassThickness)), thickness)
    );
    assert!(
        near(
            f64::from(regular.float(F::RefractiveIndex)),
            refractive_index
        ) && near(
            f64::from(regular.float(F::ProfileExponent)),
            profile_exponent
        )
    );
    assert!(near(G26.lens.bezel_width(), 20.0) && G26.lens.paraxial_displacement(0.3).is_none());
}

#[test]
fn g26_clear_is_what_the_clear_preset_resolves_to() {
    let clear = G26.clear.unwrap();
    for dark in [true, false] {
        let preset = preset("clear", dark);
        let tint = if dark {
            clear.dark_tint
        } else {
            clear.light_tint
        };
        assert!(
            (preset.float(F::TintOverDark) - tint[0]).abs() < 1e-6
                && (preset.float(F::TintOverBright) - tint[1]).abs() < 1e-6
        );
        assert!((preset.float(F::Saturation) - clear.saturation).abs() < 1e-6);
        assert!(
            near(f64::from(preset.float(F::Frost)), clear.frost_sigma)
                && near(f64::from(preset.float(F::Dispersion)), clear.dispersion)
        );
        assert!(near(
            f64::from(preset.float(F::LightIntensity)),
            clear.light_intensity
        ));
    }
    assert!(
        near(f64::from(G26.clear.unwrap().dark_tint[1]), 0.35),
        "Apple's 35 percent dimming over bright media"
    );
}

#[test]
fn g27_frost_is_a_cloud_and_a_ghost() {
    let (light, dark) = (G27.light.frost, G27.dark.frost);
    assert!(near(light.narrow_sigma, 0.6) && near(dark.narrow_sigma, 0.6));
    let (lw, dw) = (light.wide.unwrap(), dark.wide.unwrap());
    assert!(near(lw.sigma, 14.0) && near(dw.sigma, 14.0));
    assert!(lw.held_is_max && !dw.held_is_max);
    assert!(near(lw.held_offset, 0.4) && near(dw.held_offset, 0.45));
    assert!(near(lw.cloud_opacity, 0.73) && near(dw.cloud_opacity, 0.85));
    assert!(near(lw.cloud_white_weight, 2.0) && near(dw.cloud_white_weight, 0.5));
    assert!(near(lw.ghost_weight, 0.8) && near(dw.ghost_weight, 2.5));
    assert!(G26.light.frost.wide.is_none(), "G26 has one blur");
}

#[test]
fn g27_tone_is_light_f8_at_half_and_dark_white_at_an_eighth() {
    let (light, dark) = (G27.light.tone, G27.dark.tone);
    assert_eq!(light.tint, [0xf8; 3]);
    assert!(
        (light.tint_over_dark - 0.53).abs() < 1e-6
            && (light.saturation - 2.1).abs() < 1e-6
            && light.body_shade.is_none()
    );
    assert_eq!(dark.tint, [0xff; 3]);
    assert!((dark.tint_over_bright - 0.12).abs() < 1e-6 && (dark.saturation - 1.4).abs() < 1e-6);
    assert!((dark.body_shade.unwrap() - 0.28).abs() < 1e-6);
    // a white page under dark glass comes out near 184/255 by the body shade's own rule
    let shaded = 1.0 - dark.body_shade.unwrap();
    assert!((shaded * 255.0 - 184.0).abs() < 1.0, "{}", shaded * 255.0);
}

#[test]
fn g27_rim_has_two_lobes_on_the_vertical_axis_and_a_hairline() {
    let (light, dark) = (G27.light.rim, G27.dark.rim);
    assert!(
        near(light.light_angle, 0.0) && near(dark.light_angle, 180.0),
        "lit from the top, lit from the bottom"
    );
    assert!(near(light.strength, 1.0) && near(dark.strength, 1.15));
    assert!(near(light.opposite, 0.85) && near(light.luma_power, 0.3) && near(light.inset, 0.75));
    assert!(
        near(light.core[0], 0.175)
            && near(light.core[1], 1.5)
            && near(light.tail[0], 0.035)
            && near(light.tail[1], 8.0)
    );
    assert!(
        light.glow_and_inner_shadow.is_none(),
        "G27 drops the engine's glow and inner shadow"
    );
    let (lh, dh) = (light.hairline.unwrap(), dark.hairline.unwrap());
    assert!(
        near(lh.shade, 0.314)
            && near(lh.ends, 0.2)
            && near(dh.ends, 0.0)
            && near(lh.across_power, 1.75)
    );
    assert!(near(lh.width[0], 0.75) && near(lh.width[1], 2.0) && near(lh.read_offset, 2.5));
    assert!((lh.shade * 255.0 - 80.0).abs() < 0.1, "0.314 is 80 of 255");
    assert!(G26.light.rim.hairline.is_none());
}

#[test]
fn g27_shadow_is_faint_and_only_under_light_glass() {
    let shadow = G27.light.shadow.unwrap();
    assert!(near(shadow.opacity, 0.016) && near(shadow.blur, 10.0) && near(shadow.offset, 7.0));
    assert!(G27.dark.shadow.is_none());
    assert!(
        near(G26.dark.shadow.unwrap().opacity, 0.12),
        "the engine's own shadow"
    );
}

#[test]
fn g27_lens_is_the_paraxial_law_measured_on_a_56_point_control() {
    let LensLaw::Paraxial {
        thickness,
        refractive_index,
        fold,
    } = G27.lens
    else {
        panic!("G27 is paraxial")
    };
    assert!(near(thickness, 10.7) && near(refractive_index, 1.24) && near(fold, 0.74));
    assert!(
        near(G27.lens.bezel_width(), 10.7),
        "the bezel is as wide as the thickness"
    );
    let amplitude = G27.lens.paraxial_displacement(0.0).unwrap();
    assert!(
        (amplitude - 20.5).abs() < 0.1,
        "A = 8 thickness (n - 1) = {amplitude}"
    );
    assert!(near(G27.lens.paraxial_displacement(1.0).unwrap(), 0.0));
    assert!(
        near(
            G27.lens.paraxial_displacement(0.5).unwrap(),
            amplitude / 4.0
        ),
        "falls as the square of the way in"
    );
    assert!(near(G27.dispersion, 0.0) && G27.clear.is_none());
}

#[test]
fn a_material_is_found_by_name() {
    assert_eq!(by_name("G26").map(|m| m.generation), Some(Generation::G26));
    assert_eq!(by_name("G27").map(|m| m.generation), Some(Generation::G27));
    assert!(by_name("G28").is_none());
    assert!(
        std::ptr::eq(G26.appearance(true), &G26.dark)
            && std::ptr::eq(G26.appearance(false), &G26.light)
    );
}

#[test]
fn the_regular27_preset_is_g27_in_both_appearances() {
    for dark in [false, true] {
        let preset = preset("regular27", dark);
        let appearance = G27.appearance(dark);
        assert_eq!(
            by_option(preset.int(I::Material)).map(|(_, chosen)| *chosen),
            Some(*appearance),
            "the preset names the material and the appearance it is"
        );
        assert_eq!(colour(preset.int(I::TintColor)), appearance.tone.tint);
        assert!((preset.float(F::TintOverDark) - appearance.tone.tint_over_dark).abs() < 1e-6);
        assert!((preset.float(F::TintOverBright) - appearance.tone.tint_over_bright).abs() < 1e-6);
        assert!((preset.float(F::Saturation) - appearance.tone.saturation).abs() < 1e-6);
        assert!(
            (preset.float(F::BodyShade) - appearance.tone.body_shade.unwrap_or(0.0)).abs() < 1e-6
        );
        let wide = appearance.frost.wide.unwrap();
        assert!(near(
            f64::from(preset.float(F::Frost)),
            appearance.frost.narrow_sigma
        ));
        assert!(near(f64::from(preset.float(F::FrostWide)), wide.sigma));
        assert!(near(
            f64::from(preset.float(F::FrostCloud)),
            wide.cloud_opacity
        ));
        assert!(near(
            f64::from(preset.float(F::LightAngle)),
            appearance.rim.light_angle
        ));
        assert!(near(
            f64::from(preset.float(F::RimLight)),
            appearance.rim.strength
        ));
        assert!(near(f64::from(preset.float(F::RimShade)), 1.0));
        assert!(
            near(f64::from(preset.float(F::LightIntensity)), 0.0)
                && near(f64::from(preset.float(F::RimGlow)), 0.0)
                && near(f64::from(preset.float(F::InnerShadow)), 0.0),
            "the first generation's light is off"
        );
        assert!(near(
            f64::from(preset.float(F::ShadowOpacity)),
            appearance.shadow.map_or(0.0, |shadow| shadow.opacity)
        ));
    }
}

#[test]
fn the_material_option_names_the_current_material_or_nothing() {
    assert!(by_option(0).is_none() && by_option(-1).is_none() && by_option(3).is_none());
    assert_eq!(
        by_option(OPTION_CURRENT_LIGHT).map(|(m, _)| m.name),
        Some("G27")
    );
    assert_eq!(
        by_option(OPTION_CURRENT_DARK).map(|(_, a)| a.tone.body_shade),
        Some(Some(0.28))
    );
}

#[test]
fn the_regular27_preset_has_g27s_lens() {
    for dark in [false, true] {
        let preset = preset("regular27", dark);
        let LensLaw::Paraxial {
            thickness,
            refractive_index,
            ..
        } = G27.lens
        else {
            panic!("G27 bends by the thin-prism law");
        };
        assert!(
            near(f64::from(preset.float(F::GlassThickness)), thickness)
                && near(f64::from(preset.float(F::BezelWidth)), thickness)
                && near(
                    f64::from(preset.float(F::RefractiveIndex)),
                    refractive_index
                )
        );
    }
}

use super::builtin::{DARK_THEME_DEFAULTS, LIGHT_THEME_DEFAULTS, model_defaults};
use super::ids::{FloatOption as F, IntOption as I};
use super::parse::{parse_float, parse_int, set_field, strip_dynamic_tag_marker};
use super::*;

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-5
}

/// The built-in presets as the plugin holds them: bytes.
fn builtin_bytes() -> Vec<u8> {
    Store::builtin().to_bytes().unwrap()
}

fn resolved(store: &Store, preset: &str, dark: bool, levels: &Levels) -> Values {
    let bytes = store.to_bytes().unwrap();
    resolve(&Request {
        store: StoreView::new(&bytes).unwrap(),
        preset,
        dark,
        levels,
    })
}

fn user_store(lines: &[&str]) -> Store {
    let mut store = Store::builtin();
    for line in lines {
        store.add_line(line).unwrap();
    }
    store
}

#[test]
fn every_option_is_read_by_its_own_name_into_its_own_slot() {
    let mut values = Values::UNSET;
    let mut index = 0_u32;
    for option in F::ALL {
        index += 1;
        assert!(
            set_field(&mut values, option.key(), &index.to_string()),
            "{}",
            option.key()
        );
    }
    for option in I::ALL {
        index += 1;
        assert!(
            set_field(&mut values, option.key(), &index.to_string()),
            "{}",
            option.key()
        );
    }
    let mut check = 0_u16;
    for option in F::ALL {
        check += 1;
        assert!(
            near(values.float(option), f32::from(check)),
            "no option overwrites another: {}",
            option.key()
        );
    }
    for option in I::ALL {
        check += 1;
        assert_eq!(values.int(option), i64::from(check), "{}", option.key());
    }
}

#[test]
fn the_tables_agree_with_the_enums() {
    for (position, option) in F::ALL.iter().enumerate() {
        assert_eq!(option.index(), position);
        assert_eq!(F::from_key(option.key()), Some(*option));
    }
    for (position, option) in I::ALL.iter().enumerate() {
        assert_eq!(option.index(), position);
        assert_eq!(I::from_key(option.key()), Some(*option));
    }
    let mut keys: Vec<&str> = F::ALL
        .iter()
        .map(|o| o.key())
        .chain(I::ALL.iter().map(|o| o.key()))
        .collect();
    keys.sort_unstable();
    keys.dedup();
    assert_eq!(keys.len(), FLOAT_COUNT + INT_COUNT, "every key is its own");
}

#[test]
fn the_options_that_were_there_before_keep_their_names() {
    let mut values = Values::UNSET;
    assert!(
        set_field(&mut values, "blur_strength", "1.5") && near(values.float(F::BlurStrength), 1.5)
    );
    assert!(
        set_field(&mut values, "tint_color", "0x8899aa22")
            && values.int(I::TintColor) == 0x8899_aa22
    );
    assert!(set_field(&mut values, "blur_iterations", "3") && values.int(I::BlurIterations) == 3);
    assert!(set_field(&mut values, "xray", "1") && values.int(I::Xray) == 1);
    assert!(set_field(&mut values, "saturation", "2.5") && near(values.float(F::Saturation), 2.5));
    assert!(
        !set_field(&mut values, "no_such_option", "1"),
        "an unknown option is refused"
    );
    assert!(
        !set_field(&mut values, "frost", "soft"),
        "a value that is not a number is refused"
    );
    // Lua hands numbers over as text with a fraction, booleans as 0 or 1
    assert!(set_field(&mut values, "model", "1.000000") && values.int(I::Model) == 1);
    assert!(set_field(&mut values, "frost", "6.000000") && near(values.float(F::Frost), 6.0));
}

#[test]
fn numbers_are_read_as_from_chars_reads_them() {
    assert_eq!(
        parse_float("1.5abc"),
        Some(1.5),
        "the longest prefix counts"
    );
    assert_eq!(parse_float("+5"), None, "there is no leading plus");
    assert_eq!(parse_float(".5"), Some(0.5));
    assert_eq!(parse_float("5."), Some(5.0));
    assert_eq!(parse_float("1e3"), Some(1000.0));
    assert_eq!(
        parse_float("1e"),
        Some(1.0),
        "an exponent without digits is not one"
    );
    assert_eq!(parse_float("-3"), Some(-3.0));
    assert_eq!(parse_float("abc"), None);
    assert_eq!(parse_float(""), None);
    assert_eq!(parse_float("1e39"), None, "out of range is a failure");
    assert_eq!(parse_float("1e-99"), None, "and so is underflow to nothing");
    assert_eq!(parse_float("0.0000"), Some(0.0));
    assert!(parse_float("inf").is_some_and(f32::is_infinite));
    assert!(parse_float("NaN").is_some_and(f32::is_nan));
    assert_eq!(parse_int("12abc"), Some(12));
    assert_eq!(parse_int("0x10"), Some(16));
    assert_eq!(parse_int("0X1f"), Some(31));
    assert_eq!(parse_int("-5"), Some(-5));
    assert_eq!(
        parse_int("0x"),
        Some(0),
        "0x alone is a zero followed by a letter"
    );
    assert_eq!(parse_int("99999999999999999999"), None);
    assert_eq!(parse_int("x"), None);
}

#[test]
fn the_one_line_form_names_a_preset_its_layer_and_its_parent() {
    let line = parse_line(
        "name:pane:dark, inherits:regular, frost:4, bezel_width:12, tint_color:0x10203040",
    )
    .unwrap();
    assert!(line.name == "pane" && line.variant == Variant::Dark && line.inherits == "regular");
    assert!(near(line.values.float(F::Frost), 4.0) && near(line.values.float(F::BezelWidth), 12.0));
    assert_eq!(line.values.int(I::TintColor), 0x1020_3040);
    assert!(
        line.values.float(F::GlassThickness) < 0.0,
        "what the line does not name stays unset"
    );
}

#[test]
fn a_line_that_does_not_read_says_why() {
    assert_eq!(parse_line("frost:4"), Err(LineError::MissingName));
    assert_eq!(
        parse_line("name:x:blue"),
        Err(LineError::InvalidVariant("blue".into()))
    );
    assert_eq!(
        parse_line("name:x, wobble:3"),
        Err(LineError::InvalidSetting("wobble".into(), "3".into()))
    );
    assert_eq!(
        parse_line("name:x, frost"),
        Err(LineError::InvalidToken("frost".into()))
    );
    assert_eq!(
        parse_line("name:x:").unwrap_err().to_string(),
        "preset: invalid variant '' (expected dark or light)"
    );
    assert_eq!(
        parse_line("name:x, wobble:3").unwrap_err().to_string(),
        "preset: unknown or invalid setting 'wobble:3'"
    );
}

#[test]
fn the_dynamic_tag_marker_is_stripped_from_names() {
    assert_eq!(parse_line("name:firefox*").unwrap().name, "firefox");
    assert_eq!(parse_line("name:a, inherits:b*").unwrap().inherits, "b");
    assert_eq!(strip_dynamic_tag_marker("x**"), "x*", "one marker only");
}

#[test]
fn a_merge_overrides_only_what_is_set() {
    let mut base = Values::UNSET;
    base.set_float(F::Frost, 6.0);
    base.set_float(F::BezelWidth, 20.0);
    let mut over = Values::UNSET;
    over.set_float(F::Frost, 2.0);
    base.merge(&over);
    assert!(near(base.float(F::Frost), 2.0) && near(base.float(F::BezelWidth), 20.0));
}

#[test]
fn the_built_in_presets_are_there_and_the_classic_ones_stay_classic() {
    let store = Store::builtin();
    for name in [
        "regular",
        "clear",
        "classic_clear",
        "high_contrast",
        "subtle",
        "glass",
        "pomme",
    ] {
        assert!(store.get(name).is_some(), "{name}");
    }
    let levels = Levels::default();
    for name in ["high_contrast", "subtle", "glass", "pomme"] {
        assert!(
            !uses_model(&resolved(&store, name, true, &levels)),
            "{name} stays on the classic shader"
        );
    }
    assert!(
        uses_model(&resolved(&store, "regular", true, &levels))
            && uses_model(&resolved(&store, "clear", false, &levels))
    );
}

#[test]
fn regular_carries_the_measured_numbers() {
    let store = Store::builtin();
    let dark = resolved(&store, "regular", true, &Levels::default());
    let light = resolved(&store, "regular", false, &Levels::default());
    assert!(
        near(dark.float(F::BezelWidth), 20.0)
            && near(dark.float(F::GlassThickness), 53.0)
            && near(dark.float(F::ProfileExponent), 2.6)
    );
    assert!(
        near(dark.float(F::Frost), 6.0)
            && near(dark.float(F::TintOverDark), 0.76)
            && near(light.float(F::TintOverDark), 0.70)
    );
    assert_eq!(
        (dark.int(I::TintColor), light.int(I::TintColor)),
        (0x2525_25ff, 0xfafa_faff)
    );
    assert!(
        near(dark.float(F::ShadowOpacity), 0.12)
            && near(dark.float(F::ShadowRadius), 40.0)
            && near(dark.float(F::ShadowOffset), 8.0)
    );
    assert!(near(dark.float(F::Saturation), 2.5) && near(light.float(F::Saturation), 1.95));
}

#[test]
fn clear_is_regular_barely_frosted_and_dims_bright_content() {
    let clear = resolved(&Store::builtin(), "clear", true, &Levels::default());
    assert!(
        near(clear.float(F::Frost), 2.0)
            && near(clear.float(F::TintOverBright), 0.35)
            && near(clear.float(F::TintOverDark), 0.18)
    );
    assert!(
        near(clear.float(F::BezelWidth), 20.0),
        "and keeps the lens it copied"
    );
    assert!(near(clear.float(F::Dispersion), 0.1) && near(clear.float(F::LightIntensity), 0.4));
}

#[test]
fn a_preset_that_inherits_keeps_what_it_does_not_override() {
    let store = user_store(&["name:mine, inherits:regular, frost:9"]);
    let mine = resolved(&store, "mine", true, &Levels::default());
    assert!(
        near(mine.float(F::Frost), 9.0)
            && near(mine.float(F::BezelWidth), 20.0)
            && uses_model(&mine)
    );
}

#[test]
fn an_unknown_preset_resolves_to_what_the_levels_and_defaults_say() {
    let values = resolved(
        &Store::builtin(),
        "no_such_preset",
        true,
        &Levels::default(),
    );
    assert!(near(
        values.float_or(F::Frost, model_defaults::FROST),
        model_defaults::FROST
    ));
    assert!(values.float(F::Frost) < 0.0, "nothing set it");
}

#[test]
fn the_chain_goes_variant_then_shared_then_parent() {
    let store = user_store(&[
        "name:parent, frost:1, bezel_width:11, glass_thickness:12",
        "name:child, inherits:parent, frost:2, bezel_width:21",
        "name:child:dark, frost:3",
    ]);
    let levels = Levels::default();
    let dark = resolved(&store, "child", true, &levels);
    let light = resolved(&store, "child", false, &levels);
    assert!(
        near(dark.float(F::Frost), 3.0),
        "the variant beats the shared layer"
    );
    assert!(
        near(light.float(F::Frost), 2.0),
        "the other appearance does not see it"
    );
    assert!(
        near(dark.float(F::BezelWidth), 21.0),
        "shared beats the parent"
    );
    assert!(
        near(dark.float(F::GlassThickness), 12.0),
        "and the parent supplies the rest"
    );
}

#[test]
fn a_chain_of_parents_is_followed_eight_deep_and_no_further() {
    let mut store = Store::new();
    for depth in 0..10 {
        store.set_inherits(&format!("p{depth}"), &format!("p{}", depth + 1));
    }
    store.set_field("p7", Variant::Shared, "frost", "7");
    store.set_field("p8", Variant::Shared, "bezel_width", "8");
    let values = resolved(&store, "p0", true, &Levels::default());
    assert!(
        near(values.float(F::Frost), 7.0),
        "the eighth preset counts"
    );
    assert!(values.float(F::BezelWidth) < 0.0, "the ninth does not");
}

#[test]
fn a_loop_of_parents_ends() {
    let mut store = Store::new();
    store.set_inherits("a", "b");
    store.set_inherits("b", "a");
    store.set_field("b", Variant::Dark, "frost", "5");
    assert!(near(
        resolved(&store, "a", true, &Levels::default()).float(F::Frost),
        5.0
    ));
}

#[test]
fn presets_beat_the_theme_level_which_beats_the_global_one() {
    let store = user_store(&["name:p, frost:4"]);
    let mut levels = Levels::default();
    levels.global.set_float(F::Frost, 1.0);
    levels.global.set_float(F::BezelWidth, 10.0);
    levels.global.set_float(F::RimWidth, 5.0);
    levels.dark.set_float(F::BezelWidth, 30.0);
    levels.light.set_float(F::BezelWidth, 40.0);
    let dark = resolved(&store, "p", true, &levels);
    let light = resolved(&store, "p", false, &levels);
    assert!(
        near(dark.float(F::Frost), 4.0),
        "the preset beats the global level"
    );
    assert!(
        near(dark.float(F::BezelWidth), 30.0) && near(light.float(F::BezelWidth), 40.0),
        "the theme beats the global level"
    );
    assert!(
        near(dark.float(F::RimWidth), 5.0),
        "and the global level is the last that is asked"
    );
    assert!(
        dark.float(F::Dispersion) < 0.0,
        "the caller's default is not part of the chain"
    );
}

#[test]
fn a_negative_value_is_not_set() {
    let mut levels = Levels::default();
    levels.dark.set_float(F::Frost, -1.0);
    levels.global.set_float(F::Frost, 3.0);
    assert!(near(
        resolved(&Store::new(), "x", true, &levels).float(F::Frost),
        3.0
    ));
    levels.dark.set_int(I::Model, 0);
    assert_eq!(
        resolved(&Store::new(), "x", true, &levels).int(I::Model),
        0,
        "zero is a value"
    );
}

#[test]
fn the_store_round_trips_through_its_bytes() {
    let store = user_store(&[
        "name:mine, inherits:clear, frost:9, tint_color:0x01020304",
        "name:mine:light, saturation:1.5",
    ]);
    let bytes = store.to_bytes().unwrap();
    assert_eq!(Store::from_bytes(&bytes).unwrap(), store);
    let view = StoreView::new(&bytes).unwrap();
    assert!(view.contains("mine") && view.contains("regular") && !view.contains("nothing"));
    assert_eq!(view.find("mine").unwrap().inherits, "clear");
    assert_eq!(
        StoreView::new(&bytes[..3]).unwrap_err(),
        StoreError::Truncated
    );
    assert_eq!(
        StoreView::new(b"nope....").unwrap_err(),
        StoreError::BadMagic
    );
    assert!(
        Store::from_bytes(&bytes[..bytes.len() - 1]).is_err(),
        "a cut-short block is refused"
    );
}

#[test]
fn a_long_name_is_refused_not_cut() {
    let mut store = Store::new();
    store.define(&"x".repeat(70_000));
    assert_eq!(store.to_bytes(), Err(StoreError::NameTooLong(70_000)));
}

#[test]
fn a_user_preset_over_a_built_in_one_changes_only_what_it_sets() {
    let mut user = Store::new();
    user.add_line("name:regular, frost:9").unwrap();
    user.add_line("name:extra, frost:1").unwrap();
    let committed = Store::committed(&user);
    let regular = resolved(&committed, "regular", true, &Levels::default());
    assert!(
        near(regular.float(F::Frost), 9.0) && near(regular.float(F::BezelWidth), 20.0),
        "regular keeps the rest"
    );
    assert!(committed.get("extra").is_some() && committed.get("clear").is_some());
    assert_eq!(committed.len(), Store::builtin().len() + 1);
}

#[test]
fn a_preset_is_found_with_or_without_its_marker() {
    let mut store = Store::new();
    store.set_field("firefox*", Variant::Dark, "frost", "3");
    assert!(store.get("firefox").is_some() && store.get("firefox*").is_none());
}

#[test]
fn self_sampling_is_found_in_any_layer() {
    let mut store = Store::builtin();
    assert!(!store.any_float_positive(F::SelfSample));
    store.set_field("p", Variant::Light, "self_sample", "0.5");
    assert!(store.any_float_positive(F::SelfSample));
    let bytes = store.to_bytes().unwrap();
    assert!(
        StoreView::new(&bytes)
            .unwrap()
            .any_float_positive(F::SelfSample)
    );
}

#[test]
fn the_registered_defaults_are_what_the_classic_shader_always_had() {
    assert!(
        (F::BlurStrength.global_default() - 2.0).abs() < 1e-9
            && (F::BevelAngle.global_default() - 315.0).abs() < 1e-9
    );
    assert_eq!(I::TintColor.global_default(), 0x8899_aa22);
    assert!(
        F::Brightness.global_default() < 0.0
            && F::Frost.global_default() < 0.0
            && I::Model.global_default() < 0
    );
    assert!(
        near(DARK_THEME_DEFAULTS.brightness, 0.82)
            && near(LIGHT_THEME_DEFAULTS.adaptive_boost, 0.4)
    );
}

#[test]
fn lengths_are_scaled_to_device_pixels_and_ranges_are_held() {
    let mut options = Values::UNSET;
    options.set_float(F::BezelWidth, 20.0);
    options.set_float(F::RefractiveIndex, 9.0);
    options.set_float(F::Dispersion, 4.0);
    options.set_float(F::ShadowOpacity, 3.0);
    options.set_float(F::RimWidth, 0.2);
    options.set_int(I::TintColor, 0x2040_80ff);
    let params = model_params(&options, 1.6);
    assert!((params.lens.bezel_width - f64::from(20.0_f32 * 1.6)).abs() < 1e-9);
    assert!(
        (params.lens.refractive_index - 3.0).abs() < 1e-12
            && (params.lens.dispersion - 1.0).abs() < 1e-12
    );
    assert!(
        near(params.shadow_opacity, 1.0) && near(params.rim_width, 1.0),
        "held to the range, and a rim is a pixel at least"
    );
    assert!(near(params.tint_color[0], 32.0 / 255.0) && near(params.tint_color[2], 128.0 / 255.0));
    assert!(
        near(params.frost, model_defaults::FROST * 1.6),
        "an unset option takes the model's default"
    );
    assert!(
        near(params.saturation, 1.0),
        "and the classic ones leave the colour alone"
    );
    assert!(
        (model_params(&options, 0.0).lens.bezel_width - 20.0).abs() < 1e-12,
        "a scale of nothing is 1"
    );
}

#[test]
fn a_change_asks_for_exactly_the_rebuilds_that_depend_on_it() {
    let base = model_params(&Values::UNSET, 1.0);
    assert_eq!(changes(&base, &base), Changes::default());
    let mut other = base;
    other.lens.bezel_width += 1.0;
    assert_eq!(
        changes(&base, &other),
        Changes {
            field: true,
            lens: false,
            frost: false
        }
    );
    other = base;
    other.lens.dispersion = 0.5;
    assert_eq!(
        changes(&base, &other),
        Changes {
            field: false,
            lens: true,
            frost: false
        }
    );
    other = base;
    other.tint_color = [0.5, 0.5, 0.5];
    assert_eq!(
        changes(&base, &other),
        Changes {
            field: false,
            lens: false,
            frost: true
        }
    );
    other = base;
    other.shadow_opacity = 0.0;
    assert!(
        changes(&base, &other).field,
        "the shadow switching off or on is the field's"
    );
    other = base;
    other.lens.refraction = 2.0;
    assert_eq!(
        changes(&base, &other),
        Changes::default(),
        "refraction is a uniform: no rebuild"
    );
}

#[test]
fn the_builtin_bytes_are_stable() {
    assert_eq!(builtin_bytes(), builtin_bytes());
}

#[test]
fn the_current_material_reaches_the_passes_through_its_option_and_its_settings() {
    use crate::material::{OPTION_CURRENT_DARK, OPTION_CURRENT_LIGHT};

    // with nothing chosen every term of the current material is off
    let plain = model_params(&Values::UNSET, 1.6);
    assert!(!plain.lens.paraxial);
    assert!(
        near(plain.layered.frost_wide, 0.0)
            && near(plain.layered.rim_light, 0.0)
            && near(plain.layered.rim_shade, 0.0)
            && near(plain.layered.body_shade, 0.0)
            && near(plain.layered.adaptive_saturation, 0.0)
            && near(plain.layered.edge_absorption, 0.0)
    );
    assert!(near(plain.layered.cloud_weight, 1.0) && near(plain.layered.ghost_weight, 1.0));
    assert!(
        near(plain.layered.third, 0.75),
        "a third of a point in device pixels, and on a coarse screen never under three quarters of one"
    );
    assert!(near(model_params(&Values::UNSET, 3.0).layered.third, 1.0));

    // the dark appearance, named by the option alone: the material's own numbers
    let mut options = Values::UNSET;
    options.set_int(I::Material, OPTION_CURRENT_DARK);
    let dark = model_params(&options, 1.6);
    assert!(dark.lens.paraxial, "the thin prism");
    assert!(
        near(dark.layered.frost_wide, 14.0 * 1.6),
        "a length, so in device pixels"
    );
    assert!(near(dark.layered.frost_cloud, 0.85) && near(dark.layered.frost_hold, -0.45));
    assert!(near(dark.layered.cloud_weight, 0.5) && near(dark.layered.ghost_weight, 2.5));
    assert!(near(dark.layered.body_shade, 0.28) && near(dark.layered.adaptive_saturation, 0.2));
    assert!(near(dark.layered.rim_light, 1.15) && near(dark.layered.rim_shade, 1.0));
    assert!(near(dark.layered.rim_shade_ends, 0.0) && near(dark.layered.edge_absorption, 0.035));

    // the light one holds its ghost from the other side and keeps a fifth of the hairline at the ends
    options.set_int(I::Material, OPTION_CURRENT_LIGHT);
    let light = model_params(&options, 1.0);
    assert!(near(light.layered.frost_hold, 0.4) && near(light.layered.frost_cloud, 0.73));
    assert!(near(light.layered.rim_shade_ends, 0.2) && near(light.layered.body_shade, 0.0));

    // a setting, where one is given, is what counts
    options.set_float(F::FrostCloud, 0.5);
    options.set_float(F::BodyShade, 0.6);
    options.set_float(F::RimLight, 0.0);
    let tuned = model_params(&options, 1.0);
    assert!(near(tuned.layered.frost_cloud, 0.5) && near(tuned.layered.body_shade, 0.6));
    assert!(
        near(tuned.layered.rim_light, 0.0),
        "0 is a setting: the lobes are off"
    );

    // and each of them asks for the frost again, the law for the lens table
    assert!(changes(&light, &tuned).frost);
    assert!(changes(&plain, &dark).lens && changes(&plain, &dark).frost);
}

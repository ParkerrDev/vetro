use super::*;

#[test]
fn every_knob_is_in_its_place_and_its_default_in_range() {
    assert_eq!(Id::ALL.len(), COUNT);
    for (at, id) in Id::ALL.iter().enumerate() {
        assert_eq!(id.index(), at);
        let knob = id.knob();
        assert!(knob.min <= knob.max, "{}: range", knob.label);
        assert!(
            (knob.min..=knob.max).contains(&knob.default),
            "{}: default out of range",
            knob.label
        );
        assert!((Tuning::IOS27.get(*id) - knob.default).abs() < f32::EPSILON);
    }
}

#[test]
fn keys_are_unique_and_only_colour_parts_have_none() {
    for (at, knob) in KNOBS.iter().enumerate() {
        if knob.shown == Shown::Part {
            assert_eq!(knob.key, "");
            continue;
        }
        assert_ne!(knob.key, "");
        assert_eq!(
            Id::from_key(knob.key).map(Id::index),
            Some(at),
            "{} is not unique",
            knob.key
        );
    }
    assert_eq!(Id::from_key(""), None);
}

#[test]
fn the_colour_is_followed_by_its_two_parts() {
    let at = Id::TintRed.index();
    assert_eq!(KNOBS[at].shown, Shown::Colour);
    assert_eq!(KNOBS[at + 1].shown, Shown::Part);
    assert_eq!(KNOBS[at + 2].shown, Shown::Part);
}

#[test]
fn a_written_tuning_reads_back_the_same() {
    let mut tuning = Tuning::DEFAULT;
    tuning.set(Id::Frost, 12.5);
    tuning.set(Id::Tint, 0.42);
    tuning.set(Id::SeeWindows, 0.0);
    tuning.set(Id::TintRed, 1.0);
    let text = format(&tuning);
    let (read, skipped) = parse(&text);
    assert_eq!(skipped, 0, "{text}");
    assert!((read.get(Id::Frost) - 12.5).abs() < 1e-4);
    assert!((read.get(Id::Tint) - 0.42).abs() < 1e-4);
    assert!(!read.is_on(Id::SeeWindows));
    assert!((read.get(Id::TintRed) - 1.0).abs() < 1e-4);
    assert_eq!(format(&read), text, "writing what was read changes nothing");
}

#[test]
fn values_are_held_to_their_ranges() {
    let (tuning, skipped) = parse("frost = 999\ntint = -3\nlight_angle = 400\n");
    assert_eq!(skipped, 0);
    assert!((tuning.get(Id::Frost) - 60.0).abs() < f32::EPSILON);
    assert!(tuning.get(Id::Tint).abs() < f32::EPSILON);
    assert!((tuning.get(Id::LightAngle) - 90.0).abs() < f32::EPSILON);
}

#[test]
fn what_does_not_read_is_skipped_and_counted() {
    let text = "# a comment\n\nfrost = 4\nnot_a_knob = 1\ntint = lots\nno equals sign\ntint_color = red\nsee_windows = maybe\n";
    let (tuning, skipped) = parse(text);
    assert_eq!(skipped, 5);
    assert!((tuning.get(Id::Frost) - 4.0).abs() < f32::EPSILON);
    assert!((tuning.get(Id::Tint) - Tuning::DEFAULT.get(Id::Tint)).abs() < f32::EPSILON);
}

#[test]
fn a_value_that_is_not_a_number_never_lands() {
    let (tuning, skipped) = parse("frost = NaN\ntint = inf\n");
    assert_eq!(skipped, 2);
    assert_eq!(tuning, Tuning::DEFAULT);
    let mut set = Tuning::DEFAULT;
    set.set(Id::Lens, f32::NAN);
    assert_eq!(set, Tuning::DEFAULT);
}

#[test]
fn colours_and_switches_read_in_their_forms() {
    let (tuning, skipped) = parse("tint_color = #ff8000\nsee_windows = off\n");
    assert_eq!(skipped, 0);
    let [red, green, blue] = tuning.tint_colour();
    assert!((red - 1.0).abs() < 1e-6 && (green - 128.0 / 255.0).abs() < 1e-6 && blue.abs() < 1e-6);
    assert!(!tuning.is_on(Id::SeeWindows));
    for on in ["on", "yes", "true", "1", "ON"] {
        assert!(
            parse(&format!("see_windows = {on}"))
                .0
                .is_on(Id::SeeWindows),
            "{on}"
        );
    }
    assert_eq!(parse("tint_color = #12345").1, 1);
    assert_eq!(parse("tint_color = #12345g").1, 1);
}

#[test]
fn a_switch_is_on_or_off_and_nothing_between() {
    let mut tuning = Tuning::DEFAULT;
    tuning.set(Id::SeeWindows, 0.3);
    assert!(tuning.get(Id::SeeWindows).abs() < f32::EPSILON);
}

#[test]
fn reading_stops_after_max_lines() {
    let mut text = "frost = 1\n".repeat(MAX_LINES);
    text.push_str("frost = 2\n");
    let (tuning, _) = parse(&text);
    assert!((tuning.get(Id::Frost) - 1.0).abs() < f32::EPSILON);
}

#[test]
fn from_values_holds_each_to_its_range_and_keeps_defaults_for_the_rest() {
    let tuning = Tuning::from_values(&[2.0, f32::NAN]);
    assert!((tuning.get(Id::Tint) - 1.0).abs() < f32::EPSILON);
    assert!((tuning.get(Id::Frost) - Tuning::DEFAULT.get(Id::Frost)).abs() < f32::EPSILON);
    assert_eq!(
        Tuning::from_values(&Tuning::DEFAULT.values()),
        Tuning::DEFAULT
    );
}

#[test]
fn presets_have_unique_names_and_the_first_is_the_default() {
    assert_eq!(PRESETS[0].tuning(), Tuning::DEFAULT);
    for (at, preset) in PRESETS.iter().enumerate() {
        assert_eq!(PRESETS.iter().position(|p| p.name == preset.name), Some(at));
        assert_eq!(
            Preset::named(preset.name).map(|p| p.name),
            Some(preset.name)
        );
        for (id, value) in preset.changes {
            let knob = id.knob();
            assert!(
                (knob.min..=knob.max).contains(value),
                "{}: {}",
                preset.name,
                knob.label
            );
        }
    }
    assert!(Preset::named("nonsense").is_none());
}

#[test]
fn the_glass_comes_as_the_authors_pick() {
    assert_eq!(PRESETS[0].name, "authors-pick");
    assert_eq!(
        Preset::named("authors-pick").map(Preset::tuning),
        Some(Tuning::DEFAULT)
    );
    assert_eq!(parse("").0, Tuning::DEFAULT);
    assert_eq!(
        Preset::named("ios27").map(Preset::tuning),
        Some(Tuning::IOS27)
    );
}

#[test]
fn the_authors_pick_has_no_white_veil_a_darker_tint_and_a_bent_body() {
    let ios = Tuning::IOS27;
    let pick = Preset::named("authors-pick").unwrap().tuning();
    let luma = |tuning: &Tuning| {
        let [red, green, blue] = tuning.tint_colour();
        0.2126 * red + 0.7152 * green + 0.0722 * blue
    };
    assert!(pick.get(Id::Lift).abs() < f32::EPSILON);
    assert!(luma(&pick) < luma(&ios));
    assert!(pick.get(Id::BodyBend) > 0.0);
    assert!(ios.get(Id::BodyBend).abs() < f32::EPSILON);
}

#[test]
fn clear_to_tinted_runs_in_order() {
    let tint = |name: &str| Preset::named(name).map(|p| p.tuning().get(Id::Tint));
    let order = ["ultra-clear", "clear", "ios27", "tinted", "fully-tinted"].map(tint);
    for pair in order.windows(2) {
        assert!(pair[0] < pair[1], "{order:?}");
    }
}

#[test]
fn a_preset_changes_the_glass_and_leaves_the_motion() {
    let mut mine = Tuning::DEFAULT;
    mine.set(Id::Pace, 1.5);
    mine.set(Id::GenieClose, 0.0);
    mine.set(Id::WobbleBounce, 0.2);
    let tinted = Preset::named("tinted")
        .unwrap()
        .tuning()
        .with_motion_of(&mine);
    assert!((tinted.get(Id::Pace) - 1.5).abs() < f32::EPSILON);
    assert!(!tinted.is_on(Id::GenieClose));
    assert!((tinted.get(Id::WobbleBounce) - 0.2).abs() < f32::EPSILON);
    assert!((tinted.get(Id::Tint) - 0.55).abs() < f32::EPSILON);
    for (id, _) in PRESETS.iter().flat_map(|preset| preset.changes) {
        assert_ne!(id.knob().group, Group::Motion, "{}", id.knob().label);
    }
}

#[test]
fn the_motion_comes_with_the_genie_on_close_jelly_and_wobble() {
    let tuning = Tuning::DEFAULT;
    assert!((tuning.get(Id::Pace) - 1.0).abs() < f32::EPSILON);
    assert!(tuning.is_on(Id::GenieClose));
    assert!(!tuning.is_on(Id::GenieOpen));
    assert!(tuning.is_on(Id::Jelly));
    assert!(tuning.is_on(Id::Wobble));
    assert!(!tuning.is_on(Id::WobbleMoves));
    let (read, skipped) = parse("pace = 3\ngenie_close = off\nwobble_time = 0.8\n");
    assert_eq!(skipped, 0);
    assert!((read.get(Id::Pace) - 2.0).abs() < f32::EPSILON);
    assert!(!read.is_on(Id::GenieClose));
    assert!((read.get(Id::WobbleTime) - 0.8).abs() < 1e-6);
    let again = parse(&format(&read)).0;
    for id in Id::ALL {
        if id.knob().group == Group::Motion {
            assert!(
                (again.get(id) - read.get(id)).abs() < 1e-6,
                "{}",
                id.knob().key
            );
        }
    }
}

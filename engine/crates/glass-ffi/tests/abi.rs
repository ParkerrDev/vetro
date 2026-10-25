//! The exported functions, called the way the plugin calls them: with
//! pointers to memory this test owns, and nothing but integers coming back.

#![expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "this file is a test: a failed expectation is a failed test, which is what it is for"
)]

use glass_ffi::*;

fn bytes(text: &str) -> GlassBytes {
    GlassBytes {
        data: text.as_ptr(),
        len: text.len(),
    }
}

fn unset() -> GlassValues {
    GlassValues {
        f: [-1.0; GLASS_FLOAT_OPTIONS],
        i: [-1; GLASS_INT_OPTIONS],
    }
}

/// A block of presets in memory the test owns.
struct Block {
    memory: Vec<u8>,
    store: GlassStore,
}

impl Block {
    fn new() -> Self {
        let mut memory = vec![0_u8; GLASS_STORE_CAPACITY];
        let store = GlassStore {
            data: memory.as_mut_ptr(),
            capacity: memory.len(),
            len: 0,
        };
        Self { memory, store }
    }

    fn committed(user: &Self) -> Self {
        let mut block = Self::new();
        assert_eq!(
            glass_store_commit(Some(&user.store), Some(&mut block.store)),
            GLASS_OK
        );
        block
    }

    fn add(&mut self, line: &str) -> i32 {
        glass_store_add_line(Some(&mut self.store), bytes(line), None)
    }

    fn resolve(&self, preset: &str, dark: bool) -> GlassValues {
        let request = GlassResolveRequest {
            store: GlassBytes {
                data: self.memory.as_ptr(),
                len: self.store.len,
            },
            preset: bytes(preset),
            levels: GlassLevels {
                global: unset(),
                dark: unset(),
                light: unset(),
            },
            dark: u32::from(dark),
        };
        let mut out = unset();
        assert_eq!(
            glass_resolve_options(Some(&request), Some(&mut out)),
            GLASS_OK
        );
        out
    }
}

const FROST: usize = 32;
const MODEL: usize = 5;

#[test]
fn the_interface_has_a_version() {
    assert_eq!(glass_abi_version(), GLASS_ABI_VERSION);
}

#[test]
fn a_null_pointer_is_an_answer_not_a_crash() {
    assert_eq!(glass_frost_plan(9.6, None), GLASS_ERR_NULL);
    assert_eq!(glass_uses_model(None), GLASS_ERR_NULL);
    assert_eq!(glass_layout(None, 1.0, 1.0, None), GLASS_ERR_NULL);
    assert_eq!(glass_store_clear(None), GLASS_ERR_NULL);
    let mut out = GlassRect::default();
    assert_eq!(glass_sample_rect(None, Some(&mut out)), GLASS_ERR_NULL);
}

#[test]
fn the_tone_curve_says_where_it_stops_rising() {
    let mut peak = GlassTonePeak {
        at: -1.0,
        value: -1.0,
    };
    assert_eq!(glass_tone_peak(0.9, 0.0, 1.0, Some(&mut peak)), GLASS_OK);
    assert!(
        peak.at > 0.25 && peak.at < 0.55 && peak.value > 0.27,
        "{peak:?}"
    );
    assert_eq!(glass_tone_peak(0.25, 0.0, 0.82, Some(&mut peak)), GLASS_OK);
    assert!(peak.at > 1.0, "a gentle dim never falls: {peak:?}");
    assert_eq!(glass_tone_peak(0.9, 0.0, 1.0, None), GLASS_ERR_NULL);
}

#[test]
fn every_option_can_be_asked_its_name_and_nothing_else_can() {
    let mut info = GlassOptionInfo {
        key: std::ptr::null(),
        key_len: 0,
        global_default: 0.0,
    };
    let mut keys = Vec::new();
    for (kind, count) in [(0, GLASS_FLOAT_OPTIONS), (1, GLASS_INT_OPTIONS)] {
        for index in 0..count {
            assert_eq!(
                glass_option_info(kind, u32::try_from(index).unwrap(), Some(&mut info)),
                GLASS_OK
            );
            let key = key_of(&info);
            keys.push(key);
        }
        assert_eq!(
            glass_option_info(kind, u32::try_from(count).unwrap(), Some(&mut info)),
            GLASS_ERR_UNKNOWN
        );
    }
    assert_eq!(glass_option_info(2, 0, Some(&mut info)), GLASS_ERR_UNKNOWN);
    assert_eq!(keys[0], "blur_strength");
    assert_eq!(keys[FROST], "frost");
    assert_eq!(keys[GLASS_FLOAT_OPTIONS + MODEL], "model");
}

/// The key of an option, read back out of the static memory it points to.
fn key_of(info: &GlassOptionInfo) -> String {
    // the key points to a static string of this very library: rebuild it from the
    // known options instead of dereferencing a pointer in a test
    glass_core::options::FloatOption::ALL
        .iter()
        .map(|o| (o.key(), o.key().as_ptr()))
        .chain(
            glass_core::options::IntOption::ALL
                .iter()
                .map(|o| (o.key(), o.key().as_ptr())),
        )
        .find(|(key, ptr)| *ptr == info.key && key.len() == info.key_len)
        .map(|(key, _)| key.to_owned())
        .unwrap()
}

#[test]
fn presets_are_edited_in_a_block_the_caller_owns() {
    let mut user = Block::new();
    assert_eq!(user.add("name:mine, inherits:regular, frost:9"), GLASS_OK);
    assert_eq!(user.add("name:mine:light, saturation:1.5"), GLASS_OK);
    assert_eq!(
        glass_store_define(Some(&mut user.store), bytes("empty*")),
        GLASS_OK
    );
    let committed = Block::committed(&user);
    assert_eq!(
        glass_store_contains(Some(&committed.store), bytes("mine")),
        1
    );
    assert_eq!(
        glass_store_contains(Some(&committed.store), bytes("empty")),
        1
    );
    assert_eq!(
        glass_store_contains(Some(&committed.store), bytes("regular")),
        1
    );
    assert_eq!(
        glass_store_contains(Some(&committed.store), bytes("nothing")),
        0
    );
    let values = committed.resolve("mine", true);
    assert!((values.f[FROST] - 9.0).abs() < 1e-6, "{}", values.f[FROST]);
    assert_eq!(values.i[MODEL], 1, "it inherits regular's model");
    assert!(
        committed.resolve("mine", false).f[12] > 1.4,
        "saturation of the light variant"
    );
    assert!((committed.resolve("nothing", true).f[FROST] + 1.0).abs() < 1e-6);
}

#[test]
fn a_line_that_does_not_read_says_why() {
    let mut user = Block::new();
    let mut message = GlassText {
        bytes: [0; GLASS_TEXT_BYTES],
        len: 0,
    };
    assert_eq!(
        glass_store_add_line(Some(&mut user.store), bytes("frost:4"), Some(&mut message)),
        GLASS_ERR_PARSE
    );
    let text = std::str::from_utf8(&message.bytes[..message.len as usize]).unwrap();
    assert_eq!(text, "preset: missing required 'name' field");
    assert_eq!(user.add("name:x:blue"), GLASS_ERR_PARSE);
    assert_eq!(user.add("name:\u{ff}"), GLASS_OK, "UTF-8 names are names");
    let bad = GlassBytes {
        data: [0xff_u8, 0xfe].as_ptr(),
        len: 2,
    };
    assert_eq!(
        glass_store_add_line(Some(&mut user.store), bad, None),
        GLASS_ERR_ARG,
        "and bytes that are not UTF-8 are refused"
    );
}

#[test]
fn single_settings_come_one_at_a_time_as_lua_gives_them() {
    let mut user = Block::new();
    let edit = |variant, key: &'static str, value: &'static str| GlassFieldEdit {
        name: bytes("lua"),
        variant,
        key: bytes(key),
        value: bytes(value),
    };
    assert_eq!(
        glass_store_set_field(Some(&mut user.store), Some(&edit(1, "frost", "7.000000"))),
        1
    );
    assert_eq!(
        glass_store_set_field(Some(&mut user.store), Some(&edit(1, "wobble", "1"))),
        0,
        "an unknown key is told, not an error"
    );
    assert_eq!(
        glass_store_set_field(Some(&mut user.store), Some(&edit(3, "frost", "1"))),
        GLASS_ERR_ARG
    );
    assert_eq!(
        glass_store_set_inherits(Some(&mut user.store), bytes("lua"), bytes("clear")),
        GLASS_OK
    );
    let committed = Block::committed(&user);
    assert!((committed.resolve("lua", true).f[FROST] - 7.0).abs() < 1e-6);
    assert!(
        (committed.resolve("lua", false).f[FROST] - 2.0).abs() < 1e-6,
        "clear's, in the other appearance"
    );
    assert_eq!(
        glass_store_any_positive(Some(&committed.store), 25),
        0,
        "self_sample is set nowhere"
    );
    assert_eq!(
        glass_store_any_positive(Some(&committed.store), 99),
        GLASS_ERR_UNKNOWN
    );
}

#[test]
fn a_store_that_does_not_fit_its_block_is_refused_and_left_alone() {
    let mut memory = vec![0_u8; 64];
    let mut small = GlassStore {
        data: memory.as_mut_ptr(),
        capacity: 64,
        len: 0,
    };
    assert_eq!(
        glass_store_add_line(Some(&mut small), bytes("name:a, frost:1"), None),
        GLASS_ERR_CAPACITY
    );
    assert_eq!(small.len, 0);
}

#[test]
fn options_become_the_models_parameters() {
    let committed = Block::committed(&Block::new());
    let options = committed.resolve("regular", true);
    assert_eq!(glass_uses_model(Some(&options)), 1);
    assert_eq!(glass_uses_model(Some(&committed.resolve("pomme", true))), 0);
    let mut params = zeroed_params();
    assert_eq!(
        glass_model_params(Some(&options), 1.6, Some(&mut params)),
        GLASS_OK
    );
    assert!((params.lens.bezel_width - 32.0).abs() < 1e-5 && (params.frost - 9.6).abs() < 1e-5);
    let mut changes = GlassChanges {
        field: 9,
        lens: 9,
        frost: 9,
    };
    assert_eq!(
        glass_params_changes(Some(&params), Some(&params), Some(&mut changes)),
        GLASS_OK
    );
    assert_eq!((changes.field, changes.lens, changes.frost), (0, 0, 0));
    let mut other = params;
    other.lens.dispersion = 0.5;
    assert_eq!(
        glass_params_changes(Some(&params), Some(&other), Some(&mut changes)),
        GLASS_OK
    );
    assert_eq!((changes.field, changes.lens, changes.frost), (0, 1, 0));
}

fn zeroed_params() -> GlassParams {
    let lens = GlassLens {
        bezel_width: 0.0,
        thickness: 0.0,
        refractive_index: 0.0,
        profile_exponent: 0.0,
        refraction: 0.0,
        dispersion: 0.0,
        paraxial: 0,
    };
    GlassParams {
        lens,
        frost: 0.0,
        light_angle: 0.0,
        light_intensity: 0.0,
        counter_light: 0.0,
        rim_width: 0.0,
        rim_glow: 0.0,
        inner_shadow: 0.0,
        shadow_opacity: 0.0,
        shadow_radius: 0.0,
        shadow_offset: 0.0,
        tint_over_dark: 0.0,
        tint_over_bright: 0.0,
        saturation: 0.0,
        brightness: 0.0,
        contrast: 0.0,
        noise_strength: 0.0,
        glass_opacity: 0.0,
        tint_color: [0.0; 3],
        layered: GlassLayered::default(),
        quartz: 0,
    }
}

fn regular_params(scale: f32) -> GlassParams {
    let options = Block::committed(&Block::new()).resolve("regular", true);
    let mut params = zeroed_params();
    assert_eq!(
        glass_model_params(Some(&options), scale, Some(&mut params)),
        GLASS_OK
    );
    params
}

#[test]
fn the_lens_table_is_written_into_the_callers_buffer() {
    let params = regular_params(1.6);
    let mut table = vec![0_u8; 2 * GLASS_LENS_TABLE_SIZE * 4];
    let mut scale = 0.0;
    assert_eq!(
        glass_lens_table(
            Some(&params.lens),
            table.as_mut_ptr(),
            table.len(),
            Some(&mut scale)
        ),
        GLASS_OK
    );
    assert!((scale - 1.25_f64.sqrt()).abs() < 1e-12);
    assert_eq!(table[3], 255, "row 0 starts opaque");
    let mut short = vec![0_u8; 100];
    assert_eq!(
        glass_lens_table(
            Some(&params.lens),
            short.as_mut_ptr(),
            short.len(),
            Some(&mut scale)
        ),
        GLASS_ERR_CAPACITY
    );
    assert_eq!(
        glass_lens_table(
            Some(&params.lens),
            std::ptr::null_mut(),
            8192,
            Some(&mut scale)
        ),
        GLASS_ERR_NULL
    );
}

#[test]
fn the_field_and_the_frost_are_laid_out_as_the_core_works_out() {
    let params = regular_params(1.6);
    let mut layout = zero_layout();
    assert_eq!(
        glass_layout(Some(&params), 2544.0, 48.0, Some(&mut layout)),
        GLASS_OK
    );
    assert_eq!(
        (layout.field_scale, layout.margin, layout.shadow_reach),
        (4, 96, 77)
    );
    assert_eq!(layout.field_texels, [684, 60]);
    let mut frost = GlassFrostPlan {
        reduction: 0,
        sigma: 0.0,
        face_applied: 0,
        passes: 0,
        pass_sigma: 0.0,
        pairs: 0,
    };
    assert_eq!(glass_frost_plan(params.frost, Some(&mut frost)), GLASS_OK);
    assert_eq!(
        (
            frost.reduction,
            frost.passes,
            frost.pairs,
            frost.face_applied
        ),
        (4, 1, 4, 1)
    );
}

#[test]
fn the_frost_is_read_from_a_snapped_rectangle_and_the_reaches_make_room() {
    let params = regular_params(1.6);
    let query = GlassSampleQuery {
        pane: GlassRect {
            x: 13.0,
            y: 7.0,
            w: 101.0,
            h: 33.0,
        },
        box_x: 100.0,
        box_y: 20.0,
        source_w: 2560.0,
        source_h: 1600.0,
        frost: params.frost,
        reach: 24,
    };
    let mut rect = GlassRect::default();
    assert_eq!(glass_sample_rect(Some(&query), Some(&mut rect)), GLASS_OK);
    assert_eq!((rect.x, rect.w), (84.0, 160.0));
    let mut reaches = GlassReaches {
        frost: 0,
        shadow: 0,
        damage: 0,
        padding: 0,
        repaint: 0.0,
    };
    assert_eq!(
        glass_reaches(Some(&params), 77, Some(&mut reaches)),
        GLASS_OK
    );
    assert_eq!(
        (reaches.frost, reaches.damage, reaches.padding),
        (24, 77, 77)
    );
}

#[test]
fn the_glass_shader_is_given_its_constants_and_opacities() {
    let params = regular_params(1.6);
    let mut constants = GlassDrawConstants {
        thickness_per_fit: 0.0,
        interior_coverage: 0.0,
        solid_coverage: 0.0,
        outer_coverage: 0.0,
        rim_coverage: 0.0,
        deviation_scale: 0.0,
        dispersive: 9,
        light_direction: [0.0; 2],
    };
    assert_eq!(
        glass_draw_constants(Some(&params), 1.25_f64.sqrt(), Some(&mut constants)),
        GLASS_OK
    );
    assert_eq!(constants.dispersive, 0);
    let mut opacities = GlassOpacities {
        glass: 0.0,
        shadow: 0.0,
    };
    assert_eq!(
        glass_opacities(Some(&params), 0.5, Some(&mut opacities)),
        GLASS_OK
    );
    assert!((opacities.glass - 0.5).abs() < 1e-6 && (opacities.shadow - 0.06).abs() < 1e-6);
}

fn zero_layout() -> GlassLayout {
    GlassLayout {
        field_sigma: 0.0,
        field_scale: 0,
        margin: 0,
        shadow_reach: 0,
        field_texels: [0; 2],
        shadow_texels: [0; 2],
        cells: [0; 2],
        initial_fit: 0,
        blur_sigma: 0.0,
        blur_radius: 0,
        shadow_on: 0,
        shadow_sigma: 0.0,
        shadow_taps: 0,
        small: 0,
    }
}

/// A map of cells with one pane in it, as the GPU would draw it.
fn one_pane_map() -> (Vec<u8>, usize, usize) {
    let (width, height) = (30, 20);
    let mut cells = vec![0_u8; width * height * 4];
    for y in 3..=10 {
        for x in 4..=17 {
            let edge = x == 4 || x == 17 || y == 3 || y == 10;
            let face = (6..=15).contains(&x) && (5..=8).contains(&y);
            let at = (y * width + x) * 4;
            cells[at] = if edge { 100 } else { 255 };
            cells[at + 1] = if face { 255 } else { 0 };
        }
    }
    (cells, width, height)
}

fn no_room() -> GlassPaneBuffers {
    GlassPaneBuffers {
        panes: std::ptr::null_mut(),
        pane_capacity: 0,
        interiors: std::ptr::null_mut(),
        interior_capacity: 0,
        fit: std::ptr::null_mut(),
        fit_capacity: 0,
        pane_count: 0,
        interior_count: 0,
        fit_count: 0,
    }
}

fn map_request(cells: &[u8], width: usize, height: usize) -> GlassMapRequest {
    GlassMapRequest {
        cells: GlassBytes {
            data: cells.as_ptr(),
            len: cells.len(),
        },
        width,
        height,
        margin: 0,
        field_scale: 1,
        sigma: 10.0,
        bezel_width: 20.0,
        box_w: 240.0,
        box_h: 160.0,
    }
}

#[test]
fn a_map_that_needs_more_room_says_how_much() {
    let (cells, width, height) = one_pane_map();
    let mut none = no_room();
    assert_eq!(
        glass_panes_from_cells(Some(&map_request(&cells, width, height)), Some(&mut none)),
        GLASS_ERR_CAPACITY
    );
    assert_eq!(
        (none.pane_count, none.interior_count, none.fit_count),
        (1, 1, width * height)
    );
}

#[test]
fn panes_are_read_from_cells_into_the_callers_room() {
    let (cells, width, height) = one_pane_map();
    let request = map_request(&cells, width, height);
    let mut panes = vec![
        GlassPane {
            rect: GlassRect::default(),
            short_side: 0.0,
            first_interior: 0,
            interior_count: 0
        };
        1
    ];
    let mut interiors = vec![GlassRect::default(); 1];
    let mut fit = vec![0_u8; width * height];
    let mut room = GlassPaneBuffers {
        panes: panes.as_mut_ptr(),
        pane_capacity: 1,
        interiors: interiors.as_mut_ptr(),
        interior_capacity: 1,
        fit: fit.as_mut_ptr(),
        fit_capacity: fit.len(),
        ..no_room()
    };
    assert_eq!(
        glass_panes_from_cells(Some(&request), Some(&mut room)),
        GLASS_OK
    );
    assert_eq!(
        panes[0].rect,
        GlassRect {
            x: 32.0,
            y: 24.0,
            w: 112.0,
            h: 64.0
        }
    );
    assert_eq!((panes[0].first_interior, panes[0].interior_count), (0, 1));
    assert_eq!(
        interiors[0],
        GlassRect {
            x: 48.0,
            y: 40.0,
            w: 80.0,
            h: 32.0
        }
    );
    assert_eq!(fit[0], 255);
    assert!(fit[8 * width + 8] < 255);
}

fn wanted_request(
    known_pane: &GlassPane,
    face: &GlassRect,
    face_count: usize,
) -> GlassWantedRequest {
    GlassWantedRequest {
        known: GlassPaneList {
            panes: std::ptr::from_ref(known_pane),
            pane_count: 1,
            interiors: std::ptr::from_ref(face),
            interior_count: face_count,
        },
        has_known: 1,
        unknown: GlassRect {
            x: 500.0,
            y: 500.0,
            w: 50.0,
            h: 40.0,
        },
        has_unknown: 1,
        whole: GlassRect {
            x: 0.0,
            y: 0.0,
            w: 1000.0,
            h: 800.0,
        },
        small: 0,
    }
}

fn known_pane() -> GlassPane {
    GlassPane {
        rect: GlassRect {
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 100.0,
        },
        short_side: 100.0,
        first_interior: 0,
        interior_count: 1,
    }
}

#[test]
fn the_panes_that_stand_now_are_the_known_ones_and_what_was_redrawn() {
    let (pane, face) = (
        known_pane(),
        GlassRect {
            x: 10.0,
            y: 10.0,
            w: 20.0,
            h: 20.0,
        },
    );
    let request = wanted_request(&pane, &face, 1);
    let mut panes = vec![pane; 4];
    let mut interiors = vec![face; 4];
    let mut out = GlassPaneBuffers {
        panes: panes.as_mut_ptr(),
        pane_capacity: 4,
        interiors: interiors.as_mut_ptr(),
        interior_capacity: 4,
        ..no_room()
    };
    assert_eq!(glass_panes_wanted(Some(&request), Some(&mut out)), GLASS_OK);
    assert_eq!((out.pane_count, out.interior_count), (2, 1));
    assert_eq!(panes[1].rect, request.unknown);
}

#[test]
fn a_face_rectangle_that_is_not_in_the_list_is_refused() {
    let (pane, face) = (
        known_pane(),
        GlassRect {
            x: 10.0,
            y: 10.0,
            w: 20.0,
            h: 20.0,
        },
    );
    let bad = wanted_request(&pane, &face, 0);
    let mut out = no_room();
    assert_eq!(
        glass_panes_wanted(Some(&bad), Some(&mut out)),
        GLASS_ERR_ARG
    );
}

#[test]
fn shaders_are_asked_their_length_and_then_their_text() {
    assert_eq!(glass_shader_count(), 17);
    for index in 0..glass_shader_count() {
        let (mut name_len, mut source_len) = (0_usize, 0_usize);
        assert_eq!(
            glass_shader_name(index, std::ptr::null_mut(), 0, Some(&mut name_len)),
            GLASS_OK
        );
        assert_eq!(
            glass_shader_source(index, std::ptr::null_mut(), 0, Some(&mut source_len)),
            GLASS_OK
        );
        let mut name = vec![0_u8; name_len];
        let mut source = vec![0_u8; source_len];
        assert_eq!(
            glass_shader_name(index, name.as_mut_ptr(), name.len(), Some(&mut name_len)),
            GLASS_OK
        );
        assert_eq!(
            glass_shader_source(
                index,
                source.as_mut_ptr(),
                source.len(),
                Some(&mut source_len)
            ),
            GLASS_OK
        );
        let name = String::from_utf8(name).unwrap();
        assert_eq!(
            String::from_utf8(source).unwrap(),
            glass_core::shaders::source(&name).unwrap()
        );
    }
    let mut length = 0;
    assert_eq!(
        glass_shader_name(99, std::ptr::null_mut(), 0, Some(&mut length)),
        GLASS_ERR_UNKNOWN
    );
    let mut tiny = [0_u8; 2];
    assert_eq!(
        glass_shader_name(0, tiny.as_mut_ptr(), 2, Some(&mut length)),
        GLASS_ERR_CAPACITY
    );
}

#[test]
fn a_spring_is_stepped_in_the_callers_memory_and_retargeting_is_setting_the_target() {
    let mut spring = GlassSpring {
        position: 0.0,
        velocity: 0.0,
        target: 1.0,
        duration: 0.4,
        bounce: 0.0,
    };
    for _ in 0..400 {
        assert_eq!(glass_spring_step(Some(&mut spring), 0.01), GLASS_OK);
        assert!(
            spring.position <= 1.0 + 1e-12,
            "critical damping never overshoots"
        );
    }
    assert!((spring.position - 1.0).abs() < 1e-6);
    spring.target = -2.0;
    let before = spring.velocity;
    assert_eq!(glass_spring_step(Some(&mut spring), 0.0), GLASS_OK);
    assert!(
        (spring.velocity - before).abs() < 1e-15,
        "no time, no change"
    );
    assert_eq!(glass_spring_step(None, 0.1), GLASS_ERR_NULL);
}

#[test]
fn the_selector_stretches_and_the_rubber_band_gives_way() {
    let mut stretch = GlassStretch {
        along: 0.0,
        across: 0.0,
    };
    assert_eq!(
        glass_selector_stretch(400.0, 1, Some(&mut stretch)),
        GLASS_OK
    );
    assert!((stretch.along - 1.14).abs() < 1e-12);
    let mut shown = 0.0;
    assert_eq!(glass_rubber_band(100.0, 20.0, Some(&mut shown)), GLASS_OK);
    assert!(shown > 0.0 && shown < 20.0);
}

#[test]
fn a_declared_rectangle_and_a_union_of_two_samples() {
    let shape = GlassDeclaredRect {
        width: 400.0,
        height: 300.0,
        radii: [40.0; 4],
    };
    let mut sample = GlassSample {
        distance: 1.0,
        normal: [0.0; 2],
    };
    assert_eq!(
        glass_declared_sample(Some(&shape), 230.0, 0.0, Some(&mut sample)),
        GLASS_OK
    );
    assert!(
        (sample.distance - 30.0).abs() < 1e-9
            && (sample.normal[0] - 1.0).abs() < 1e-12
            && sample.normal[1].abs() < 1e-12
    );
    let two = [
        GlassSample {
            distance: 1.0,
            normal: [1.0, 0.0],
        },
        GlassSample {
            distance: 1.0,
            normal: [-1.0, 0.0],
        },
    ];
    let mut joined = sample;
    assert_eq!(
        glass_union(two.as_ptr(), two.len(), 10.0, Some(&mut joined)),
        GLASS_OK
    );
    assert!(joined.distance < 1.0, "facing outlines join with a neck");
    assert_eq!(
        glass_union(two.as_ptr(), 0, 10.0, Some(&mut joined)),
        GLASS_ERR_ARG,
        "nothing to join"
    );
}

#[test]
fn the_current_material_crosses_whole() {
    let committed = Block::committed(&Block::new());
    let options = committed.resolve("regular27", true);
    assert_eq!(glass_uses_model(Some(&options)), 1);
    let mut params = zeroed_params();
    assert_eq!(
        glass_model_params(Some(&options), 1.6, Some(&mut params)),
        GLASS_OK
    );
    assert_eq!(params.lens.paraxial, 1);
    assert!(
        (params.lens.bezel_width - 17.12).abs() < 1e-4
            && (params.lens.thickness - 17.12).abs() < 1e-4
    );
    assert!((params.frost - 0.96).abs() < 1e-5, "the ghost: 0.6 points");
    let layered = params.layered;
    assert!(
        (layered.frost_wide - 22.4).abs() < 1e-4,
        "the cloud: 14 points"
    );
    assert!((layered.frost_cloud - 0.85).abs() < 1e-6 && (layered.frost_hold + 0.45).abs() < 1e-6);
    assert!((layered.cloud_weight - 0.5).abs() < 1e-6 && (layered.ghost_weight - 2.5).abs() < 1e-6);
    assert!((layered.body_shade - 0.28).abs() < 1e-6 && (layered.rim_light - 1.15).abs() < 1e-6);
    assert!((layered.rim_shade - 1.0).abs() < 1e-6 && layered.rim_shade_ends.abs() < 1e-6);
    assert!((params.light_angle - 180.0).abs() < 1e-5 && params.light_intensity.abs() < 1e-6);
    assert!(
        params.shadow_opacity.abs() < 1e-6,
        "the dark appearance casts none"
    );
}

#[test]
fn the_current_materials_cloud_and_lens_are_planned_as_any_are() {
    let committed = Block::committed(&Block::new());
    let mut params = zeroed_params();
    assert_eq!(
        glass_model_params(
            Some(&committed.resolve("regular27", true)),
            1.6,
            Some(&mut params)
        ),
        GLASS_OK
    );
    let layered = params.layered;
    // the cloud is what sizes the reach, and it is planned as any frost is: an eighth of the screen
    let mut reaches = GlassReaches {
        frost: 0,
        shadow: 0,
        damage: 0,
        padding: 0,
        repaint: 0.0,
    };
    assert_eq!(
        glass_reaches(Some(&params), 0, Some(&mut reaches)),
        GLASS_OK
    );
    assert_eq!(reaches.frost, 56, "two and a half deviations of 22.4");
    let mut plan = GlassFrostPlan {
        reduction: 0,
        sigma: 0.0,
        face_applied: 0,
        passes: 0,
        pass_sigma: 0.0,
        pairs: 0,
    };
    assert_eq!(
        glass_frost_plan(layered.frost_wide, Some(&mut plan)),
        GLASS_OK
    );
    assert_eq!((plan.reduction, plan.passes), (8, 1));

    // the lens table is the thin prism's: on the outline the whole of its scale, no height
    let mut table = vec![0_u8; 2 * GLASS_LENS_TABLE_SIZE * 4];
    let mut scale = 0.0;
    assert_eq!(
        glass_lens_table(
            Some(&params.lens),
            table.as_mut_ptr(),
            table.len(),
            Some(&mut scale)
        ),
        GLASS_OK
    );
    assert!((scale - 8.0 * 0.24).abs() < 1e-6);
    let row_one = &table[GLASS_LENS_TABLE_SIZE * 4..];
    assert_eq!((row_one[1], row_one[3]), (255, 0));
}

#[test]
fn the_current_materials_light_appearance_is_its_own() {
    let committed = Block::committed(&Block::new());
    let mut params = zeroed_params();
    assert_eq!(
        glass_model_params(
            Some(&committed.resolve("regular27", true)),
            1.6,
            Some(&mut params)
        ),
        GLASS_OK
    );
    // and the light appearance is another material in every one of these
    let mut light = zeroed_params();
    assert_eq!(
        glass_model_params(
            Some(&committed.resolve("regular27", false)),
            1.0,
            Some(&mut light)
        ),
        GLASS_OK
    );
    assert!(
        (light.layered.frost_hold - 0.4).abs() < 1e-6
            && (light.shadow_opacity - 0.016).abs() < 1e-6
    );
    let mut changes = GlassChanges {
        field: 9,
        lens: 9,
        frost: 9,
    };
    assert_eq!(
        glass_params_changes(
            Some(&regular_params(1.6)),
            Some(&params),
            Some(&mut changes)
        ),
        GLASS_OK
    );
    assert_eq!((changes.field, changes.lens, changes.frost), (1, 1, 1));
}

#[test]
fn the_rims_constants_are_handed_to_a_shader_the_core_does_not_make() {
    let mut rim = GlassRimConstants::default();
    assert_eq!(glass_rim_constants(Some(&mut rim)), GLASS_OK);
    assert!((rim.opposite - 0.85).abs() < 1e-6 && (rim.luma_power - 0.3).abs() < 1e-6);
    assert!((rim.core[0] - 0.175).abs() < 1e-6 && (rim.core[1] - 1.5).abs() < 1e-6);
    assert!((rim.tail[0] - 0.035).abs() < 1e-6 && (rim.tail[1] - 8.0).abs() < 1e-6);
    assert!(
        (rim.inset - 2.25).abs() < 1e-6,
        "three quarters of a point, in thirds"
    );
    assert!((rim.hairline_shade - 0.314).abs() < 1e-6 && (rim.hairline_power - 1.75).abs() < 1e-6);
    assert!(
        (rim.hairline_width[0] - 0.75).abs() < 1e-6 && (rim.hairline_width[1] - 2.0).abs() < 1e-6
    );
    assert!((rim.hairline_read - 2.5).abs() < 1e-6);
    assert_eq!(glass_rim_constants(None), GLASS_ERR_NULL);
}

#[test]
fn apple_s_shader_is_given_its_block_for_each_preset_and_none_for_no_preset() {
    let mut out = GlassQuartz {
        block: [0.0; GLASS_QUARTZ_FLOATS],
        scale_ref: 0.0,
        reach: 0.0,
        blur: 0.0,
    };
    assert_eq!(glass_quartz(1, Some(&mut out)), GLASS_OK);
    assert!(
        (out.blur - 28.0).abs() < f32::EPSILON,
        "the regular panel's blur"
    );
    assert!(
        (out.block[4] - 4.0).abs() < f32::EPSILON,
        "inner refraction at float 4"
    );
    assert!((out.scale_ref - 75.0).abs() < f32::EPSILON);
    assert!(out.reach > 0.0);
    for code in 2..=5 {
        assert_eq!(glass_quartz(code, Some(&mut out)), GLASS_OK);
    }
    // the lens: its tint follows the bright layer and the hairline, at float 100
    assert!(
        out.block[103] > 0.0 && out.block[99] > 0.0,
        "the lens's tint and hairline"
    );
    assert!(
        out.blur < 28.0,
        "the lens blurs less than the frosted panel"
    );
    assert!(glass_quartz(0, Some(&mut out)) < 0);
    assert!(glass_quartz(9, Some(&mut out)) < 0);
}

fn empty_text() -> GlassText {
    GlassText {
        bytes: [0; GLASS_TEXT_BYTES],
        len: 0,
    }
}

fn text(slot: &GlassText) -> String {
    String::from_utf8(slot.bytes[..slot.len as usize].to_vec()).unwrap()
}

#[test]
fn a_tuning_is_read_and_written_as_a_file() {
    let mut tuning = GlassTuning {
        values: [0.0; GLASS_TUNING_KNOBS],
    };
    assert_eq!(glass_tuning_default(Some(&mut tuning)), GLASS_OK);
    // a file's text over the defaults: one setting, one line that does not read
    assert_eq!(
        glass_tuning_parse(bytes("frost = 21\nnonsense = 2\n"), Some(&mut tuning)),
        1
    );
    assert!(
        (tuning.values[1] - 21.0).abs() < f32::EPSILON,
        "frost is knob 1"
    );
    // and written back out: the length first, then the text
    let mut length = 0_usize;
    assert_eq!(
        glass_tuning_format(Some(&tuning), std::ptr::null_mut(), 0, Some(&mut length)),
        GLASS_OK
    );
    let mut written = vec![0_u8; length];
    assert_eq!(
        glass_tuning_format(
            Some(&tuning),
            written.as_mut_ptr(),
            written.len(),
            Some(&mut length)
        ),
        GLASS_OK
    );
    let written = String::from_utf8(written).unwrap();
    assert!(written.contains("frost = 21.0"), "{written}");
    assert!(
        glass_tuning_format(
            Some(&tuning),
            written.clone().as_mut_ptr(),
            3,
            Some(&mut length)
        ) < 0,
        "too small a buffer"
    );
    // a knob set from C is held to its range
    assert_eq!(glass_tuning_set(Some(&mut tuning), 1, 500.0), GLASS_OK);
    assert!((tuning.values[1] - 60.0).abs() < f32::EPSILON);
    assert!(
        glass_tuning_set(
            Some(&mut tuning),
            u32::try_from(GLASS_TUNING_KNOBS).unwrap(),
            1.0
        ) < 0
    );
}

#[test]
fn a_tuning_becomes_the_block_of_apples_shader() {
    let mut tuning = GlassTuning {
        values: [0.0; GLASS_TUNING_KNOBS],
    };
    assert_eq!(glass_tuning_default(Some(&mut tuning)), GLASS_OK);
    let defaults = tuning;
    assert_eq!(glass_tuning_set(Some(&mut tuning), 1, 500.0), GLASS_OK);
    // the block of a tuning carries its frost as the face's blur
    let mut out = GlassQuartz {
        block: [0.0; GLASS_QUARTZ_FLOATS],
        scale_ref: 0.0,
        reach: 0.0,
        blur: 0.0,
    };
    assert_eq!(glass_quartz_tuned(Some(&tuning), Some(&mut out)), GLASS_OK);
    assert!((out.blur - 60.0).abs() < f32::EPSILON);
    assert_eq!(
        glass_quartz_tuned(Some(&defaults), Some(&mut out)),
        GLASS_OK
    );
    let mut coded = out;
    assert_eq!(glass_quartz(6, Some(&mut coded)), GLASS_OK);
    assert_eq!(coded.block, out.block, "kind 6 is the default tuning");
    assert_eq!(glass_quartz_tuned(None, Some(&mut out)), GLASS_ERR_NULL);
}

#[test]
fn every_knob_and_preset_is_described() {
    let mut knob = GlassKnob {
        key: empty_text(),
        label: empty_text(),
        help: empty_text(),
        unit: empty_text(),
        group: 9,
        shown: 9,
        min: 0.0,
        max: 0.0,
        step: 0.0,
        default_value: 0.0,
    };
    for index in 0..u32::try_from(GLASS_TUNING_KNOBS).unwrap() {
        assert_eq!(glass_tuning_knob(index, Some(&mut knob)), GLASS_OK);
        assert!(knob.shown <= 3 && knob.group <= 4);
        assert!(knob.min <= knob.default_value && knob.default_value <= knob.max);
        if knob.shown != 3 {
            assert_ne!(text(&knob.label), "");
        }
    }
    assert_eq!(glass_tuning_knob(0, Some(&mut knob)), GLASS_OK);
    assert_eq!(text(&knob.key), "tint");
    assert!(glass_tuning_knob(u32::try_from(GLASS_TUNING_KNOBS).unwrap(), Some(&mut knob)) < 0);

    let mut preset = GlassTuningPreset {
        name: empty_text(),
        label: empty_text(),
    };
    let mut tuning = GlassTuning {
        values: [0.0; GLASS_TUNING_KNOBS],
    };
    for index in 0..u32::try_from(GLASS_TUNING_PRESETS).unwrap() {
        assert_eq!(glass_tuning_preset_info(index, Some(&mut preset)), GLASS_OK);
        let name = text(&preset.name);
        assert_eq!(
            glass_tuning_preset(bytes(&name), Some(&mut tuning)),
            GLASS_OK,
            "{name}"
        );
    }
    assert!(glass_tuning_preset(bytes("no such preset"), Some(&mut tuning)) < 0);
}

#[test]
fn a_preset_keeps_the_motion_the_user_set() {
    let mut tuning = GlassTuning {
        values: [0.0; GLASS_TUNING_KNOBS],
    };
    assert_eq!(glass_tuning_default(Some(&mut tuning)), GLASS_OK);
    assert_eq!(
        glass_tuning_set(Some(&mut tuning), GLASS_T_PACE, 1.5),
        GLASS_OK
    );
    assert_eq!(
        glass_tuning_set(Some(&mut tuning), GLASS_T_GENIE_CLOSE, 0.0),
        GLASS_OK
    );
    assert_eq!(
        glass_tuning_preset(bytes("tinted"), Some(&mut tuning)),
        GLASS_OK
    );
    let pace = usize::try_from(GLASS_T_PACE).unwrap();
    let genie = usize::try_from(GLASS_T_GENIE_CLOSE).unwrap();
    assert!((tuning.values[pace] - 1.5).abs() < f32::EPSILON);
    assert!(tuning.values[genie].abs() < f32::EPSILON);
    assert!((tuning.values[0] - 0.55).abs() < f32::EPSILON);
}

#[test]
fn the_see_windows_constant_names_its_knob() {
    assert_eq!(
        usize::try_from(GLASS_T_SEE_WINDOWS).unwrap(),
        glass_core::tuning::Id::SeeWindows.index()
    );
}

fn mesh_of(points: &mut [f32], cols: u32, rows: u32) -> GlassMesh {
    GlassMesh {
        points: points.as_mut_ptr(),
        capacity: points.len(),
        cols,
        rows,
    }
}

#[test]
fn a_genie_mesh_is_written_into_the_callers_room() {
    let window = GlassRect {
        x: 0.0,
        y: 0.0,
        w: 200.0,
        h: 100.0,
    };
    let mouth = GlassMouth {
        x: 100.0,
        y: 500.0,
        width: 20.0,
    };
    let mut points = vec![0.0_f32; 2 * 3 * 5];
    let mut mesh = mesh_of(&mut points, 2, 4);
    assert_eq!(
        glass_genie_mesh(Some(&window), Some(&mouth), 0.0, Some(&mut mesh)),
        15
    );
    assert_eq!((points[28], points[29]), (200.0, 100.0));
    let mut mesh = mesh_of(&mut points, 2, 4);
    assert_eq!(
        glass_genie_mesh(Some(&window), Some(&mouth), 1.0, Some(&mut mesh)),
        15
    );
    assert!(points.chunks_exact(2).all(|p| (p[1] - 500.0).abs() < 1e-3));
    let mut small = vec![0.0_f32; 4];
    let mut mesh = mesh_of(&mut small, 2, 4);
    assert_eq!(
        glass_genie_mesh(Some(&window), Some(&mouth), 0.5, Some(&mut mesh)),
        GLASS_ERR_CAPACITY
    );
    let mut mesh = mesh_of(&mut points, 0, 4);
    assert_eq!(
        glass_genie_mesh(Some(&window), Some(&mouth), 0.5, Some(&mut mesh)),
        GLASS_ERR_ARG
    );
    assert_eq!(
        glass_genie_mesh(None, Some(&mouth), 0.5, None),
        GLASS_ERR_NULL
    );
}

#[test]
fn a_wobbling_sheet_lives_in_the_callers_memory() {
    let spec = GlassWobbleSpec {
        duration: 0.5,
        bounce: 0.5,
        coupling: 2.0,
        reach: 0.3,
    };
    let mut sheet = GlassWobble {
        offset: [0.0; GLASS_WOBBLE_FLOATS],
        velocity: [0.0; GLASS_WOBBLE_FLOATS],
        held: -1,
    };
    assert_eq!(glass_wobble_hold(Some(&mut sheet), 0.5, 0.5), GLASS_OK);
    assert_eq!(sheet.held, 12);
    let from = GlassRect {
        x: 0.0,
        y: 0.0,
        w: 300.0,
        h: 200.0,
    };
    let to = GlassRect { x: 30.0, ..from };
    assert_eq!(
        glass_wobble_moved(Some(&mut sheet), Some(&spec), Some(&from), Some(&to)),
        GLASS_OK
    );
    assert_eq!(sheet.offset[0], -30.0);
    assert_eq!(
        sheet.offset[24], 0.0,
        "the held point moved with the window"
    );
    assert_eq!(
        glass_wobble_step(Some(&mut sheet), Some(&spec), 1.0 / 60.0),
        0
    );
    let mut points = vec![0.0_f32; 2 * 9 * 9];
    let mut mesh = mesh_of(&mut points, 8, 8);
    assert_eq!(
        glass_wobble_mesh(Some(&sheet), Some(&to), Some(&mut mesh)),
        81
    );
    assert_eq!(glass_wobble_hold(Some(&mut sheet), -1.0, 0.0), GLASS_OK);
    assert_eq!(sheet.held, -1);
    let mut rested = 0;
    for _ in 0..600 {
        rested = glass_wobble_step(Some(&mut sheet), Some(&spec), 1.0 / 60.0);
    }
    assert_eq!(rested, 1);
    assert_eq!(
        glass_wobble_squeeze(Some(&mut sheet), 300.0, 200.0, 0.1),
        GLASS_OK
    );
    assert!(sheet.offset[0] > 0.0);
    assert_eq!(glass_wobble_step(None, Some(&spec), 0.1), GLASS_ERR_NULL);
}

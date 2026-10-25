//! The fragment shaders of the pipeline, as text with the model's constants
//! put in.
//!
//! The sources are plain GLSL ES 3.00 with no includes, so that
//! `tests/shaders.sh` can hand each rendered text to `glslangValidator`
//! without a display. All of them run under Hyprland's own vertex shader,
//! which gives `v_texcoord` in [0, 1] across the quad drawn, with (0, 0) at
//! its top left. Textures the pipeline makes for itself keep row 0 at the
//! top, as Hyprland's framebuffers and the clients' buffers do.
//!
//! A name between `@` signs in a source is a constant that lives in this
//! crate and is written in here when the text is made. `@LAYERED@` is a piece
//! of GLSL, `shaders/layered.glsl`: the frost and the face of the current
//! material, which the two shaders that draw a pane share.

use crate::lens::{TABLE_ROW_DEPTH, TABLE_ROW_LENS};
use crate::material::{G27, g27_hairline};
use crate::num::to_f32;

/// Rec. 709 luma weights: what "how bright" means to the face.
pub const LUMA: [f32; 3] = [0.2126, 0.7152, 0.0722];
/// The smallest span of alpha a coverage share is divided by.
pub const ALPHA_SPAN_FLOOR: f32 = 0.004;
/// The smallest fit the lens is given: a pane of no room still bends a little.
pub const FIT_FLOOR: f32 = 0.004;
/// The rim is not shaded where its edge factor is no more than this.
pub const EDGE_FLOOR: f32 = 0.004;

/// A fragment shader: the name it is looked up by, and its source.
const SOURCES: [(&str, &str); 17] = [
    ("coverage.frag", include_str!("../shaders/coverage.frag")),
    ("outline.frag", include_str!("../shaders/outline.frag")),
    ("fieldblur.frag", include_str!("../shaders/fieldblur.frag")),
    ("field.frag", include_str!("../shaders/field.frag")),
    ("cells.frag", include_str!("../shaders/cells.frag")),
    (
        "shadowblur.frag",
        include_str!("../shaders/shadowblur.frag"),
    ),
    ("reduce.frag", include_str!("../shaders/reduce.frag")),
    ("frostblur.frag", include_str!("../shaders/frostblur.frag")),
    ("shadow.frag", include_str!("../shaders/shadow.frag")),
    ("face.frag", include_str!("../shaders/face.frag")),
    ("meniscus.frag", include_str!("../shaders/meniscus.frag")),
    ("quartz27.frag", include_str!("../shaders/quartz27.frag")),
    (
        "quartzclean.frag",
        include_str!("../shaders/quartzclean.frag"),
    ),
    (
        "quartzseed.frag",
        include_str!("../shaders/quartzseed.frag"),
    ),
    (
        "quartzflood.frag",
        include_str!("../shaders/quartzflood.frag"),
    ),
    ("changes.frag", include_str!("../shaders/changes.frag")),
    ("mesh.frag", include_str!("../shaders/mesh.frag")),
];

/// The frost and the face of the current material, for the shaders that name it.
const LAYERED: &str = include_str!("../shaders/layered.glsl");

/// A number as GLSL writes a float: always with a point.
#[must_use]
pub fn glsl_float(x: f32) -> String {
    let text = x.to_string();
    if text.contains(['.', 'e', 'E']) {
        text
    } else {
        format!("{text}.0")
    }
}

/// Two numbers as a GLSL `vec2`.
fn glsl_vec2(pair: [f64; 2]) -> String {
    format!(
        "vec2({}, {})",
        glsl_float(to_f32(pair[0])),
        glsl_float(to_f32(pair[1]))
    )
}

/// The constants that sources name, with the text each stands for. The rim's
/// are the current material's (`crate::material::G27`), the same in both of
/// its appearances; its lengths are in thirds of a point, as measured.
fn constants() -> [(&'static str, String); 15] {
    let rim = G27.light.rim;
    let hairline = g27_hairline(true);
    [
        ("@LAYERED@", LAYERED.to_owned()),
        ("@RIM_OPPOSITE@", glsl_float(to_f32(rim.opposite))),
        ("@RIM_LUMA_POWER@", glsl_float(to_f32(rim.luma_power))),
        ("@RIM_CORE@", glsl_vec2(rim.core)),
        ("@RIM_TAIL@", glsl_vec2(rim.tail)),
        ("@RIM_INSET@", glsl_float(to_f32(rim.inset * 3.0))),
        ("@HAIRLINE_SHADE@", glsl_float(to_f32(hairline.shade))),
        (
            "@HAIRLINE_POWER@",
            glsl_float(to_f32(hairline.across_power)),
        ),
        ("@HAIRLINE_READ@", glsl_float(to_f32(hairline.read_offset))),
        (
            "@LUMA@",
            format!(
                "vec3({}, {}, {})",
                glsl_float(LUMA[0]),
                glsl_float(LUMA[1]),
                glsl_float(LUMA[2])
            ),
        ),
        ("@LENS_ROW_DEPTH@", glsl_float(TABLE_ROW_DEPTH)),
        ("@LENS_ROW_LEAN@", glsl_float(TABLE_ROW_LENS)),
        ("@ALPHA_SPAN_FLOOR@", glsl_float(ALPHA_SPAN_FLOOR)),
        ("@FIT_FLOOR@", glsl_float(FIT_FLOOR)),
        ("@EDGE_FLOOR@", glsl_float(EDGE_FLOOR)),
    ]
}

/// The names of the shaders, in a fixed order.
#[must_use]
pub fn names() -> Vec<&'static str> {
    SOURCES.iter().map(|(name, _)| *name).collect()
}

/// The text of the shader called `name`, with its constants put in.
#[must_use]
pub fn source(name: &str) -> Option<String> {
    let (_, template) = SOURCES.iter().find(|(n, _)| *n == name)?;
    Some(
        constants()
            .iter()
            .fold((*template).to_owned(), |text, (key, value)| {
                text.replace(key, value)
            }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floats_are_written_the_way_glsl_reads_them() {
        assert_eq!(glsl_float(0.25), "0.25");
        assert_eq!(glsl_float(1.0), "1.0");
        assert_eq!(glsl_float(0.2126), "0.2126");
        assert_eq!(glsl_float(0.004), "0.004");
    }

    #[test]
    fn every_shader_is_there_whole() {
        let names = names();
        assert_eq!(names.len(), 17);
        for name in names {
            let text = source(name).unwrap();
            assert!(text.starts_with("#version 300 es\n"), "{name}");
            assert!(
                text.contains("void main()") && text.contains("fragColor"),
                "{name}"
            );
            assert!(!text.contains('@'), "{name} still has a constant to put in");
        }
        assert!(source("nothing.frag").is_none());
    }

    #[test]
    fn the_constants_are_in_the_text() {
        let glass = source("meniscus.frag").unwrap();
        assert!(glass.contains("const vec3 LUMA = vec3(0.2126, 0.7152, 0.0722);"));
        assert!(glass.contains("vec2(sqrt(clamp(2.0 * (1.0 - v), 0.0, 1.0)), 0.25)"));
        assert!(glass.contains("/ tableTexels, 0.75)"));
        assert_eq!(glass.matches("0.004").count(), 4);
        assert!(
            source("frostblur.frag")
                .unwrap()
                .contains("const vec3 LUMA = vec3(0.2126, 0.7152, 0.0722);")
        );
    }

    #[test]
    fn the_current_materials_rim_is_written_into_the_glass_shader() {
        let glass = source("meniscus.frag").unwrap();
        // the lobes: the far one at 0.85, a core and a tail, starting 0.75 pt (2.25 thirds) in
        assert!(glass.contains("0.85 * pow(max(-along, 0.0), 3.0)"));
        assert!(glass.contains("vec2(0.175, 1.5).x * exp(-inset / vec2(0.175, 1.5).y)"));
        assert!(glass.contains("vec2(0.035, 8.0).x * exp(-inset / vec2(0.035, 8.0).y)"));
        assert!(glass.contains("inside - max(2.25 * third, rimWidth + 1.0)"));
        // the hairline: 80 of 255 darker, read two and a half thirds outside the pixel
        assert!(glass.contains("rimShade * 0.314 * mix(rimShadeEnds, 1.0, across)"));
        assert!(glass.contains("p - inward * (2.5 * third)"));
    }

    #[test]
    fn the_two_shaders_that_draw_a_pane_share_one_frost_and_one_face() {
        for name in ["meniscus.frag", "face.frag"] {
            let text = source(name).unwrap();
            assert_eq!(
                text.matches("vec3 layeredFrost(vec2 p, vec3 cloud)")
                    .count(),
                1,
                "{name}"
            );
            assert_eq!(text.matches("vec3 faceOf(vec3 color)").count(), 1, "{name}");
            assert_eq!(
                text.matches("uniform float saturation;").count(),
                1,
                "{name}"
            );
        }
        assert!(!source("frostblur.frag").unwrap().contains("layeredFrost"));
    }
}

//! The C header is generated from this crate's source and committed. This test
//! writes it again from the source and fails when the committed one differs;
//! `GLASS_UPDATE_HEADER=1 cargo test -p glass-ffi --test header` rewrites it.
//! It then has the C and C++ compilers read the header, which checks that the
//! structs have the sizes and alignments Rust gave them.

#![expect(
    clippy::unwrap_used,
    clippy::panic,
    reason = "this file is a test: a failed expectation is a failed test, which is what it is for"
)]

use std::fmt::Write as _;
use std::path::PathBuf;
use std::process::Command;

use glass_ffi::*;
use syn::{
    Expr, ExprLit, ExprUnary, FnArg, Item, ItemConst, ItemFn, ItemStruct, Lit, Pat, PathArguments,
    ReturnType, Type, UnOp,
};

const HEADER: &str = "include/glass_ffi.h";

fn path(file: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(file)
}

/// The doc comment of an item, one `//` line for each line of it.
fn doc(attrs: &[syn::Attribute], indent: &str) -> String {
    let mut out = String::new();
    for attr in attrs {
        if let syn::Meta::NameValue(nv) = &attr.meta {
            if nv.path.is_ident("doc") {
                if let Expr::Lit(ExprLit {
                    lit: Lit::Str(s), ..
                }) = &nv.value
                {
                    let line = s.value();
                    let _ = writeln!(out, "{indent}//{line}");
                }
            }
        }
    }
    out
}

fn primitive(name: &str) -> Option<&'static str> {
    Some(match name {
        "f32" => "float",
        "f64" => "double",
        "u8" => "uint8_t",
        "u16" => "uint16_t",
        "u32" => "uint32_t",
        "u64" => "uint64_t",
        "i8" => "int8_t",
        "i16" => "int16_t",
        "i32" => "int32_t",
        "i64" => "int64_t",
        "usize" => "size_t",
        "isize" => "ptrdiff_t",
        _ => return None,
    })
}

/// The C spelling of a type that is not an array.
fn c_type(ty: &Type) -> String {
    match ty {
        Type::Ptr(p) => {
            let inner = c_type(&p.elem);
            if p.mutability.is_some() {
                format!("{inner}*")
            } else {
                format!("const {inner}*")
            }
        }
        Type::Path(p) => {
            let segment = p.path.segments.last().unwrap();
            let name = segment.ident.to_string();
            if name == "Option" {
                let PathArguments::AngleBracketed(args) = &segment.arguments else {
                    panic!("Option without a type")
                };
                let Some(syn::GenericArgument::Type(Type::Reference(r))) = args.args.first() else {
                    panic!("Option of something that is not a reference")
                };
                let inner = c_type(&r.elem);
                return if r.mutability.is_some() {
                    format!("{inner}*")
                } else {
                    format!("const {inner}*")
                };
            }
            primitive(&name).map_or(name, str::to_owned)
        }
        _ => panic!("a type the header does not know"),
    }
}

/// `type name` or `type name[N]`.
fn declaration(ty: &Type, name: &str) -> String {
    if let Type::Array(a) = ty {
        let length = match &a.len {
            Expr::Path(p) => p.path.segments.last().unwrap().ident.to_string(),
            Expr::Lit(ExprLit {
                lit: Lit::Int(i), ..
            }) => i.base10_digits().to_owned(),
            _ => panic!("an array length the header does not know"),
        };
        return format!("{} {name}[{length}]", c_type(&a.elem));
    }
    format!("{} {name}", c_type(ty))
}

fn is_repr_c(attrs: &[syn::Attribute]) -> bool {
    attrs
        .iter()
        .any(|a| a.path().is_ident("repr") && a.parse_args::<syn::Ident>().is_ok_and(|i| i == "C"))
}

fn constant(item: &ItemConst) -> Option<String> {
    if item.ident == "_" {
        return None;
    }
    let value = match &*item.expr {
        Expr::Lit(ExprLit {
            lit: Lit::Int(i), ..
        }) => i.base10_digits().to_owned(),
        Expr::Unary(ExprUnary {
            op: UnOp::Neg(_),
            expr,
            ..
        }) => match &**expr {
            Expr::Lit(ExprLit {
                lit: Lit::Int(i), ..
            }) => format!("(-{})", i.base10_digits()),
            _ => panic!("a constant the header does not know"),
        },
        _ => panic!("a constant the header does not know"),
    };
    Some(format!(
        "{}#define {} {value}\n",
        doc(&item.attrs, ""),
        item.ident
    ))
}

fn structure(item: &ItemStruct) -> String {
    let mut out = doc(&item.attrs, "");
    let _ = writeln!(out, "typedef struct {} {{", item.ident);
    for field in &item.fields {
        let name = field.ident.as_ref().unwrap().to_string();
        out.push_str(&doc(&field.attrs, "    "));
        let _ = writeln!(out, "    {};", declaration(&field.ty, &name));
    }
    let _ = writeln!(out, "}} {};\n", item.ident);
    out
}

fn function(item: &ItemFn) -> String {
    let params: Vec<String> = item
        .sig
        .inputs
        .iter()
        .map(|arg| {
            let FnArg::Typed(typed) = arg else {
                panic!("a method")
            };
            let Pat::Ident(name) = &*typed.pat else {
                panic!("a pattern")
            };
            declaration(&typed.ty, &name.ident.to_string())
        })
        .collect();
    let ret = match &item.sig.output {
        ReturnType::Default => "void".to_owned(),
        ReturnType::Type(_, ty) => c_type(ty),
    };
    let args = if params.is_empty() {
        "void".to_owned()
    } else {
        params.join(", ")
    };
    format!(
        "{}{ret} {}({args});\n\n",
        doc(&item.attrs, ""),
        item.sig.ident
    )
}

/// Size and alignment of every `#[repr(C)]` struct, from the types themselves.
fn layouts() -> Vec<(&'static str, usize, usize)> {
    macro_rules! layout {
        ($($t:ident),* $(,)?) => { vec![$((stringify!($t), size_of::<$t>(), align_of::<$t>())),*] };
    }
    layout![
        GlassRect,
        GlassFrame,
        GlassLens,
        GlassLayered,
        GlassRimConstants,
        GlassParams,
        GlassValues,
        GlassLevels,
        GlassBytes,
        GlassStore,
        GlassText,
        GlassResolveRequest,
        GlassFieldEdit,
        GlassOptionInfo,
        GlassThemeDefaults,
        GlassTonePeak,
        GlassChanges,
        GlassLayout,
        GlassFrostPlan,
        GlassSampleQuery,
        GlassReaches,
        GlassDrawConstants,
        GlassOpacities,
        GlassQuartz,
        GlassTuning,
        GlassKnob,
        GlassTuningPreset,
        GlassPane,
        GlassPaneList,
        GlassPaneBuffers,
        GlassMapRequest,
        GlassWantedRequest,
        GlassSpring,
        GlassStretch,
        GlassMouth,
        GlassMesh,
        GlassWobbleSpec,
        GlassWobble,
        GlassDeclaredRect,
        GlassSample,
    ]
}

fn generate() -> String {
    let mut constants = String::new();
    let mut structs = String::new();
    let mut functions = String::new();
    let mut struct_names = Vec::new();
    for file in ["src/types.rs", "src/exports.rs"] {
        let source = std::fs::read_to_string(path(file)).unwrap();
        for item in syn::parse_file(&source).unwrap().items {
            match item {
                Item::Const(c) => constants.push_str(&constant(&c).unwrap_or_default()),
                Item::Struct(s) if is_repr_c(&s.attrs) => {
                    struct_names.push(s.ident.to_string());
                    structs.push_str(&structure(&s));
                }
                Item::Fn(f)
                    if f.sig.abi.is_some() && matches!(f.vis, syn::Visibility::Public(_)) =>
                {
                    assert!(
                        f.attrs.iter().any(|a| a.path().is_ident("unsafe")),
                        "{} is not exported with no_mangle",
                        f.sig.ident
                    );
                    functions.push_str(&function(&f));
                }
                _ => {}
            }
        }
    }
    let mut known: Vec<&str> = layouts().iter().map(|l| l.0).collect();
    known.sort_unstable();
    struct_names.sort_unstable();
    assert_eq!(
        known, struct_names,
        "every repr(C) struct needs a line in layouts()"
    );

    let mut asserts = String::new();
    for (name, size, align) in layouts() {
        let _ = writeln!(
            asserts,
            "GLASS_STATIC_ASSERT(sizeof({name}) == {size}, \"{name} has a different size in Rust\");"
        );
        let _ = writeln!(
            asserts,
            "GLASS_STATIC_ASSERT(alignof_({name}) == {align}, \"{name} has a different alignment in Rust\");"
        );
    }
    format!(
        "/* glass_ffi.h: the C interface of the glass model (crates/glass-ffi).\n\
         *\n\
         * Generated from the Rust source by `cargo test -p glass-ffi --test header`\n\
         * (GLASS_UPDATE_HEADER=1 writes it again). Do not edit it by hand: that test\n\
         * fails while this file and the source disagree.\n\
         *\n\
         * Every function answers with an int32_t: zero or more for success, a\n\
         * negative GLASS_ERR_* for failure. Pointers are lent for the length of the\n\
         * call and never kept; results are written to memory the caller owns.\n\
         */\n\
         #ifndef GLASS_FFI_H\n#define GLASS_FFI_H\n\n\
         #include <stddef.h>\n#include <stdint.h>\n\n\
         #ifdef __cplusplus\nextern \"C\" {{\n#endif\n\n\
         {constants}\n{structs}{functions}\
         #ifdef __cplusplus\n#define GLASS_STATIC_ASSERT(condition, message) static_assert(condition, message)\n#define alignof_(type) alignof(type)\n\
         #else\n#define GLASS_STATIC_ASSERT(condition, message) _Static_assert(condition, message)\n#define alignof_(type) _Alignof(type)\n#endif\n\n\
         {asserts}\n#ifdef __cplusplus\n}}\n#endif\n\n#endif /* GLASS_FFI_H */\n"
    )
}

#[test]
fn the_option_constants_are_the_indexes_of_the_options_they_name() {
    use glass_core::options::{FloatOption, IntOption};
    let source = std::fs::read_to_string(path("src/types.rs")).unwrap();
    let (mut floats, mut ints) = (0, 0);
    for item in syn::parse_file(&source).unwrap().items {
        let Item::Const(c) = item else { continue };
        let name = c.ident.to_string();
        let Expr::Lit(ExprLit {
            lit: Lit::Int(value),
            ..
        }) = &*c.expr
        else {
            continue;
        };
        let index: usize = value.base10_parse().unwrap();
        if let Some(key) = name.strip_prefix("GLASS_F_") {
            assert_eq!(
                FloatOption::from_key(&key.to_lowercase()).map(FloatOption::index),
                Some(index),
                "{name}"
            );
            floats += 1;
        } else if let Some(key) = name.strip_prefix("GLASS_I_") {
            assert_eq!(
                IntOption::from_key(&key.to_lowercase()).map(IntOption::index),
                Some(index),
                "{name}"
            );
            ints += 1;
        }
    }
    assert_eq!(
        (floats, ints),
        (FloatOption::ALL.len(), IntOption::ALL.len()),
        "every option has its constant"
    );
}

#[test]
fn the_committed_header_is_the_one_the_source_makes() {
    let expected = generate();
    if std::env::var_os("GLASS_UPDATE_HEADER").is_some() {
        std::fs::write(path(HEADER), &expected).unwrap();
    }
    let committed = std::fs::read_to_string(path(HEADER)).unwrap_or_default();
    assert!(
        committed == expected,
        "{HEADER} is stale: run `GLASS_UPDATE_HEADER=1 cargo test -p glass-ffi --test header`"
    );
}

#[test]
fn the_c_and_c_plus_plus_compilers_agree_with_rust_about_every_layout() {
    let dir = std::env::temp_dir().join(format!("glass-header-check-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let include = path("include");
    for (compiler, language, standard) in [("cc", "c", "-std=c11"), ("c++", "c++", "-std=c++23")] {
        let source = dir.join(format!("check.{language}"));
        std::fs::write(&source, "#include \"glass_ffi.h\"\nint main(void) { return glass_abi_version() == GLASS_ABI_VERSION ? 0 : 1; }\n").unwrap();
        let outcome = Command::new(compiler)
            .args([
                standard,
                "-Wall",
                "-Wextra",
                "-Werror",
                "-fsyntax-only",
                "-x",
                language,
            ])
            .arg(format!("-I{}", include.display()))
            .arg(&source)
            .output();
        match outcome {
            Ok(done) => assert!(
                done.status.success(),
                "{compiler} rejects the header:\n{}",
                String::from_utf8_lossy(&done.stderr)
            ),
            Err(error) => eprintln!(
                "{compiler} is not available ({error}): its check of the header is skipped"
            ),
        }
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

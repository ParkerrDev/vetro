# HyprGlass Plugin

CXX ?= g++
CC ?= gcc
WAYLAND_SCANNER ?= wayland-scanner
CXXFLAGS = -fPIC -g -O2 -std=c++23
# Never ask for an executable stack, whatever an object or the linker defaults to:
# glibc 2.41+ refuses to load such a library.
LDFLAGS = -shared -Wl,-z,noexecstack
INCLUDES = $(shell pkg-config --cflags hyprland pixman-1 libdrm) -Icrates/glass-ffi/include
LIBS = $(shell pkg-config --libs hyprland) -ldl

# Shown by `hyprctl plugin list`. Release builds pass HYPRGLASS_VERSION; other
# builds describe their own checkout (never an enclosing repository), keeping
# only characters safe in a C string. Changing it needs `make clean`.
ifndef HYPRGLASS_VERSION
HYPRGLASS_VERSION := $(shell [ -e .git ] && git describe --tags --dirty 2>/dev/null | sed 's/^v//' | tr -cd 'A-Za-z0-9._+-')
endif
ifeq ($(HYPRGLASS_VERSION),)
HYPRGLASS_VERSION := dev
endif
CXXFLAGS += -DHYPRGLASS_VERSION='"$(HYPRGLASS_VERSION)"'

# Keeps two loaded copies from sharing their globals. GCC only, whatever CXX calls it: clang has no such flag.
ifneq ($(shell $(CXX) --version 2>/dev/null | grep -c 'Free Software Foundation'),0)
	CXXFLAGS += --no-gnu-unique
endif

TARGET = hyprglass.so
SOURCES = src/main.cpp src/LoadGuard.cpp src/GlassDecoration.cpp src/GlassPassElement.cpp src/GlassRenderer.cpp src/GlassLayerSurface.cpp src/GlassLayerModel.cpp src/Meniscus.cpp src/GlassLayerPassElement.cpp src/GlassLayerCompositeElement.cpp src/BackgroundDamageObserver.cpp src/PluginConfig.cpp src/GlassBridge.cpp src/PresetResolve.cpp src/ShaderManager.cpp src/Diagnostics.cpp src/GlassSubsurfaceState.cpp src/GlassSubsurfacePassElement.cpp src/GlassSubsurfaceCompositeElement.cpp src/ItemHints.cpp src/Backdrop.cpp src/Tuning.cpp src/Beneath.cpp src/Motion.cpp
OBJ = $(SOURCES:.cpp=.o)
HEADERS = $(wildcard src/*.hpp) crates/glass-ffi/include/glass_ffi.h

# ── The glass model: Rust, behind a C interface ──────────────────────────────
# crates/glass-core is the model (pure computation, no unsafe code) and
# crates/glass-ffi is its C interface, built as a static library and linked into
# the plugin. Every decision about how the glass looks is made there; the C++
# keeps Hyprland's side: hooks, pass elements, framebuffers, GL and damage.
# Built with the locked dependencies (Cargo.lock) in the release profile, whose
# panic = "abort" means nothing can unwind into the compositor.
CARGO ?= cargo
GLASS_LIB = target/release/libglass_ffi.a
RUST_SOURCES = Cargo.toml Cargo.lock $(shell find crates -type f \( -name '*.rs' -o -name '*.frag' -o -name Cargo.toml \))

# ── hyprglass_item_v1 protocol helper ────────────────────────────────────────
# Built as its own shared library and embedded (via ItemHelperBlob.S) inside
# hyprglass.so, so it can be dlopen'd with RTLD_NODELETE and outlive a plugin
# reload — see AGENTS.md's unload-safety note. Depends only on
# libwayland-server, never on Hyprland headers.

PROTOCOL_XML    = protocols/hyprglass-item-v1.xml
ITEM_HELPER_DIR = src/item-helper
PROTOCOL_HEADER = $(ITEM_HELPER_DIR)/hyprglass-item-v1-server-protocol.h
PROTOCOL_CODE   = $(ITEM_HELPER_DIR)/hyprglass-item-v1-protocol.c

HELPER_SO       = $(ITEM_HELPER_DIR)/hyprglass-item-helper.so
HELPER_SOURCES  = $(ITEM_HELPER_DIR)/helper.c $(PROTOCOL_CODE)
HELPER_OBJ      = $(HELPER_SOURCES:.c=.o)

WAYLAND_SERVER_CFLAGS = $(shell pkg-config --cflags wayland-server)
WAYLAND_SERVER_LIBS   = $(shell pkg-config --libs wayland-server)

ITEM_HELPER_BLOB_OBJ = src/ItemHelperBlob.o

all: $(TARGET)

%.o : %.cpp $(HEADERS)
	@echo "[$(CXX)] $<"
	@$(CXX) -c $(CXXFLAGS) $(INCLUDES) $< -o $@

$(PROTOCOL_HEADER): $(PROTOCOL_XML)
	@echo "[wayland-scanner] server-header $<"
	@$(WAYLAND_SCANNER) server-header $< $@

$(PROTOCOL_CODE): $(PROTOCOL_XML)
	@echo "[wayland-scanner] private-code $<"
	@$(WAYLAND_SCANNER) private-code $< $@

# helper.c includes the generated server header; every helper object depends
# on it existing first, not just the .c file the pattern rule already covers.
$(HELPER_OBJ): $(PROTOCOL_HEADER)

$(ITEM_HELPER_DIR)/%.o: $(ITEM_HELPER_DIR)/%.c
	@echo "[$(CC)] $<"
	@$(CC) -c -fPIC -O2 -std=c11 $(WAYLAND_SERVER_CFLAGS) -I$(ITEM_HELPER_DIR) $< -o $@

$(HELPER_SO): $(HELPER_OBJ)
	@echo "Linking $(HELPER_SO)..."
	@$(CC) -shared -fPIC -Wl,-z,noexecstack -o $@ $(HELPER_OBJ) $(WAYLAND_SERVER_LIBS)

# .incbin's path is relative to this Makefile's own working directory (the
# repo root), the same base every other SOURCES path here is written against.
$(ITEM_HELPER_BLOB_OBJ): src/ItemHelperBlob.S $(HELPER_SO)
	@echo "[as] $<"
	@$(CC) -c $< -o $@

$(GLASS_LIB): $(RUST_SOURCES)
	@echo "[cargo] glass-ffi"
	@$(CARGO) build --release --locked -p glass-ffi

# --exclude-libs keeps the library's symbols out of the plugin's own table: the
# plugin exports its entry points and nothing the model is made of.
$(TARGET): $(OBJ) $(ITEM_HELPER_BLOB_OBJ) $(GLASS_LIB)
	@echo "Linking $(TARGET)..."
	@$(CXX) $(LDFLAGS) $(OBJ) $(ITEM_HELPER_BLOB_OBJ) $(GLASS_LIB) -Wl,--exclude-libs,ALL -o $@ $(LIBS)
	@echo "Done!"

# Tests that need no display: the model's own (lens, options, panes, schedule,
# shaders, motion, shapes, materials: cargo test), the C header against the
# Rust source and against the C and C++ compilers' idea of every layout, and a
# compile of every fragment shader (skipped with a note when glslangValidator is
# not installed).
TEST_DIR = build/tests

test: $(GLASS_LIB)
	@$(CARGO) test --workspace --locked
	@$(CARGO) test --locked -p glass-ffi --test header
	@mkdir -p $(TEST_DIR)
	@CXX="$(CXX)" GLASS_LIB="$(CURDIR)/$(GLASS_LIB)" sh tests/shaders.sh

# Everything a change has to pass: formatting, clippy with every warning an
# error, the tests, the documentation, and the dependency policy.
check: test
	@$(CARGO) fmt --all --check
	@$(CARGO) clippy --workspace --all-targets --locked -- -D warnings
	@RUSTDOCFLAGS="-D warnings" $(CARGO) doc --no-deps --workspace --locked --quiet
	@$(CARGO) deny check

clean:
	rm -f $(OBJ) $(TARGET) $(ITEM_HELPER_BLOB_OBJ) $(HELPER_OBJ) $(HELPER_SO) $(PROTOCOL_HEADER) $(PROTOCOL_CODE)
	rm -rf $(TEST_DIR) target

.PHONY: all clean test check

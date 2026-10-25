# Vetro Glass, the window where the liquid glass is tuned (`vetro tune`).
#
#   make            build vetro-glass here
#   make install    put it in ~/.local/bin and its desktop entry in
#                   ~/.local/share/applications (both links into this folder)
#
# It links the engine's own library (engine/crates/glass-ffi), which says what
# every knob is: building it needs cargo, as the engine does.

ENGINE    := ../../engine
GLASS_LIB := $(ENGINE)/target/release/libglass_ffi.a
CC        ?= cc
PKGS      := gtk4 libadwaita-1
CFLAGS    += -std=c17 -O2 -g -Wall -Wextra -Wpedantic -Wshadow -Wformat=2 -Werror \
             -I$(ENGINE)/crates/glass-ffi/include $(shell pkg-config --cflags $(PKGS))
LDLIBS    += $(shell pkg-config --libs $(PKGS)) -lpthread -ldl -lm
SOURCES   := src/main.c src/model.c src/hypr.c
HEADERS   := src/model.h src/hypr.h $(ENGINE)/crates/glass-ffi/include/glass_ffi.h
BRANDING  := ../../assets/branding
RESOURCE_FILES := resources/vetro.gresource.xml resources/style.css $(BRANDING)/vetro.svg $(BRANDING)/vetro-symbolic.svg
RESOURCE_OBJECT := .build/vetro-resources.o
PREFIX    ?= $(HOME)/.local

all: vetro-glass

vetro-glass: $(SOURCES) $(HEADERS) $(GLASS_LIB) $(RESOURCE_OBJECT) Makefile
	$(CC) $(CFLAGS) $(SOURCES) $(RESOURCE_OBJECT) $(GLASS_LIB) $(LDLIBS) -o $@

.build/vetro-resources.c: $(RESOURCE_FILES)
	mkdir -p .build
	glib-compile-resources resources/vetro.gresource.xml --sourcedir=resources --sourcedir=$(BRANDING) --generate-source --target=$@

$(RESOURCE_OBJECT): .build/vetro-resources.c
	$(CC) $(CFLAGS) -Wno-overlength-strings -c $< -o $@

# The engine's Makefile knows how its library is built.
$(GLASS_LIB): $(shell find $(ENGINE)/crates -type f \( -name '*.rs' -o -name '*.frag' -o -name Cargo.toml \))
	$(MAKE) -C $(ENGINE) target/release/libglass_ffi.a

install: vetro-glass
	mkdir -p $(PREFIX)/bin $(PREFIX)/share/applications $(PREFIX)/share/icons/hicolor/scalable/apps $(PREFIX)/share/icons/hicolor/symbolic/apps
	ln -sfn $(CURDIR)/vetro-glass $(PREFIX)/bin/vetro-glass
	ln -sfn $(CURDIR)/vetro-glass.desktop $(PREFIX)/share/applications/dev.vetro.Glass.desktop
	ln -sfn $(abspath $(BRANDING))/vetro.svg $(PREFIX)/share/icons/hicolor/scalable/apps/dev.vetro.Glass.svg
	ln -sfn $(abspath $(BRANDING))/vetro-symbolic.svg $(PREFIX)/share/icons/hicolor/symbolic/apps/dev.vetro.Glass-symbolic.svg

clean:
	rm -f vetro-glass
	rm -rf .build

.PHONY: all install clean

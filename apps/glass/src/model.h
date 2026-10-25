/* model.h: the glass as the user tunes it, its file, and the way back.
 *
 * The values, their ranges and the presets are the engine's own
 * (glass_core::tuning, through libglass_ffi): the app and the compositor can
 * never disagree about what a knob means. */
#ifndef VETRO_GLASS_MODEL_H
#define VETRO_GLASS_MODEL_H

#include "glass_ffi.h"

#include <limits.h>
#include <stdbool.h>
#include <stddef.h>

/* Steps kept to undo, and to redo: the oldest goes when there are more. */
#define MODEL_HISTORY 64

typedef struct {
    GlassTuning current; /* what the sliders show and the glass is drawn with */
    GlassTuning applied; /* what the file holds: the glass at the next login */
    GlassTuning undo[MODEL_HISTORY];
    unsigned undo_count;
    GlassTuning redo[MODEL_HISTORY];
    unsigned redo_count;
    char path[PATH_MAX]; /* ~/.config/vetro/glass.conf */
} Model;

/* Finds the file and reads it, or takes the defaults where there is none. */
bool model_init(Model *model);

/* Writes the current tuning to the file, in one step (a reader never sees half
 * of it), and takes it as applied. */
bool model_save(Model *model);

/* Whether what is shown differs from what is applied. */
bool model_dirty(const Model *model);

/* Keeps the current tuning as a step to come back to, before a change, and
 * forgets what could be redone. */
void model_checkpoint(Model *model);

/* Back one step, or forward again; false when there is none. */
bool model_undo(Model *model);
bool model_redo(Model *model);

/* Sets knob `index`, held to its range by the engine's rules. */
void model_set(Model *model, unsigned index, float value);

/* Takes the preset named `name` whole. */
bool model_preset(Model *model, const char *name);

/* The request that has the running glass draw the current tuning without
 * writing the file: "hyprglass tune key=value ...". False if it does not fit. */
bool model_preview_request(const Model *model, char *out, size_t cap);

#endif

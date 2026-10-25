/* model.c: the glass as the user tunes it, its file, and the way back. */
#include "model.h"

#include <assert.h>
#include <gio/gio.h>
#include <stdio.h>
#include <string.h>

/* A tuning file is a few dozen short lines; anything larger is not one. */
#define MAX_FILE (64 * 1024)

static bool find_path(Model *model) {
    const char *config = g_get_user_config_dir();
    const int written = snprintf(model->path, sizeof(model->path), "%s/vetro/glass.conf", config);
    return written > 0 && (size_t)written < sizeof(model->path);
}

/* The file's text over the defaults; the defaults alone when there is no file. */
static void read_file(Model *model) {
    (void)glass_tuning_default(&model->current);
    gchar *text = NULL;
    gsize length = 0;
    if (!g_file_get_contents(model->path, &text, &length, NULL))
        return;
    if (length <= MAX_FILE) {
        const GlassBytes bytes = {.data = (const uint8_t *)text, .len = length};
        if (glass_tuning_parse(bytes, &model->current) < 0)
            (void)glass_tuning_default(&model->current);
    }
    g_free(text);
}

bool model_init(Model *model) {
    assert(model != NULL);
    memset(model, 0, sizeof(*model));
    if (!find_path(model))
        return false;
    read_file(model);
    model->applied = model->current;
    return true;
}

/* The current tuning as the file's text, NUL-terminated; g_free it. */
static char *format_text(const Model *model, size_t *length) {
    *length = 0;
    if (glass_tuning_format(&model->current, NULL, 0, length) < 0 || *length == 0 || *length > MAX_FILE)
        return NULL;
    char *text = g_malloc(*length + 1);
    if (glass_tuning_format(&model->current, (uint8_t *)text, *length, length) < 0) {
        g_free(text);
        return NULL;
    }
    text[*length] = '\0';
    return text;
}

bool model_save(Model *model) {
    assert(model != NULL);
    size_t length = 0;
    char *text = format_text(model, &length);
    gchar *folder = g_path_get_dirname(model->path);
    bool ok = text != NULL && g_mkdir_with_parents(folder, 0755) == 0;
    /* g_file_set_contents writes a sibling and renames it over the file */
    ok = ok && g_file_set_contents(model->path, text, (gssize)length, NULL);
    g_free(folder);
    g_free(text);
    if (ok)
        model->applied = model->current;
    return ok;
}

bool model_dirty(const Model *model) {
    assert(model != NULL);
    for (unsigned i = 0; i < GLASS_TUNING_KNOBS; ++i) {
        if (model->current.values[i] != model->applied.values[i])
            return true;
    }
    return false;
}

/* Puts `value` on a stack of MODEL_HISTORY, dropping the oldest when it is full. */
static void push(GlassTuning *stack, unsigned *count, const GlassTuning *value) {
    if (*count >= MODEL_HISTORY) {
        memmove(stack, stack + 1, sizeof(stack[0]) * (MODEL_HISTORY - 1));
        *count = MODEL_HISTORY - 1;
    }
    stack[*count] = *value;
    *count += 1;
}

void model_checkpoint(Model *model) {
    assert(model != NULL);
    push(model->undo, &model->undo_count, &model->current);
    model->redo_count = 0;
}

bool model_undo(Model *model) {
    assert(model != NULL);
    if (model->undo_count == 0)
        return false;
    push(model->redo, &model->redo_count, &model->current);
    model->undo_count -= 1;
    model->current = model->undo[model->undo_count];
    return true;
}

bool model_redo(Model *model) {
    assert(model != NULL);
    if (model->redo_count == 0)
        return false;
    push(model->undo, &model->undo_count, &model->current);
    model->redo_count -= 1;
    model->current = model->redo[model->redo_count];
    return true;
}

void model_set(Model *model, unsigned index, float value) {
    assert(model != NULL);
    (void)glass_tuning_set(&model->current, index, value);
}

bool model_preset(Model *model, const char *name) {
    assert(model != NULL && name != NULL);
    const GlassBytes bytes = {.data = (const uint8_t *)name, .len = strlen(name)};
    return glass_tuning_preset(bytes, &model->current) >= 0;
}

/* Adds `piece` to `out`, which holds `*used` bytes; false if it would not fit. */
static bool append(char *out, size_t cap, size_t *used, const char *piece) {
    const size_t length = strlen(piece);
    if (*used + length + 1 > cap)
        return false;
    memcpy(out + *used, piece, length);
    *used += length;
    out[*used] = '\0';
    return true;
}

bool model_preview_request(const Model *model, char *out, size_t cap) {
    assert(model != NULL && out != NULL && cap > 0);
    out[0] = '\0';
    size_t length = 0;
    char *text = format_text(model, &length);
    if (text == NULL)
        return false;
    size_t used = 0;
    bool ok = append(out, cap, &used, "hyprglass tune");
    /* The file's lines are `key = value`; the request takes them as key=value. */
    gchar **lines = g_strsplit(text, "\n", -1);
    for (gchar **line = lines; ok && *line != NULL; ++line) {
        gchar *equals = strchr(*line, '=');
        if ((*line)[0] == '#' || equals == NULL)
            continue;
        *equals = '\0';
        ok = append(out, cap, &used, " ") && append(out, cap, &used, g_strstrip(*line)) &&
             append(out, cap, &used, "=") && append(out, cap, &used, g_strstrip(equals + 1));
    }
    g_strfreev(lines);
    g_free(text);
    return ok;
}

/* main.c: Vetro Glass, where the liquid glass is tuned by hand.
 *
 * Every slider is one of the engine's knobs (glass_core::tuning): its name,
 * words, range and default come from the engine's own library, so this window
 * can never offer a value the glass does not understand. A change is shown at
 * once, at most thirty times a second while a slider moves: the engine is told
 * the values (`hyprglass tune key=value ...`) and the glass under this very
 * window changes with it. Apply writes them to ~/.config/vetro/glass.conf, the
 * glass at every login; until then Undo and Redo step through the changes,
 * Revert goes back to what is applied, and closing asks which to keep. */
#include "hypr.h"
#include "model.h"

#include <adwaita.h>
#include <math.h>
#include <stdio.h>
#include <string.h>

#define APP_ID "dev.vetro.Glass"
#define FLUSH_MS 33
#define TEXT_MAX (GLASS_TEXT_BYTES + 1)
#define GROUPS 5
#define LOOKS 5
#define REQUEST_MAX 4096
/* Changes to one knob closer together than this are one step to undo: a
 * slider dragged is one step, not a hundred. */
#define BURST_US (700 * 1000)
#define NO_KNOB (-1)

typedef struct {
    Model model;
    GlassKnob knobs[GLASS_TUNING_KNOBS];
    GtkWidget *control[GLASS_TUNING_KNOBS]; /* GtkScale, AdwSwitchRow or GtkColorDialogButton */
    GtkWidget *value[GLASS_TUNING_KNOBS];   /* a slider's value, in words */
    GtkWindow *window;
    AdwBanner *banner;
    AdwToastOverlay *toasts;
    AdwComboRow *look;
    GSimpleAction *undo, *redo, *apply, *revert;
    guint flush;
    int burst_knob;  /* the knob the last change was to, or NO_KNOB */
    gint64 burst_at; /* and when, in g_get_monotonic_time's microseconds */
    bool updating;   /* the widgets are being set from the model: their signals change nothing */
} App;

static const char *const GROUP_TITLE[GROUPS] = {"Glass", "Edge", "Light", "Behind the glass", "Motion"};
static const char *const GROUP_TEXT[GROUPS] = {
    "The body of the glass: how clear or tinted it is, how frosted, and what it does with colour.",
    "Where the glass ends: the lens that bends what is behind it, and the edge iOS 27 darkens.",
    "The light the glass catches: iOS 27 keys it from straight above and keeps it still.",
    "What shows through.",
    "How things move: the pace of every animation, the panels' too, the genie, and windows that land like jelly and wobble as they are dragged. A preset leaves these as they are.",
};
/* `vetro glass look` names, and what this window calls them */
static const char *const LOOK_NAME[LOOKS] = {"apple", "lens", "macos", "liquid", "regular"};
static const char *const LOOK_LABEL[LOOKS + 1] = {
    "Tuned, with these sliders",
    "Lens (18 October, evening)",
    "macOS 27 recipe (18 October, afternoon)",
    "Liquid (15 to 18 October)",
    "Regular (12 to 15 October)",
};

static void text_of(const GlassText *text, char *out, size_t cap) {
    const size_t length = text->len < cap - 1 ? text->len : cap - 1;
    memcpy(out, text->bytes, length);
    out[length] = '\0';
}

/* A slider's value in its unit: a share as a percentage, points, a factor,
 * degrees, seconds. */
static void format_value(const GlassKnob *knob, double value, char *out, size_t cap) {
    char unit[TEXT_MAX];
    text_of(&knob->unit, unit, sizeof(unit));
    if (strcmp(unit, "%") == 0)
        snprintf(out, cap, "%d %%", (int)lround(value * 100.0));
    else if (strcmp(unit, "pt") == 0)
        snprintf(out, cap, "%.1f pt", value);
    else if (strcmp(unit, "x") == 0)
        snprintf(out, cap, "%.2f×", value);
    else if (strcmp(unit, "deg") == 0)
        snprintf(out, cap, "%d°", (int)lround(value));
    else if (strcmp(unit, "s") == 0)
        snprintf(out, cap, "%.2f s", value);
    else
        snprintf(out, cap, "%.2f", value);
}

static void show_value(App *app, unsigned index) {
    if (app->value[index] == NULL)
        return;
    char text[64];
    format_value(&app->knobs[index], app->model.current.values[index], text, sizeof(text));
    gtk_label_set_text(GTK_LABEL(app->value[index]), text);
}

static void toast(App *app, const char *message) {
    adw_toast_overlay_add_toast(app->toasts, adw_toast_new(message));
}

/* Sends a request to the engine. The engine that runs now may be one from
 * before tuning existed: then the banner says what to do. */
static void tell_engine(App *app, const char *request) {
    char reply[512];
    const bool heard = hypr_request(request, reply, sizeof(reply));
    adw_banner_set_revealed(app->banner, !heard || strncmp(reply, "hyprglass tune:", 15) != 0);
}

/* Has the engine draw the current values, without writing the file. */
static gboolean flush(gpointer data) {
    App *app = data;
    app->flush = 0;
    char request[REQUEST_MAX];
    if (model_preview_request(&app->model, request, sizeof(request)))
        tell_engine(app, request);
    return G_SOURCE_REMOVE;
}

static void schedule(App *app) {
    if (app->flush == 0)
        app->flush = g_timeout_add(FLUSH_MS, flush, app);
}

/* The buttons to the model: what can be undone, redone, applied or reverted. */
static void update_state(App *app) {
    const bool dirty = model_dirty(&app->model);
    g_simple_action_set_enabled(app->undo, app->model.undo_count > 0);
    g_simple_action_set_enabled(app->redo, app->model.redo_count > 0);
    g_simple_action_set_enabled(app->apply, dirty);
    g_simple_action_set_enabled(app->revert, dirty);
}

/* Before a change: a step to undo to, unless it carries on the drag of the
 * same knob. NO_KNOB (a preset, a switch, a reset) is a step of its own. */
static void begin_change(App *app, int knob) {
    const gint64 now = g_get_monotonic_time();
    const bool same = knob != NO_KNOB && knob == app->burst_knob && now - app->burst_at < BURST_US;
    if (!same)
        model_checkpoint(&app->model);
    app->burst_knob = knob;
    app->burst_at = now;
}

/* After a change: shown on the glass, and the buttons brought up to date. */
static void changed(App *app) {
    schedule(app);
    update_state(app);
}

static void on_scale(GtkRange *range, gpointer data) {
    App *app = data;
    if (app->updating)
        return;
    const unsigned index = GPOINTER_TO_UINT(g_object_get_data(G_OBJECT(range), "knob"));
    begin_change(app, (int)index);
    model_set(&app->model, index, (float)gtk_range_get_value(range));
    show_value(app, index);
    changed(app);
}

static void on_switch(GObject *row, GParamSpec *spec, gpointer data) {
    (void)spec;
    App *app = data;
    if (app->updating)
        return;
    const unsigned index = GPOINTER_TO_UINT(g_object_get_data(row, "knob"));
    begin_change(app, NO_KNOB);
    model_set(&app->model, index, adw_switch_row_get_active(ADW_SWITCH_ROW(row)) ? 1.0f : 0.0f);
    changed(app);
}

static void on_colour(GObject *button, GParamSpec *spec, gpointer data) {
    (void)spec;
    App *app = data;
    if (app->updating)
        return;
    const unsigned index = GPOINTER_TO_UINT(g_object_get_data(button, "knob"));
    const GdkRGBA *rgba = gtk_color_dialog_button_get_rgba(GTK_COLOR_DIALOG_BUTTON(button));
    begin_change(app, NO_KNOB);
    model_set(&app->model, index, rgba->red);
    model_set(&app->model, index + 1, rgba->green);
    model_set(&app->model, index + 2, rgba->blue);
    changed(app);
}

/* Every widget to the model's values, without their signals taking them back. */
static void refresh(App *app) {
    app->updating = true;
    const float *values = app->model.current.values;
    for (unsigned i = 0; i < GLASS_TUNING_KNOBS; ++i) {
        GtkWidget *control = app->control[i];
        if (control == NULL)
            continue;
        if (GTK_IS_RANGE(control))
            gtk_range_set_value(GTK_RANGE(control), values[i]);
        else if (ADW_IS_SWITCH_ROW(control))
            adw_switch_row_set_active(ADW_SWITCH_ROW(control), values[i] >= 0.5f);
        else if (GTK_IS_COLOR_DIALOG_BUTTON(control) && i + 2 < GLASS_TUNING_KNOBS) {
            const GdkRGBA rgba = {values[i], values[i + 1], values[i + 2], 1.0f};
            gtk_color_dialog_button_set_rgba(GTK_COLOR_DIALOG_BUTTON(control), &rgba);
        }
        show_value(app, i);
    }
    app->updating = false;
}

/* A slider row: its name and value above, a line on what it does, and the
 * slider the width of the row, as iOS lays out its own. */
static GtkWidget *slider_row(App *app, unsigned index) {
    const GlassKnob *knob = &app->knobs[index];
    char label[TEXT_MAX], help[TEXT_MAX], key[TEXT_MAX];
    text_of(&knob->label, label, sizeof(label));
    text_of(&knob->help, help, sizeof(help));
    text_of(&knob->key, key, sizeof(key));

    GtkWidget *title = gtk_label_new(label);
    gtk_label_set_xalign(GTK_LABEL(title), 0.0f);
    gtk_widget_set_hexpand(title, TRUE);
    GtkWidget *value = gtk_label_new("");
    gtk_widget_add_css_class(value, "numeric");
    gtk_widget_add_css_class(value, "dim-label");
    GtkWidget *top = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 12);
    gtk_box_append(GTK_BOX(top), title);
    gtk_box_append(GTK_BOX(top), value);

    GtkWidget *words = gtk_label_new(help);
    gtk_label_set_xalign(GTK_LABEL(words), 0.0f);
    gtk_label_set_wrap(GTK_LABEL(words), TRUE);
    gtk_widget_add_css_class(words, "dim-label");
    gtk_widget_add_css_class(words, "caption");

    GtkWidget *scale = gtk_scale_new_with_range(GTK_ORIENTATION_HORIZONTAL, knob->min, knob->max, knob->step);
    gtk_scale_set_draw_value(GTK_SCALE(scale), FALSE);
    gtk_range_set_value(GTK_RANGE(scale), app->model.current.values[index]);
    if (strcmp(key, "tint") == 0) {
        gtk_scale_add_mark(GTK_SCALE(scale), knob->min, GTK_POS_BOTTOM, "Clear");
        gtk_scale_add_mark(GTK_SCALE(scale), knob->max, GTK_POS_BOTTOM, "Tinted");
    } else if (strcmp(key, "light_angle") == 0) {
        gtk_scale_add_mark(GTK_SCALE(scale), -45.0, GTK_POS_BOTTOM, "iOS 26");
        gtk_scale_add_mark(GTK_SCALE(scale), 0.0, GTK_POS_BOTTOM, "iOS 27");
    }
    g_object_set_data(G_OBJECT(scale), "knob", GUINT_TO_POINTER(index));
    g_signal_connect(scale, "value-changed", G_CALLBACK(on_scale), app);

    GtkWidget *column = gtk_box_new(GTK_ORIENTATION_VERTICAL, 4);
    gtk_widget_set_margin_start(column, 12);
    gtk_widget_set_margin_end(column, 12);
    gtk_widget_set_margin_top(column, 10);
    gtk_widget_set_margin_bottom(column, 6);
    gtk_box_append(GTK_BOX(column), top);
    gtk_box_append(GTK_BOX(column), words);
    gtk_box_append(GTK_BOX(column), scale);

    GtkWidget *row = adw_preferences_row_new();
    adw_preferences_row_set_title(ADW_PREFERENCES_ROW(row), label);
    gtk_list_box_row_set_child(GTK_LIST_BOX_ROW(row), column);
    gtk_list_box_row_set_activatable(GTK_LIST_BOX_ROW(row), FALSE);
    app->control[index] = scale;
    app->value[index] = value;
    show_value(app, index);
    return row;
}

static GtkWidget *switch_row(App *app, unsigned index) {
    const GlassKnob *knob = &app->knobs[index];
    char label[TEXT_MAX], help[TEXT_MAX];
    text_of(&knob->label, label, sizeof(label));
    text_of(&knob->help, help, sizeof(help));
    GtkWidget *row = adw_switch_row_new();
    adw_preferences_row_set_title(ADW_PREFERENCES_ROW(row), label);
    adw_action_row_set_subtitle(ADW_ACTION_ROW(row), help);
    adw_switch_row_set_active(ADW_SWITCH_ROW(row), app->model.current.values[index] >= 0.5f);
    g_object_set_data(G_OBJECT(row), "knob", GUINT_TO_POINTER(index));
    g_signal_connect(row, "notify::active", G_CALLBACK(on_switch), app);
    app->control[index] = row;
    return row;
}

static GtkWidget *colour_row(App *app, unsigned index) {
    const GlassKnob *knob = &app->knobs[index];
    char label[TEXT_MAX], help[TEXT_MAX];
    text_of(&knob->label, label, sizeof(label));
    text_of(&knob->help, help, sizeof(help));
    GtkWidget *row = adw_action_row_new();
    adw_preferences_row_set_title(ADW_PREFERENCES_ROW(row), label);
    adw_action_row_set_subtitle(ADW_ACTION_ROW(row), help);
    GtkColorDialog *dialog = gtk_color_dialog_new();
    gtk_color_dialog_set_with_alpha(dialog, FALSE);
    GtkWidget *button = gtk_color_dialog_button_new(dialog);
    gtk_widget_set_valign(button, GTK_ALIGN_CENTER);
    g_object_set_data(G_OBJECT(button), "knob", GUINT_TO_POINTER(index));
    g_signal_connect(button, "notify::rgba", G_CALLBACK(on_colour), app);
    adw_action_row_add_suffix(ADW_ACTION_ROW(row), button);
    app->control[index] = button;
    return row;
}

/* The knobs of one group, in the engine's order. */
static GtkWidget *knob_group(App *app, unsigned group) {
    GtkWidget *box = adw_preferences_group_new();
    adw_preferences_group_set_title(ADW_PREFERENCES_GROUP(box), GROUP_TITLE[group]);
    adw_preferences_group_set_description(ADW_PREFERENCES_GROUP(box), GROUP_TEXT[group]);
    for (unsigned i = 0; i < GLASS_TUNING_KNOBS; ++i) {
        const GlassKnob *knob = &app->knobs[i];
        if (knob->group != group)
            continue;
        GtkWidget *row = NULL;
        if (knob->shown == 0)
            row = slider_row(app, i);
        else if (knob->shown == 1)
            row = switch_row(app, i);
        else if (knob->shown == 2)
            row = colour_row(app, i);
        if (row != NULL)
            adw_preferences_group_add(ADW_PREFERENCES_GROUP(box), row);
    }
    return box;
}

static void on_preset(GtkButton *button, gpointer data) {
    App *app = data;
    const char *name = g_object_get_data(G_OBJECT(button), "preset");
    if (name == NULL)
        return;
    begin_change(app, NO_KNOB);
    if (!model_preset(&app->model, name))
        return;
    refresh(app);
    changed(app);
}

/* The engine's presets, as buttons: each sets every slider of the glass at once. */
static GtkWidget *preset_group(App *app) {
    GtkWidget *box = adw_preferences_group_new();
    adw_preferences_group_set_title(ADW_PREFERENCES_GROUP(box), "Start from");
    adw_preferences_group_set_description(ADW_PREFERENCES_GROUP(box),
                                          "Each sets every slider below. The Author's pick is the glass as it comes.");
    GtkWidget *flow = gtk_flow_box_new();
    gtk_widget_add_css_class(flow, "vetro-presets");
    gtk_flow_box_set_selection_mode(GTK_FLOW_BOX(flow), GTK_SELECTION_NONE);
    gtk_flow_box_set_max_children_per_line(GTK_FLOW_BOX(flow), 4);
    gtk_flow_box_set_column_spacing(GTK_FLOW_BOX(flow), 6);
    gtk_flow_box_set_row_spacing(GTK_FLOW_BOX(flow), 6);
    for (unsigned i = 0; i < GLASS_TUNING_PRESETS; ++i) {
        GlassTuningPreset preset;
        if (glass_tuning_preset_info(i, &preset) < 0)
            continue;
        char name[TEXT_MAX], label[TEXT_MAX];
        text_of(&preset.name, name, sizeof(name));
        text_of(&preset.label, label, sizeof(label));
        GtkWidget *button = gtk_button_new_with_label(label);
        gtk_widget_add_css_class(button, "pill");
        g_object_set_data_full(G_OBJECT(button), "preset", g_strdup(name), g_free);
        g_signal_connect(button, "clicked", G_CALLBACK(on_preset), app);
        gtk_flow_box_append(GTK_FLOW_BOX(flow), button);
    }
    adw_preferences_group_add(ADW_PREFERENCES_GROUP(box), flow);
    return box;
}

/* Which look `vetro glass look` says is drawn: the index into LOOK_NAME. */
static guint current_look(void) {
    gchar *out = NULL;
    const gchar *argv[] = {"vetro", "glass", "look", NULL};
    guint found = 0;
    if (g_spawn_sync(NULL, (gchar **)argv, NULL, G_SPAWN_SEARCH_PATH | G_SPAWN_STDERR_TO_DEV_NULL, NULL, NULL, &out, NULL, NULL, NULL) &&
        out != NULL) {
        g_strstrip(out);
        for (guint i = 0; i < LOOKS; ++i) {
            if (strcmp(out, LOOK_NAME[i]) == 0)
                found = i;
        }
    }
    g_free(out);
    return found;
}

static void on_look(GObject *row, GParamSpec *spec, gpointer data) {
    (void)spec;
    App *app = data;
    if (app->updating)
        return;
    const guint chosen = adw_combo_row_get_selected(ADW_COMBO_ROW(row));
    if (chosen >= LOOKS)
        return;
    const gchar *argv[] = {"vetro", "glass", "look", LOOK_NAME[chosen], NULL};
    GError *error = NULL;
    if (!g_spawn_async(NULL, (gchar **)argv, NULL, G_SPAWN_SEARCH_PATH | G_SPAWN_STDOUT_TO_DEV_NULL, NULL, NULL, NULL, &error)) {
        toast(app, "Could not run vetro glass look");
        g_clear_error(&error);
        return;
    }
    toast(app, chosen == 0 ? "The tuned glass is drawn again" : "The sliders change the tuned glass, which is not drawn now");
}

static GtkWidget *look_group(App *app) {
    GtkWidget *box = adw_preferences_group_new();
    adw_preferences_group_set_title(ADW_PREFERENCES_GROUP(box), "Earlier looks");
    adw_preferences_group_set_description(ADW_PREFERENCES_GROUP(box),
                                          "The glass exactly as it was before. The sliders change only the tuned glass.");
    GtkStringList *labels = gtk_string_list_new(LOOK_LABEL);
    GtkWidget *row = adw_combo_row_new();
    adw_preferences_row_set_title(ADW_PREFERENCES_ROW(row), "Glass");
    adw_combo_row_set_model(ADW_COMBO_ROW(row), G_LIST_MODEL(labels));
    app->updating = true;
    adw_combo_row_set_selected(ADW_COMBO_ROW(row), current_look());
    app->updating = false;
    g_signal_connect(row, "notify::selected", G_CALLBACK(on_look), app);
    app->look = ADW_COMBO_ROW(row);
    adw_preferences_group_add(ADW_PREFERENCES_GROUP(box), row);
    return box;
}

static void on_reset(GtkButton *button, gpointer data) {
    (void)button;
    App *app = data;
    begin_change(app, NO_KNOB);
    if (glass_tuning_default(&app->model.current) < 0)
        return;
    refresh(app);
    changed(app);
    toast(app, "Back to the Author's pick, not applied yet");
}

/* Stops a preview still waiting to be sent: what follows decides the glass. */
static void cancel_flush(App *app) {
    if (app->flush != 0) {
        g_source_remove(app->flush);
        app->flush = 0;
    }
}

/* Writes the file and has the engine read it: the glass at every login from
 * now on. False, with the reason shown, when the file could not be written. */
static bool apply_now(App *app) {
    cancel_flush(app);
    if (!model_save(&app->model)) {
        toast(app, "Could not write ~/.config/vetro/glass.conf");
        return false;
    }
    tell_engine(app, "hyprglass tune reload");
    app->burst_knob = NO_KNOB;
    update_state(app);
    return true;
}

/* The glass back to what is applied, for changes that are not kept. */
static void discard_now(App *app) {
    cancel_flush(app);
    if (model_dirty(&app->model))
        tell_engine(app, "hyprglass tune reload");
    app->model.current = app->model.applied;
}

static void do_apply(GSimpleAction *action, GVariant *parameter, gpointer data) {
    (void)action;
    (void)parameter;
    App *app = data;
    if (model_dirty(&app->model) && apply_now(app))
        toast(app, "Applied: the glass stays like this, at every login too");
}

static void do_revert(GSimpleAction *action, GVariant *parameter, gpointer data) {
    (void)action;
    (void)parameter;
    App *app = data;
    if (!model_dirty(&app->model))
        return;
    begin_change(app, NO_KNOB);
    app->model.current = app->model.applied;
    refresh(app);
    changed(app);
    toast(app, "Back to the glass as last applied");
}

/* One step back or forward. The next drag of a slider starts a step of its own. */
static void step(App *app, bool forward) {
    if (!(forward ? model_redo(&app->model) : model_undo(&app->model)))
        return;
    app->burst_knob = NO_KNOB;
    refresh(app);
    changed(app);
}

static void do_undo(GSimpleAction *action, GVariant *parameter, gpointer data) {
    (void)action;
    (void)parameter;
    step(data, false);
}

static void do_redo(GSimpleAction *action, GVariant *parameter, gpointer data) {
    (void)action;
    (void)parameter;
    step(data, true);
}

/* The window's own action `name`, which the buttons and the shortcuts use. */
static GSimpleAction *add_action(GtkWidget *window, const char *name, GCallback handler, App *app) {
    GSimpleAction *action = g_simple_action_new(name, NULL);
    g_signal_connect(action, "activate", handler, app);
    g_action_map_add_action(G_ACTION_MAP(window), G_ACTION(action));
    /* the window keeps it from here on, and the app's pointer lives as long */
    g_object_unref(action);
    return action;
}

static void add_actions(App *app, GtkWidget *window, GtkApplication *application) {
    app->undo = add_action(window, "undo", G_CALLBACK(do_undo), app);
    app->redo = add_action(window, "redo", G_CALLBACK(do_redo), app);
    app->apply = add_action(window, "apply", G_CALLBACK(do_apply), app);
    app->revert = add_action(window, "revert", G_CALLBACK(do_revert), app);
    const char *const undo[] = {"<Control>z", NULL};
    const char *const redo[] = {"<Control><Shift>z", "<Control>y", NULL};
    const char *const apply[] = {"<Control>s", NULL};
    gtk_application_set_accels_for_action(application, "win.undo", undo);
    gtk_application_set_accels_for_action(application, "win.redo", redo);
    gtk_application_set_accels_for_action(application, "win.apply", apply);
}

static void on_close_answer(GObject *source, GAsyncResult *result, gpointer data) {
    App *app = data;
    const char *answer = adw_alert_dialog_choose_finish(ADW_ALERT_DIALOG(source), result);
    if (strcmp(answer, "apply") == 0) {
        if (!apply_now(app))
            return;
    } else if (strcmp(answer, "discard") == 0) {
        discard_now(app);
    } else {
        return;
    }
    gtk_window_destroy(app->window);
}

/* Closing with changes not applied asks whether to keep them. */
static gboolean on_close_request(GtkWindow *window, gpointer data) {
    (void)window;
    App *app = data;
    if (!model_dirty(&app->model))
        return FALSE;
    AdwDialog *dialog = adw_alert_dialog_new(
        "Apply the changes?", "The glass shows changes that are not applied yet. If they are discarded, it goes back to how it was.");
    AdwAlertDialog *alert = ADW_ALERT_DIALOG(dialog);
    adw_alert_dialog_add_responses(alert, "cancel", "Cancel", "discard", "Discard", "apply", "Apply", NULL);
    adw_alert_dialog_set_response_appearance(alert, "discard", ADW_RESPONSE_DESTRUCTIVE);
    adw_alert_dialog_set_response_appearance(alert, "apply", ADW_RESPONSE_SUGGESTED);
    adw_alert_dialog_set_default_response(alert, "apply");
    adw_alert_dialog_set_close_response(alert, "cancel");
    adw_alert_dialog_choose(alert, GTK_WIDGET(app->window), NULL, on_close_answer, app);
    return TRUE;
}

static GtkWidget *icon_button(const char *icon, const char *action, const char *tip) {
    GtkWidget *button = gtk_button_new_from_icon_name(icon);
    gtk_actionable_set_action_name(GTK_ACTIONABLE(button), action);
    gtk_widget_set_tooltip_text(button, tip);
    return button;
}

static GtkWidget *header(App *app) {
    GtkWidget *bar = adw_header_bar_new();
    GtkWidget *brand = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 8);
    GtkWidget *icon = gtk_image_new_from_icon_name(APP_ID "-symbolic");
    gtk_image_set_pixel_size(GTK_IMAGE(icon), 18);
    gtk_box_append(GTK_BOX(brand), icon);
    GtkWidget *title = gtk_label_new("Vetro Glass");
    gtk_widget_add_css_class(title, "vetro-header");
    gtk_box_append(GTK_BOX(brand), title);
    adw_header_bar_set_title_widget(ADW_HEADER_BAR(bar), brand);
    GtkWidget *undo = icon_button("edit-undo-symbolic", "win.undo", "Undo (Ctrl+Z)");
    GtkWidget *redo = icon_button("edit-redo-symbolic", "win.redo", "Redo (Ctrl+Shift+Z)");
    GtkWidget *revert = gtk_button_new_with_label("Revert");
    gtk_actionable_set_action_name(GTK_ACTIONABLE(revert), "win.revert");
    gtk_widget_set_tooltip_text(revert, "Back to the glass as last applied");
    GtkWidget *reset = gtk_button_new_with_label("Reset");
    gtk_widget_set_tooltip_text(reset, "Every slider back to the Author's pick, the glass as it comes");
    g_signal_connect(reset, "clicked", G_CALLBACK(on_reset), app);
    GtkWidget *apply = gtk_button_new_with_label("Apply");
    gtk_widget_add_css_class(apply, "suggested-action");
    gtk_actionable_set_action_name(GTK_ACTIONABLE(apply), "win.apply");
    gtk_widget_set_tooltip_text(apply, "Keep this glass, at every login too (Ctrl+S)");
    adw_header_bar_pack_start(ADW_HEADER_BAR(bar), undo);
    adw_header_bar_pack_start(ADW_HEADER_BAR(bar), redo);
    adw_header_bar_pack_start(ADW_HEADER_BAR(bar), revert);
    adw_header_bar_pack_end(ADW_HEADER_BAR(bar), apply);
    adw_header_bar_pack_end(ADW_HEADER_BAR(bar), reset);
    return bar;
}

static GtkWidget *brand_group(void) {
    GtkWidget *group = adw_preferences_group_new();
    GtkWidget *hero = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 20);
    gtk_widget_add_css_class(hero, "vetro-hero");

    GtkWidget *icon = gtk_image_new_from_resource("/dev/vetro/Glass/icons/scalable/apps/" APP_ID ".svg");
    gtk_image_set_pixel_size(GTK_IMAGE(icon), 88);
    gtk_widget_set_valign(icon, GTK_ALIGN_CENTER);
    gtk_box_append(GTK_BOX(hero), icon);

    GtkWidget *words = gtk_box_new(GTK_ORIENTATION_VERTICAL, 6);
    gtk_widget_set_valign(words, GTK_ALIGN_CENTER);
    gtk_widget_set_hexpand(words, TRUE);
    const char *const text[] = {"VETRO", "Liquid glass for Omarchy"};
    const char *const style[] = {"vetro-wordmark", "vetro-tagline"};
    for (unsigned i = 0; i < G_N_ELEMENTS(text); ++i) {
        GtkWidget *label = gtk_label_new(text[i]);
        gtk_label_set_xalign(GTK_LABEL(label), 0.0f);
        gtk_label_set_wrap(GTK_LABEL(label), TRUE);
        gtk_widget_add_css_class(label, style[i]);
        gtk_box_append(GTK_BOX(words), label);
    }
    gtk_box_append(GTK_BOX(hero), words);
    adw_preferences_group_add(ADW_PREFERENCES_GROUP(group), hero);
    return group;
}

static GtkWidget *page(App *app) {
    GtkWidget *content = adw_preferences_page_new();
    adw_preferences_page_add(ADW_PREFERENCES_PAGE(content), ADW_PREFERENCES_GROUP(brand_group()));
    adw_preferences_page_add(ADW_PREFERENCES_PAGE(content), ADW_PREFERENCES_GROUP(preset_group(app)));
    for (unsigned group = 0; group < GROUPS; ++group)
        adw_preferences_page_add(ADW_PREFERENCES_PAGE(content), ADW_PREFERENCES_GROUP(knob_group(app, group)));
    adw_preferences_page_add(ADW_PREFERENCES_PAGE(content), ADW_PREFERENCES_GROUP(look_group(app)));
    return content;
}

static void on_activate(GtkApplication *application, gpointer data) {
    App *app = data;
    GtkWindow *existing = gtk_application_get_active_window(application);
    if (existing != NULL) {
        gtk_window_present(existing);
        return;
    }
    adw_style_manager_set_color_scheme(adw_style_manager_get_default(), ADW_COLOR_SCHEME_PREFER_DARK);
    GdkDisplay *display = gdk_display_get_default();
    gtk_icon_theme_add_resource_path(gtk_icon_theme_get_for_display(display), "/dev/vetro/Glass/icons");
    GtkCssProvider *css = gtk_css_provider_new();
    gtk_css_provider_load_from_resource(css, "/dev/vetro/Glass/style.css");
    gtk_style_context_add_provider_for_display(display, GTK_STYLE_PROVIDER(css), GTK_STYLE_PROVIDER_PRIORITY_APPLICATION);
    g_object_unref(css);

    GtkWidget *window = adw_application_window_new(application);
    app->window = GTK_WINDOW(window);
    app->burst_knob = NO_KNOB;
    add_actions(app, window, application);
    g_signal_connect(window, "close-request", G_CALLBACK(on_close_request), app);

    app->banner = ADW_BANNER(adw_banner_new("The glass running now is from before tuning: run vetro glass load, or log in again"));
    app->toasts = ADW_TOAST_OVERLAY(adw_toast_overlay_new());
    adw_toast_overlay_set_child(app->toasts, page(app));
    refresh(app);

    GtkWidget *view = adw_toolbar_view_new();
    adw_toolbar_view_add_top_bar(ADW_TOOLBAR_VIEW(view), header(app));
    adw_toolbar_view_add_top_bar(ADW_TOOLBAR_VIEW(view), GTK_WIDGET(app->banner));
    adw_toolbar_view_set_content(ADW_TOOLBAR_VIEW(view), GTK_WIDGET(app->toasts));

    gtk_widget_add_css_class(window, "vetro-window");
    gtk_window_set_icon_name(GTK_WINDOW(window), APP_ID);
    gtk_window_set_title(GTK_WINDOW(window), "Vetro Glass");
    gtk_window_set_default_size(GTK_WINDOW(window), 560, 860);
    adw_application_window_set_content(ADW_APPLICATION_WINDOW(window), view);
    update_state(app);
    gtk_window_present(GTK_WINDOW(window));

    /* The engine reads the file again, so the glass is the glass applied even
     * if an earlier window ended while it showed changes. */
    tell_engine(app, "hyprglass tune reload");
}

/* Ending without the question (the app was told to quit): changes not applied
 * are not kept, and the glass goes back to the file. The window is gone by
 * now, so the engine is told directly. */
static void on_shutdown(GApplication *application, gpointer data) {
    (void)application;
    App *app = data;
    cancel_flush(app);
    if (model_dirty(&app->model)) {
        char reply[512];
        (void)hypr_request("hyprglass tune reload", reply, sizeof(reply));
    }
}

static bool load_knobs(App *app) {
    for (unsigned i = 0; i < GLASS_TUNING_KNOBS; ++i) {
        if (glass_tuning_knob(i, &app->knobs[i]) < 0)
            return false;
    }
    return true;
}

int main(int argc, char **argv) {
    static App app;
    if (glass_abi_version() != GLASS_ABI_VERSION || !load_knobs(&app) || !model_init(&app.model)) {
        fprintf(stderr, "vetro-glass: the glass library does not match this app\n");
        return 1;
    }
    AdwApplication *application = adw_application_new(APP_ID, G_APPLICATION_DEFAULT_FLAGS);
    g_signal_connect(application, "activate", G_CALLBACK(on_activate), &app);
    g_signal_connect(application, "shutdown", G_CALLBACK(on_shutdown), &app);
    const int status = g_application_run(G_APPLICATION(application), argc, argv);
    g_object_unref(application);
    return status;
}

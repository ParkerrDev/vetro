#include "model.h"
#include <gio/gio.h>
#include <glib/gstdio.h>
#include <string.h>

int main(void) {
    char *scratch = g_dir_make_tmp("vetro-looks-XXXXXX", NULL);
    g_assert_nonnull(scratch);
    g_setenv("XDG_CONFIG_HOME", scratch, TRUE);
    Model model;
    g_assert_true(model_init(&model));
    GlassTuning original = model.current;
    g_assert_true(model_preset(&model, "tinted"));
    g_assert_true(model_dirty(&model));
    GlassTuning tinted = model.current;
    g_assert_true(model_save_look(&model, "Evening glass"));
    g_assert_false(g_file_test(model.path, G_FILE_TEST_EXISTS));
    g_assert_false(model_save_look(&model, "Evening glass"));
    g_assert_false(model_save_look(&model, "../escape"));
    g_assert_false(model_save_look(&model, "/tmp/escape"));
    g_assert_false(model_save_look(&model, ""));
    g_assert_false(model_save_look(&model, " hidden"));
    g_assert_false(model_save_look(&model, "<markup>"));
    g_assert_true(model_save_look(&model, "Été"));
    char **looks = model_list_looks(&model);
    g_assert_cmpuint(g_strv_length(looks), ==, 2);
    g_strfreev(looks);
    model.current = original;
    g_assert_true(model_load_look(&model, "Evening glass"));
    /* The engine's text format stores colours as 8-bit RGB. */
    for (unsigned i = 0; i < GLASS_TUNING_KNOBS; ++i)
        g_assert_cmpfloat_with_epsilon(model.current.values[i], tinted.values[i], 0.5f / 255.0f);
    tinted = model.current;
    g_assert_true(model_undo(&model));
    g_assert_cmpmem(&model.current, sizeof(original), &original, sizeof(original));
    g_assert_true(model_redo(&model));
    g_assert_true(model_save(&model));
    g_assert_false(model_dirty(&model));
    Model reopened;
    g_assert_true(model_init(&reopened));
    g_assert_cmpmem(&reopened.current, sizeof(tinted), &tinted, sizeof(tinted));
    char *folder = g_build_filename(scratch, "vetro", "looks", NULL);
    char *bad = g_build_filename(folder, "Broken.conf", NULL);
    g_assert_true(g_file_set_contents(bad, "frost = nonsense\n", -1, NULL));
    g_assert_false(model_load_look(&model, "Broken"));
    g_assert_cmpmem(&model.current, sizeof(tinted), &tinted, sizeof(tinted));
    g_assert_cmpuint(model.undo_count, ==, 1);
    g_assert_cmpint(g_remove(bad), ==, 0);
    GFile *link = g_file_new_for_path(bad);
    g_assert_true(g_file_make_symbolic_link(link, model.path, NULL, NULL));
    g_assert_false(model_load_look(&model, "Broken"));
    g_assert_false(model_save_look(&model, "Broken"));
    g_object_unref(link);
    g_remove(bad);
    looks = model_list_looks(&model);
    for (char **name = looks; *name; ++name) {
        char *path = g_strdup_printf("%s/%s.conf", folder, *name);
        g_remove(path);
        g_free(path);
    }
    g_strfreev(looks);
    g_remove(model.path);
    g_rmdir(folder);
    char *parent = g_path_get_dirname(folder);
    g_rmdir(parent);
    g_rmdir(scratch);
    g_free(parent); g_free(folder); g_free(bad); g_free(scratch);
    g_print("Saved looks: round-trip, undo/redo, explicit apply, invalid names and symlink protection passed\n");
    return 0;
}

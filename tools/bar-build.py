#!/usr/bin/env python3
"""Build Vetro's bar from the Omarchy bar that is installed.

Vetro's bar is Omarchy's own, every widget, panel and gesture of it, with
three things changed:

  the glass   the pane of glass is drawn by the bar's own surface, which is
              above windows, and not by a strip of its own under them. A
              window that slides past the bar goes under the glass too.
  no drags    nothing on the bar is moved by dragging it, and the bar is not
              dragged to another edge. Where things are is what shell.json
              says (`omarchy bar move`, `omarchy bar position`).
  no toggle   a double click on the bar does not make it see-through: the
              glass is its background.

It is built here, and not kept as a copy in the repository, so that it is
always the bar of the Omarchy that is installed: an update to Omarchy's bar is
in Vetro's the next time this runs (install.sh, `vetro on`, and Omarchy's hook
after an update). Each change is made at one place that must be found exactly
once. If Omarchy's bar has changed so that one is not, nothing is built, the
last build is removed, and Omarchy's own bar is used with the glass strip
under it, as before Vetro had a bar.

  bar-build.py SOURCE OUT VETRO     build OUT from SOURCE; VETRO is the folder
                                    of Vetro's own parts (shell/bar): the
                                    plugin's manifest and the pane of glass
  bar-build.py --stamp SOURCE       print what a build from SOURCE is marked
                                    with, to tell whether one is up to date

Exit 0 when OUT is built, 1 when it could not be (the reason is printed).
"""
import hashlib
import os
import shutil
import sys

# (what it is, the text to find, the text to put there). Each `find` must be
# in Omarchy's Bar.qml exactly once.
CHANGES = [
    (
        "the switch for the gestures Vetro's bar does without",
        "Item {\n  id: root\n",
        "Item {\n  id: root\n\n"
        "  // Vetro: the bar and what is on it stay where shell.json puts them, and the\n"
        "  // bar keeps its glass. No drag moves a widget or the bar, and no double\n"
        "  // click makes the bar see-through.\n"
        "  readonly property bool vetroGestures: false\n",
    ),
    (
        "the pane of glass is part of the bar's surface",
        '    WlrLayershell.namespace: "omarchy-bar"\n'
        "    WlrLayershell.layer: WlrLayer.Top\n"
        "\n"
        "    Loader {\n",
        '    WlrLayershell.namespace: "vetro:bar"\n'
        "    WlrLayershell.layer: WlrLayer.Top\n"
        "\n"
        "    // Vetro: the glass, drawn by this surface and so above windows.\n"
        "    VetroPane {\n"
        "      anchors.fill: parent\n"
        "      vertical: root.vertical\n"
        "      barSize: root.barSize\n"
        "      seeThrough: root.transparent\n"
        "    }\n"
        "\n"
        "    Loader {\n",
    ),
    (
        "no widget is moved by dragging it",
        'readonly property bool canReorder: root.shell && typeof root.shell.mutateShellConfig === "function"\n',
        "readonly property bool canReorder: root.vetroGestures && root.shell"
        ' && typeof root.shell.mutateShellConfig === "function"\n',
    ),
    (
        "the bar is not dragged to another edge",
        "    function startDrag(x, y) {\n      if (dragging) return\n",
        "    function startDrag(x, y) {\n      if (!root.vetroGestures || dragging) return\n",
    ),
    (
        "a double click does not make the bar see-through",
        "        root.toggleTransparency()\n",
        "        if (root.vetroGestures) root.toggleTransparency()\n",
    ),
]

FILES = ("Bar.qml", "BarModel.js")
# Vetro's own parts. The manifest is copied; the pane is linked, so that it is
# always the one in this checkout. It sits beside Bar.qml, where QML finds a
# component by its file's name, and where its own import of Vetro's library
# (a folder beside the plugin's) leads to the same place as every other
# plugin's does.
MANIFEST = "manifest.json"
PANE = "VetroPane.qml"
HERE = os.path.dirname(os.path.abspath(__file__))


def stamp(source):
    """What a build is marked with: Omarchy's two files, the manifest, this file."""
    digest = hashlib.sha256()
    for name in FILES:
        with open(os.path.join(source, name), "rb") as handle:
            digest.update(handle.read())
    for path in (os.path.join(HERE, "..", "shell", "bar", MANIFEST), os.path.abspath(__file__)):
        with open(path, "rb") as handle:
            digest.update(handle.read())
    return digest.hexdigest()


def changed(text):
    """Omarchy's Bar.qml with Vetro's changes, or the change that has no place."""
    for what, find, put in CHANGES:
        count = text.count(find)
        if count != 1:
            return None, "%s: its place is in Omarchy's Bar.qml %d times, not once" % (what, count)
        text = text.replace(find, put)
    return text, None


def build(source, out, vetro):
    for name in FILES:
        if not os.path.isfile(os.path.join(source, name)):
            return "Omarchy's bar is not at %s (%s is missing)" % (source, name)
    for name in (MANIFEST, PANE):
        if not os.path.isfile(os.path.join(vetro, name)):
            return "Vetro's own parts are not at %s (%s is missing)" % (vetro, name)
    with open(os.path.join(source, "Bar.qml"), encoding="utf-8") as handle:
        text, why = changed(handle.read())
    if text is None:
        return why
    fresh = out + ".new"
    shutil.rmtree(fresh, ignore_errors=True)
    os.makedirs(fresh)
    with open(os.path.join(fresh, "Bar.qml"), "w", encoding="utf-8") as handle:
        handle.write(text)
    shutil.copyfile(os.path.join(source, "BarModel.js"), os.path.join(fresh, "BarModel.js"))
    shutil.copyfile(os.path.join(vetro, MANIFEST), os.path.join(fresh, MANIFEST))
    os.symlink(os.path.join(vetro, PANE), os.path.join(fresh, PANE))
    with open(os.path.join(fresh, "built-from"), "w", encoding="utf-8") as handle:
        handle.write(stamp(source) + "\n")
    # The new one takes the old one's place in one step each way.
    old = out + ".old"
    shutil.rmtree(old, ignore_errors=True)
    if os.path.lexists(out):
        os.rename(out, old)
    os.rename(fresh, out)
    shutil.rmtree(old, ignore_errors=True)
    return None


def main(argv):
    if len(argv) == 3 and argv[1] == "--stamp":
        try:
            print(stamp(argv[2]))
        except OSError:
            return 1
        return 0
    if len(argv) != 4:
        sys.stderr.write(__doc__)
        return 2
    source, out, vetro = argv[1], os.path.abspath(argv[2]), os.path.abspath(argv[3])
    # A folder this builds into is one it may also remove: never a short path.
    if out.count(os.sep) < 4 or os.path.basename(out) != "bar":
        print("refusing to build into %s" % out)
        return 1
    if os.path.lexists(out) and not os.path.isfile(os.path.join(out, "built-from")):
        print("%s is there and was not built by this: left alone" % out)
        return 1
    why = build(source, out, vetro)
    if why is not None:
        shutil.rmtree(out, ignore_errors=True)
        print(why)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))

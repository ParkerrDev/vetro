# How Vetro builds the engine

This folder is the glass engine, a fork of hyprnux/hyprglass (BSD 3-Clause; see
`LICENSE`), with the whole of its history in this repository. `README.md` is the
engine's own, as upstream wrote it and the fork extended it. This file is about
what Vetro adds around it.

```
build.sh        builds the engine for the Hyprland that is installed
pins            which engine commit goes with which Hyprland commit
post-update.sh  the hook Omarchy runs after an update, which runs build.sh
```

`build.sh` reads the commit of the installed Hyprland from its headers
(`pkg-config --cflags hyprland`), finds that commit in `pins`, fetches the engine
commit the line names from this repository, runs `make` on it, and puts the result
in `~/.local/lib/vetro`:

```
hyprglass.so            the plugin (the engine's name inside Hyprland is hyprglass)
hyprglass.built-for     the Hyprland commit, its tag, and the engine commit
```

A Hyprland that is not in `pins` is refused (exit 3) and whatever was built is
left where it is. A line is added to `pins` only after that pair has been built and
seen working: the engine hooks into Hyprland's private drawing code, and a pairing
that merely compiles proves nothing. A line is

```
<Hyprland commit>  <engine commit>  [<folder>]   # what they are
```

where the engine commit is a commit of this repository, and the folder is where
the engine's Makefile is in that commit: `.` for the commits that came in with the
hyprglass history (the engine was the whole tree), `engine` for one made since.

```sh
engine/build.sh            build if there is nothing built for this Hyprland
engine/build.sh --force    build again regardless
engine/build.sh --check    build nothing: exit 0 built, 1 not built, 3 no pin
```

A build is moved into place and never written over: a running Hyprland may have
the old file loaded, and keeps it until the next login. The engine is loaded by
`vetro glass load` from outside Hyprland's config, never by the config itself.

The engine's own tests (the lens model, option parsing, the shaders; they compile,
and need no display) run with `make -C engine test`, or
`VETRO_TEST_ENGINE=1 scripts/check-repo`.

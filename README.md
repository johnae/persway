## Persway - a simple sway ipc daemon

This is a small daemon that listens to sway events over an ipc socket. It will set the workspace name dynamically to the name of the focused application if `workspace-renaming` is enabled.
If `autolayout` is enabled (see command line options below), it will also alternate between horizontal / vertical splits, sort of like AwesomeWM.

The meat of persway are the on-window-focus and on-exit handlers which can be used to set the opacity of focused and non-focused windows for example (see below examples.)

```
persway 0.5.0
I am Persway. A friendly daemon.

I talk to the Sway Compositor and persuade it to do little evil things. Give me an option and see what it brings.

USAGE:
    persway [FLAGS] [OPTIONS]

FLAGS:
    -a, --autolayout
            Set the level of opacity to give non-focused containers, the default of 1.0 means persway will not set any
            opacity at all. Do not set opacity of the windows with given criteria. Multiple criteria can be specified.
            Enable autolayout, alternating between horizontal and vertical somewhat reminiscent of the Awesome WM
    -h, --help
            Prints help information

    -V, --version
            Prints version information

    -w, --workspace-renaming
            Enable automatic workspace renaming based on what is running in the workspace (eg. application name)


OPTIONS:
    -e, --on-exit <on-exit>
            Called when persway exits. This can be used to reset any opacity changes or other settings when persway
            exits. For example, if changing the opacity on window focus, you would probably want to reset that on exit
            like this:

            [tiling] opacity 1

            Eg. set all tiling windows to opacity 1
    -f, --on-window-focus <on-window-focus>
            Called when window comes into focus. To automatically set the opacity of all other windows to 0.8 for
            example, you would set this to:

            [tiling] opacity 0.8; opacity 1

            Eg. set all tiling windows to opacity 0.8 but set the currently focused window to opacity 1. Or if you want
            to skip some applications - in this case firefox - you would do something like:

            [tiling] opacity 0.8; [app_id="firefox"] opacity 1; opacity 1
    -l, --on-window-focus-leave <on-window-focus-leave>
            Called when window leaves focus. To automatically mark these for example, you would set this to:

            mark --add _prev

            and then in your sway config:

            bindsym Mod1+tab [con_mark=_prev] focus
```

If you have trouble with workspace naming/numbering and switching workspaces, please see this issue comment: https://github.com/johnae/persway/issues/2#issuecomment-644343784 - the gist of it is that it is likely a sway config issue.


### Nix flake

If you happen to be on [NixOS](https://nixos.org) or you're using the Nix Package Manager, you can easily use the flake and overlay from this repo (provided you're using Nix flakes ofc).

Persway is released under the MIT license.
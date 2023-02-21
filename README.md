## Persway - a simple sway ipc daemon

This is a small daemon that listens to sway events over an ipc socket. It will set the workspace name dynamically to the name of the focused application if `workspace-renaming` is enabled.
If `Autolayout` is enabled (see command line options below), it will also alternate between horizontal / vertical splits, sort of like AwesomeWM.

THE meat of persway are the on-window-focus and on-exit handlers which can be used to set the opacity of focused and non-focused windows for example (see below examples.)

There is one breaking change in 0.6.0 which is that cli flags and options that earlier were given to persway directly, now go under the `daemon` subcommand. This is because persway has gained the ability to talk to itself :-). That is - in version 0.6.0 persway can talk to itself through a socket. This in turn is because persway now supports two layouts: `spiral` and `stack_main` (sometimes referred to as master stack). Especially `stack_main` becomes especially useful when it's easy to move around in the stack. `spiral` is basically what was called `autolayout` in earlier versions of persway.


Main cli interface:

```
I am Persway. A friendly daemon.

I talk to the Sway Compositor and persuade it to do little evil things. Give me an option and see what it brings. I also talk to myself.

Usage: persway [OPTIONS] <COMMAND>

Commands:
  daemon
          This starts the persway daemon
  stack-focus-next
          Applies to stack main layout - focuses the next stacked window
  stack-focus-prev
          Applies to stack main layout - focuses the previous stacked window
  stack-swap-visible
          Applies to stack main layout - swaps the visible stacked window with the main window
  stack-main-rotate-next
          Applies to stack main layout - pops the top of the stack into main while pushing the old main window to the bottom of the stack
  change-layout
          Changes the layout of the focused workspace
  help
          Print this message or the help of the given subcommand(s)

Options:
  -s, --socket-path <SOCKET_PATH>
          Path to control socket. This option applies both to daemon and client. Defaults to <XDG_RUNTIME_DIR>/persway-<WAYLAND_DISPLAY>.sock

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```

The daemon cli interface:

```
This starts the persway daemon

Usage: persway daemon [OPTIONS]

Options:
  -d, --default-layout <DEFAULT_LAYOUT>
          Which layout should be the default when no other layout has been specified for a workspace. Options are: manual, spiral and stack_main
          
          [default: manual]

  -w, --workspace-renaming
          Enable automatic workspace renaming based on what is running in the workspace (eg. application name)

  -f, --on-window-focus <ON_WINDOW_FOCUS>
          Called when window comes into focus. To automatically set the opacity of all other windows to 0.8 for example, you would set this to:
          
          [tiling] opacity 0.8; opacity 1
          
          Eg. set all tiling windows to opacity 0.8 but set the currently focused window to opacity 1. Or if you want to skip some applications - in this case firefox - you would do something like:
          
          [tiling] opacity 0.8; [app_id="firefox"] opacity 1; opacity 1

  -l, --on-window-focus-leave <ON_WINDOW_FOCUS_LEAVE>
          Called when window leaves focus. To automatically mark these for example, you would set this to:
          
          mark --add _prev
          
          and then in your sway config:
          
          bindsym Mod1+tab [con_mark=_prev] focus

  -e, --on-exit <ON_EXIT>
          Called when persway exits. This can be used to reset any opacity changes or other settings when persway exits. For example, if changing the opacity on window focus, you would probably want to reset that on exit like this:
          
          [tiling] opacity 1
          
          Eg. set all tiling windows to opacity 1

  -h, --help
          Print help (see a summary with '-h')
```

The `change-layout` subcommand takes layout subcommands:

```
Changes the layout of the focused workspace

Usage: persway change-layout <COMMAND>

Commands:
  spiral      The spiral autotiling layout tiles windows in a spiral formation, similar to AwesomeWM
  stack-main  The stack_main autotiling layout keeps a stack of windows on the side of a larger main area, this layout comes with a few commands to control it as well
  manual      The normal sway dynamic tiling
  help        Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

The `change-layout` `stack-main` subcommand takes a few options:

```
The stack_main autotiling layout keeps a stack of windows on the side of a larger main area, this layout comes with a few commands to control it as well

Usage: persway change-layout stack-main --size <SIZE> --stack-layout <STACK_LAYOUT>

Options:
  -s, --size <SIZE>                  Size of the main area in percent
  -l, --stack-layout <STACK_LAYOUT>  The sway layout of the stack: tabbed, tiled or stacked. Stacked is the default
  -h, --help                         Print help
```


There may be other subcommands that take options as well. Go explore.


If you have trouble with workspace naming/numbering and switching workspaces, please see this issue comment: https://github.com/johnae/persway/issues/2#issuecomment-644343784 - the gist of it is that it is likely a sway config issue.


### Nix flake

If you happen to be on [NixOS](https://nixos.org) or you're using the Nix Package Manager, you can easily use the flake and overlay from this repo (provided you're using Nix flakes ofc).

Persway is released under the MIT license.

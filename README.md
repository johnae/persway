

https://user-images.githubusercontent.com/28332/223278211-ba3943ee-becc-45e5-ae0e-4f1a121a6f17.mp4

_Parental Advisory - Explicit Content. Unmute the video for the full experience._

## Persway - the scheming, evil sway ipc daemon

Persway works with the Sway Compositor, it persuades it to do little evil things. It features window focus handlers that can be used to adjust the opacity of focused and non-focused windows among many other things. Persway currently supports two layouts: `spiral` and `stack_main`. The first alternates between horizontal and vertical splits based on window geometry - this usually results in something that looks like a spiral, this layout is the same as what persway previously just called `autolayout`. The latter, i.e `stack_main`, keeps a stack of windows on the side of a larger main area (this layout is sometimes referred to as master stack).
Persway comes with several commands to control the `stack_main` layout as you move around in it. Persway talks to itself through a socket and listens to sway events through the sway socket making it a flexible tool for manipulating the [Sway Compositor](https://github.com/swaywm/sway).

In persway version 0.6.0 the cli interface was changed in a backwards incompatible way. However, the change is minor, all the options and arguments from previous versions are now instead available underneath the `daemon` subcommand. So the migration path is simply:

If your previous pre-0.6.0 setup looked like this:
```
persway -w -e '[tiling] opacity 1' -f '[tiling] opacity 0.95; opacity 1' -l 'mark --add _prev' --autolayout 
```

The same setup on 0.6.0 and up should instead look like this:

```
persway daemon -w -e '[tiling] opacity 1' -f '[tiling] opacity 0.95; opacity 1' -l 'mark --add _prev' --default-layout spiral
```

This change was made because persway, as noted above, has gained the ability to talk to itself. That is - in version 0.6.0, persway can talk to itself through a socket to do various things.


### Configure and setup Persway

To set up Persway, you need to run the daemon using the `persway daemon` subcommand with the appropriate options. Once the daemon is running, you can use the client portion of Persway to communicate with the daemon. For example by binding keys to layout movement and switching.

In essence, to set up Persway, follow these steps:

Run the daemon using the appropriate options. Here's an example:

```
persway daemon -w -e '[tiling] opacity 1' -f '[tiling] opacity 0.95; opacity 1' -l 'mark --add _prev' -d stack_main
```

Then bind keys to layout movement and switching. For example, you could use the following sway bindings:

```
bindsym Mod4+Control+space exec persway stack-main-rotate-next
bindsym Mod4+Shift+Tab exec persway stack-focus-prev
bindsym Mod4+Tab exec persway stack-focus-next
bindsym Mod4+c exec persway change-layout stack-main --size 70 --stack-layout tiled
bindsym Mod4+space exec persway stack-swap-main
bindsym Mod4+v exec persway change-layout manual
bindsym Mod4+x exec persway change-layout stack-main --size 70
bindsym Mod4+z exec persway change-layout spiral
```

### The cli

This is the main cli interface:

```
I am Persway. An evil, scheming, friendly daemon.

I talk to the Sway Compositor and persuade it to do little evil things. Give me an option and see what it brings. I also talk to myself.

Usage: persway [OPTIONS] <COMMAND>

Commands:
  daemon
          Starts the persway daemon
  stack-focus-next
          Applies to stack main layout - focuses the next stacked window
  stack-focus-prev
          Applies to stack main layout - focuses the previous stacked window
  stack-swap-main
          Applies to stack main layout - swaps the current stacked window with the main window
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
Starts the persway daemon

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
  manual      The normal sway manual tiling
  help        Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

The `change-layout` `stack-main` subcommand takes a few options:

```
The stack_main autotiling layout keeps a stack of windows on the side of a larger main area, this layout comes with a few commands to control it as well

Usage: persway change-layout stack-main [OPTIONS]

Options:
  -s, --size <SIZE>                  Size of the main area in percent [default: 70]
  -l, --stack-layout <STACK_LAYOUT>  The sway layout of the stack: tabbed, tiled or stacked [default: stacked]
  -h, --help                         Print help

```

There are other subcommands as well. Go explore. I'll try to do a better job documenting things in the future.

If you have trouble with workspace naming/numbering and switching workspaces, please see this issue comment: https://github.com/johnae/persway/issues/2#issuecomment-644343784 - the gist of it is that it is likely a sway config issue.


### Nix flake

If you happen to be on [NixOS](https://nixos.org) or you're using the Nix Package Manager, you can easily use the flake and overlay from this repo (provided you're using Nix flakes ofc).

Persway is released under the MIT license.

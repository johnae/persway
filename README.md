## Persway - a simple sway ipc daemon

This is a small daemon that listens to sway events over an ipc socket. It sets the opacity of all non-focused windows to the configured value. It will also set the workspace name dynamically to the name of the focused application if `workspace-renaming` is enabled.
If `autolayout` is enabled (see command line options below), it will also alternate between horizontal / vertical splits, sort of like AwesomeWM.

Default opacity is `0.78` but can be changed via a command line parameter:

```
persway 0.4.0
I am Persway. A friendly daemon.

I talk to the Sway Compositor and persuade it to do little evil things. Give me an option and see what it brings.

USAGE:
    persway [FLAGS] [OPTIONS]

FLAGS:
    -a, --autolayout
            Enable autolayout, alternating between horizontal and vertical somewhat reminiscent of the Awesome WM

    -h, --help
            Prints help information

    -V, --version
            Prints version information

    -w, --workspace-renaming
            Enable automatic workspace renaming based on what is running in the workspace (eg. application name)


OPTIONS:
    -o, --opacity <opacity>
            Set the level of opacity to give non-focused containers, the default of 1.0 means persway will not set any
            opacity at all [default: 1.0]
```

If you have trouble with workspace naming/numbering and switching workspaces, please see this issue comment: https://github.com/johnae/persway/issues/2#issuecomment-644343784 - the gist of it is that it is likely a sway config issue.


### Nix flake

If you happen to be on [NixOS](https://nixos.org) or you're using the Nix Package Manager, you can easily use the flake and overlay from this repo (provided you're using Nix flakes ofc).

Persway is released under the MIT license.
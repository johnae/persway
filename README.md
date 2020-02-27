## Persway - a simple sway ipc daemon

This is a small daemon that listens to sway events over an ipc socket. It sets the opacity of all non-focused windows the configured value. It will also set the workspace name dynamically to the name of the focused application. If `autolayout` is enabled (see command line options below), it will also alternate between horizontal / vertical splits, sort of like AwesomeWM.

Default opacity is `0.78` but can be changed via a command line parameter:

```
persway 0.3.1

USAGE:
    persway [FLAGS] [OPTIONS]

FLAGS:
    -a, --autolayout
    -h, --help          Prints help information
    -V, --version       Prints version information

OPTIONS:
    -o, --opacity <opacity>     [default: 0.78]
```

Persway is released under the MIT license.
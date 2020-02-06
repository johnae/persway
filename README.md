## Persway - super simple focus opacity changer for sway

This is a small daemon that listens to focus events over an ipc socket. It sets the opacity of all non-focused windows to less than 1. It will also set the workspace name dynamically to the name of the focused application.

Default opacity is `0.78` but can be changed via a command line parameter:

```
persway 0.3.0
USAGE:
    persway [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -o, --opacity <opacity>     [default: 0.78]
```

Persway is released under the MIT license.
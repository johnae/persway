## Persway - super simple focus opacity changer for sway

This is a small daemon that listens to focus events over an ipc socket. It basically just sets the opacity of all non-focused windows to less than 1.

Default opacity is `0.78` but can be changed via a command line parameter:

```
persway 0.1.0
USAGE:
    persway [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -o, --opacity <opacity>     [default: 0.78]
```

Persway is released under the MIT license.
# Changelog

## [Unreleased]
### Breaking changes
- Remove specific opacity options (eg. -o and -s)
- Add more general event handlers (-e and -f)

## [0.4.1]
- add option to skip opacity setting for certain windows (the -s option) (see: [#6](../../issues/6))
- fix rare crash - sometimes, rarely, there is no focused workspace according to sway (eg. wake from sleep or other such situations) - handle gracefully
- set default opacity to 1.0 which also means persway won't tell sway to make any opacity changes (see: [#5](../../issues/5))
- workspace renaming: reset workspace name to number when all windows are closed (see: [#4](../../issues/4))
- start keeping a Changelog
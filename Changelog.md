# Changelog

## [0.6.1]
### Changes
- The default layout which can be set when starting daemon is now accompanied by two settings specific to the `stack_main` layout:
  ```sh
    -s, --stack-main-default-size <STACK_MAIN_DEFAULT_SIZE>
          This controls the default size of the main area in the stack_main layout

          [default: 70]

    -k, --stack-main-default-stack-layout <STACK_MAIN_DEFAULT_STACK_LAYOUT>
          This controls the default sway layout of the stack area in the stack_main layout. Any of: tabbed, tiled or stacked

          [default: stacked]
  ```
- A bit of code cleanup and removal of unnecessary marks

## [0.6.0]
### Breaking changes
- All daemon commands are now under the subcommand `daemon`
- Remove specific opacity options (eg. -o and -s)
- Add more general event handlers (-e and -f)

### Features
- New layout: `stack_main` (sometimes referred to as `master stack` in some window managers)
- Persway now listens on a socket and is both a client of daemon contained in the same binary
- New commands related to layouts, layouts can now be changed on the fly via the persway client
  ie through the subcommands:
  ```sh
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
   ```

## [0.4.1]
- add option to skip opacity setting for certain windows (the -s option) (see: [#6](../../issues/6))
- fix rare crash - sometimes, rarely, there is no focused workspace according to sway (eg. wake from sleep or other such situations) - handle gracefully
- set default opacity to 1.0 which also means persway won't tell sway to make any opacity changes (see: [#5](../../issues/5))
- workspace renaming: reset workspace name to number when all windows are closed (see: [#4](../../issues/4))
- start keeping a Changelog

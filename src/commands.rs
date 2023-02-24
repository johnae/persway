use crate::layout::{StackLayout, WorkspaceLayout, STACK_MAIN_DEFAULT_SIZE};

#[derive(clap::Parser, Debug)]
pub struct DaemonArgs {
    /// Which layout should be the default when no other layout has been specified for
    /// a workspace. Options are: manual, spiral and stack_main.
    #[arg(long, short = 'd', default_value = "manual")]
    pub default_layout: WorkspaceLayout,

    /// This controls the default size of the main area in the stack_main layout.
    #[arg(long, short = 's', default_value_t = STACK_MAIN_DEFAULT_SIZE)]
    pub stack_main_default_size: u8,

    /// This controls the default sway layout of the stack area in the stack_main layout.
    /// Any of: tabbed, tiled or stacked
    #[arg(long, short = 'k', default_value_t = StackLayout::Stacked)]
    pub stack_main_default_stack_layout: StackLayout,

    /// Enable automatic workspace renaming based on what is running
    /// in the workspace (eg. application name).
    #[arg(long, short = 'w')]
    pub workspace_renaming: bool,

    /// Called when window comes into focus. To automatically set the opacity of
    /// all other windows to 0.8 for example, you would set this to:
    ///
    /// [tiling] opacity 0.8; opacity 1
    ///
    /// Eg. set all tiling windows to opacity 0.8 but set the currently focused window to opacity 1.
    /// Or if you want to skip some applications - in this case firefox - you would do something like:
    ///
    /// [tiling] opacity 0.8; [app_id="firefox"] opacity 1; opacity 1
    #[arg(long, short = 'f')]
    pub on_window_focus: Option<String>,

    /// Called when window leaves focus. To automatically mark these for example, you would set
    /// this to:
    ///
    /// mark --add _prev
    ///
    /// and then in your sway config:
    ///
    /// bindsym Mod1+tab [con_mark=_prev] focus
    #[arg(long, short = 'l')]
    pub on_window_focus_leave: Option<String>,

    /// Called when persway exits. This can be used to reset any opacity changes
    /// or other settings when persway exits. For example, if changing the opacity
    /// on window focus, you would probably want to reset that on exit like this:
    ///
    /// [tiling] opacity 1
    ///
    /// Eg. set all tiling windows to opacity 1
    #[arg(long, short = 'e')]
    pub on_exit: Option<String>,
}

#[derive(clap::Parser, Debug)]
pub enum PerswayCommand {
    /// Starts the persway daemon
    Daemon(DaemonArgs),
    /// Applies to stack main layout - focuses the next stacked window
    StackFocusNext,
    /// Applies to stack main layout - focuses the previous stacked window
    StackFocusPrev,
    /// Applies to stack main layout - swaps the current stacked window with the main window
    StackSwapMain,
    /// Applies to stack main layout - pops the top of the stack into main while pushing the old main window to the bottom of the stack
    StackMainRotateNext,
    /// Changes the layout of the focused workspace
    ChangeLayout {
        /// Change the layout of the focused workspace, can be any of:
        /// manual, spiral, stack_main
        #[command(subcommand)]
        layout: WorkspaceLayout,
    },
}

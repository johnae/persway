use crate::layout::WorkspaceLayout;

#[derive(clap::Parser, Debug)]
pub struct DaemonArgs {
    /// Which layout should be the default when no other layout has been specified for
    /// a workspace.
    #[arg(long, default_value = "manual")]
    pub default_layout: WorkspaceLayout,

    /// Enable automatic workspace renaming based on what is running
    /// in the workspace (eg. application name).
    #[arg(long)]
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
    #[arg(long)]
    pub on_window_focus: Option<String>,

    /// Called when window leaves focus. To automatically mark these for example, you would set
    /// this to:
    ///
    /// mark --add _prev
    ///
    /// and then in your sway config:
    ///
    /// bindsym Mod1+tab [con_mark=_prev] focus
    #[arg(long)]
    pub on_window_focus_leave: Option<String>,

    /// Called when persway exits. This can be used to reset any opacity changes
    /// or other settings when persway exits. For example, if changing the opacity
    /// on window focus, you would probably want to reset that on exit like this:
    ///
    /// [tiling] opacity 1
    ///
    /// Eg. set all tiling windows to opacity 1
    #[arg(long)]
    pub on_exit: Option<String>,
}

#[derive(clap::Parser, Debug)]
pub enum PerswayCommand {
    Daemon(DaemonArgs),
    StackFocusNext,
    StackFocusPrev,
    StackSwapVisible,
    StackMainRotateNext,
    StackMainRotatePrev,
    ChangeLayout {
        /// Change the layout of the focused workspace, can be any of:
        /// manual, spiral, stack_main
        #[arg(long)]
        layout: WorkspaceLayout,
    },
}

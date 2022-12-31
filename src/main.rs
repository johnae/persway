use anyhow::{anyhow, Result};
use async_std::{
    fs,
    io::{prelude::BufReadExt, BufReader},
    prelude::*,
};
use async_trait::async_trait;
use env_logger::Env;
use futures::{select, FutureExt};
use log::{debug, error, info, warn};
use signal_hook::consts::signal::*;
use signal_hook_async_std::Signals;
use std::{process::exit, str::FromStr};
use structopt::StructOpt;
use swayipc_async::{
    Connection, Event, EventType, NodeLayout, NodeType, WindowChange, WindowEvent, Workspace,
};
use swayipc_types::Node;

use nix::sys::stat;
use nix::unistd;

#[derive(StructOpt)]
/// I am Persway. A friendly daemon.
///
/// I talk to the Sway Compositor and persuade it to do little evil things.
/// Give me an option and see what it brings.
struct Cli {
    /// Enable different layouts on workspaces. The order in which they appear
    /// correspond to workspace numbers so - the first correspond to workspace 1,
    /// the second to workspace 2 etc.
    #[structopt(short = "a", long = "auto_layout")]
    auto_layout: Option<Vec<WorkspaceLayout>>,
    /// Which layout should be the default when no other layout has been specified for
    /// a workspace.
    #[structopt(short = "d", long = "default_layout", default_value = "manual")]
    default_layout: WorkspaceLayout,
    /// Enable automatic workspace renaming based on what is running
    /// in the workspace (eg. application name).
    #[structopt(short = "w", long = "workspace-renaming")]
    workspace_renaming: bool,
    /// Automatically mark all windows with their container id
    /// In a stacked layout for example, marks show up on the right side
    /// in the titlebar. This way, you can stack windows on one screen and
    /// swap them in on a primary monitor with a certain layout at will.
    #[structopt(short = "m", long = "mark-new-windows")]
    mark_new_windows: bool,
    /// Called when window comes into focus. To automatically set the opacity of
    /// all other windows to 0.8 for example, you would set this to:
    ///
    /// [tiling] opacity 0.8; opacity 1
    ///
    /// Eg. set all tiling windows to opacity 0.8 but set the currently focused window to opacity 1.
    /// Or if you want to skip some applications - in this case firefox - you would do something like:
    ///
    /// [tiling] opacity 0.8; [app_id="firefox"] opacity 1; opacity 1
    #[structopt(short = "f", long = "on-window-focus")]
    on_window_focus: Option<String>,
    /// Called when window leaves focus. To automatically mark these for example, you would set
    /// this to:
    ///
    /// mark --add _prev
    ///
    /// and then in your sway config:
    ///
    /// bindsym Mod1+tab [con_mark=_prev] focus
    #[structopt(short = "l", long = "on-window-focus-leave")]
    on_window_focus_leave: Option<String>,
    /// Called when persway exits. This can be used to reset any opacity changes
    /// or other settings when persway exits. For example, if changing the opacity
    /// on window focus, you would probably want to reset that on exit like this:
    ///
    /// [tiling] opacity 1
    ///
    /// Eg. set all tiling windows to opacity 1
    #[structopt(short = "e", long = "on-exit")]
    on_exit: Option<String>,
    /// Path to control socket.
    socket_path: Option<String>, // TODO: make this a socket instead
}

impl FromStr for WorkspaceLayout {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "spiral" => Ok(Self::Spiral),
            "master_stack" => Ok(Self::MasterStack),
            "manual" => Ok(Self::Manual),
            _ => Err(anyhow!("I don't know about the layout '{}'", s)),
        }
    }
}

#[derive(Debug)]
pub enum WorkspaceLayout {
    Spiral,
    MasterStack,
    Manual,
}

#[derive(Debug)]
pub enum PerswayCommand {
    StackFocusNext,
    StackFocusPrev,
    SwapVisible,
    MasterCycleNext,
    MasterCyclePrev,
}

impl FromStr for PerswayCommand {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self> {
        let mut parts = s.split(" ");
        match parts.next() {
            Some("stack_focus_next") => Ok(PerswayCommand::StackFocusNext),
            Some("stack_focus_prev") => Ok(PerswayCommand::StackFocusPrev),
            Some("swap_visible") => Ok(PerswayCommand::SwapVisible),
            Some("master_cycle_next") => Ok(PerswayCommand::MasterCycleNext),
            Some("master_cycle_prev") => Ok(PerswayCommand::MasterCyclePrev),
            _ => Err(anyhow!("Don't know that one, sorry")),
        }
    }
}

trait IntoLinearNodeIterator {
    fn into_linear_iter(&self) -> LinearNodeIterator;
}

impl<'a> IntoLinearNodeIterator for &'a Node {
    fn into_linear_iter(&self) -> LinearNodeIterator {
        LinearNodeIterator::new(self)
    }
}

#[derive(Clone)]
struct LinearNodeIterator<'a> {
    stack: Vec<&'a Node>,
}

impl<'a> LinearNodeIterator<'a> {
    fn new(root: &'a Node) -> LinearNodeIterator<'a> {
        let mut stack = Vec::with_capacity(100);
        stack.push(root);
        LinearNodeIterator { stack }
    }
}

impl<'a> Iterator for LinearNodeIterator<'a> {
    type Item = &'a Node;

    fn next(&mut self) -> Option<Self::Item> {
        let node = self.stack.pop()?;
        for entry in node.nodes.iter().rev() {
            self.stack.push(&*entry);
        }
        Some(node)
    }
}

async fn get_focused_workspace(connection: &mut Connection) -> Result<Workspace> {
    let mut ws = connection.get_workspaces().await?.into_iter();
    ws.find(|w| w.focused)
        .ok_or_else(|| anyhow!("No focused workspace"))
}

async fn get_node_workspace(connection: &mut Connection, node_id: i64) -> Result<Workspace> {
    let tree = connection.get_tree().await?;
    if let Some(wstree) = tree.find_as_ref(|n| {
        n.node_type == NodeType::Workspace && n.find_as_ref(|n| n.id == node_id).is_some()
    }) {
        let mut ws = connection.get_workspaces().await?.into_iter();
        return ws
            .find(|w| w.id == wstree.id)
            .ok_or_else(|| anyhow!("No node workspace :-|"));
    }
    Err(anyhow!("Couldn't find the node workspace"))
}

fn get_stack_mark(id: i64) -> String {
    format!("_stack_{}", id)
}

fn get_master_mark(id: i64) -> String {
    format!("_master_{}", id)
}

#[async_trait]
trait WindowEventHandler {
    async fn handle(&mut self, event: &Box<WindowEvent>);
}

struct WorkspaceSpiralLayoutHandler<'a> {
    connection: &'a mut Connection,
    auto_layout: &'a Option<Vec<WorkspaceLayout>>,
    default_layout: &'a WorkspaceLayout,
}

impl<'a> WorkspaceSpiralLayoutHandler<'a> {
    fn new(
        connection: &'a mut Connection,
        auto_layout: &'a Option<Vec<WorkspaceLayout>>,
        default_layout: &'a WorkspaceLayout,
    ) -> Self {
        Self {
            connection,
            auto_layout,
            default_layout,
        }
    }

    async fn layout(&mut self) -> Result<()> {
        let tree = self.connection.get_tree().await?;
        let ws = get_focused_workspace(&mut self.connection).await?;
        let wslayout = if let Some(auto_layout) = self.auto_layout {
            auto_layout
                .get((ws.num - 1) as usize)
                .unwrap_or_else(|| self.default_layout)
        } else {
            self.default_layout
        };
        if !matches!(*wslayout, WorkspaceLayout::Spiral) {
            debug!(
                "skip spiral handler: {:?} doesn't match {:?}",
                *wslayout,
                WorkspaceLayout::Spiral
            );
            return Ok(());
        }
        let focused = tree
            .find_focused_as_ref(|n| n.focused)
            .ok_or_else(|| anyhow!("No focused node"))?;
        let parent = tree
            .find_focused_as_ref(|n| n.nodes.iter().any(|n| n.focused))
            .ok_or_else(|| anyhow!("No parent"))?;
        let is_floating = focused.node_type == NodeType::FloatingCon;
        let is_full_screen = focused.percent.unwrap_or(1.0) > 1.0;
        let is_stacked = parent.layout == NodeLayout::Stacked;
        let is_tabbed = parent.layout == NodeLayout::Tabbed;
        if !is_floating && !is_full_screen && !is_stacked && !is_tabbed {
            let cmd = if focused.rect.height > focused.rect.width {
                "split v"
            } else {
                "split h"
            };
            self.connection.run_command(cmd).await?;
        };

        Ok(())
    }
}
#[async_trait]
impl WindowEventHandler for WorkspaceSpiralLayoutHandler<'_> {
    async fn handle(&mut self, event: &Box<WindowEvent>) {
        match event.change {
            WindowChange::Focus => {
                if let Err(e) = self.layout().await {
                    error!("spiral layout handler, layout err: {}", e);
                };
            }
            _ => {}
        }
    }
}

struct WorkspaceMasterStackLayoutHandler<'a> {
    connection: &'a mut Connection,
    auto_layout: &'a Option<Vec<WorkspaceLayout>>,
    default_layout: &'a WorkspaceLayout,
}

impl<'a> WorkspaceMasterStackLayoutHandler<'a> {
    fn new(
        connection: &'a mut Connection,
        auto_layout: &'a Option<Vec<WorkspaceLayout>>,
        default_layout: &'a WorkspaceLayout,
    ) -> Self {
        Self {
            connection,
            auto_layout,
            default_layout,
        }
    }

    async fn on_new_window(
        &mut self,
        event: &WindowEvent,
    ) -> Result<Vec<Result<(), swayipc_async::Error>>> {
        let tree = self.connection.get_tree().await?;
        let ws = get_node_workspace(&mut self.connection, event.container.id).await?;
        let wstree = tree.find_as_ref(|n| n.id == ws.id).unwrap();
        debug!("new_window: {:?}", event.container.id);
        debug!("nodes_len: {}", wstree.nodes.len());
        let wslayout = if let Some(auto_layout) = self.auto_layout.as_ref() {
            auto_layout
                .get((ws.num - 1) as usize)
                .unwrap_or_else(|| self.default_layout)
        } else {
            self.default_layout
        };

        if !matches!(*wslayout, WorkspaceLayout::MasterStack) {
            return Ok(Vec::new());
        }
        match wstree.nodes.len() {
            1 => {
                let master_mark = get_master_mark(ws.id);
                let cmd = format!(
                    "[con_mark={}] unmark {}; split h; [con_id={}] mark --add {}",
                    master_mark, master_mark, event.container.id, master_mark
                );
                Ok(self.connection.run_command(cmd).await?)
            }
            2 => {
                let stack_mark = get_stack_mark(ws.id);
                let master_mark = get_master_mark(ws.id);
                let master = wstree
                    .into_linear_iter()
                    .filter(|n| n.nodes.len() == 0 && n.node_type == NodeType::Con)
                    .max_by(|x, y| x.rect.x.cmp(&y.rect.x))
                    .expect("master node should be found");
                let stack = wstree
                    .nodes
                    .iter()
                    .filter(|n| n.id != master.id)
                    .next()
                    .expect("Couldn't determine which node is the stack, none found");
                let stacked = stack.layout == NodeLayout::Stacked;

                let cmd = if !stacked {
                    format!(
                      "[con_id={}] mark --add {}; [con_id={}] focus; split v; layout stacking; resize set width 25; [con_id={}] focus parent; mark --add {}; [con_id={}] focus",
                      master.id, master_mark, stack.id, stack.id, stack_mark, master.id
                    )
                } else {
                    format!(
                        "[con_id={}] mark --add {}; [con_id={}] mark --add {}",
                        stack.id, stack_mark, master.id, master_mark
                    )
                };

                Ok(self.connection.run_command(cmd).await?)
            }
            3 => {
                let master = wstree
                    .nodes
                    .iter()
                    .filter(|n| {
                        n.nodes.len() == 0
                            && n.node_type == NodeType::Con
                            && n.id != event.container.id
                    })
                    .max_by(|x, y| x.rect.x.cmp(&y.rect.x))
                    .unwrap();

                let stack = wstree
                    .nodes
                    .iter()
                    .filter(|n| n.id != master.id)
                    .next()
                    .unwrap();

                let stack_mark = get_stack_mark(ws.id);
                let master_mark = get_master_mark(ws.id);

                let cmd = format!(
                          "[con_mark={}] unmark {}; [con_mark={}] unmark {}; [con_id={}] mark --add {}; [con_id={}] mark --add {}; [con_id={}] focus; move container to mark {}; [con_id={}] focus; swap container with con_id {}; [con_id={}] focus",
                          stack_mark, stack_mark, master_mark, master_mark, stack.id, stack_mark, event.container.id, master_mark,
                          event.container.id, stack_mark,
                          master.id, event.container.id, event.container.id
                );

                debug!("new_window: {}", cmd);

                Ok(self.connection.run_command(cmd).await?)
            }
            _ => Ok(Vec::new()),
        }
    }
    async fn on_close_window(
        &mut self,
        event: &WindowEvent,
    ) -> Result<Vec<Result<(), swayipc_async::Error>>> {
        let tree = self.connection.get_tree().await?;
        let ws = get_focused_workspace(&mut self.connection).await?;
        let wstree = tree.find_as_ref(|n| n.id == ws.id).unwrap();
        let master_mark = get_master_mark(ws.id);

        let wslayout = if let Some(auto_layout) = self.auto_layout.as_ref() {
            auto_layout
                .get((ws.num - 1) as usize)
                .unwrap_or_else(|| self.default_layout)
        } else {
            self.default_layout
        };

        if !matches!(*wslayout, WorkspaceLayout::MasterStack) {
            return Ok(Vec::new());
        }

        if event.container.marks.contains(&master_mark) {
            if let Some(stack) = wstree
                .nodes
                .iter()
                .filter(|n| n.id != event.container.id)
                .next()
            {
                let stack_visible = stack
                    .find_as_ref(|n| n.visible.is_some() && n.visible.unwrap() == true)
                    .unwrap();
                let cmd = if wstree
                    .into_linear_iter()
                    .filter(|n| n.nodes.len() == 0)
                    .count()
                    == 1
                {
                    debug!("count is 1..., stack_visible: {:?}", stack_visible);
                    format!(
                        "[con_id={}] focus; layout splith; move up; [con_id={}] mark --add {}",
                        stack_visible.id, stack_visible.id, master_mark
                    )
                } else {
                    debug!("count is more than 1...");
                    format!(
                      "[con_id={}] focus; move right; resize set width 75; [con_id={}] mark --add {}",
                      stack_visible.id, stack_visible.id, master_mark
                    )
                };
                debug!("close_window: {}", cmd);
                return Ok(self.connection.run_command(cmd).await?);
            }
        }
        Ok(Vec::new())
    }
    async fn on_move_window(
        &mut self,
        event: &WindowEvent,
    ) -> Result<Vec<Result<(), swayipc_async::Error>>> {
        let ws = get_node_workspace(&mut self.connection, event.container.id).await?;
        let focused_ws = get_focused_workspace(&mut self.connection).await?;

        if ws.id == focused_ws.id {
            debug!("move_window within workspace: {:?}", event);
            return self.on_new_window(event).await;
        } else {
            debug!("move_window to other workspace: {:?}", event);
            self.on_new_window(event).await?;
            self.on_close_window(event).await
        }
    }
}
#[async_trait]
impl WindowEventHandler for WorkspaceMasterStackLayoutHandler<'_> {
    async fn handle(&mut self, event: &Box<WindowEvent>) {
        match event.change {
            WindowChange::New => {
                if let Err(e) = self.on_new_window(&event).await {
                    error!("master/stack layout err: {}", e);
                };
            }
            WindowChange::Close => {
                if let Err(e) = self.on_close_window(&event).await {
                    error!("master/stack layout err: {}", e);
                };
            }
            WindowChange::Move => {
                if let Err(e) = self.on_move_window(&event).await {
                    error!("master/stack layout err: {}", e);
                };
            }
            _ => {
                debug!("MasterStackHandler, not handling event: {:?}", event.change);
            }
        }
    }
}

struct WorkspaceRenamingHandler<'a> {
    connection: &'a mut Connection,
}

impl<'a> WorkspaceRenamingHandler<'a> {
    fn new(connection: &'a mut Connection) -> Self {
        Self { connection }
    }

    async fn rename_workspace(&mut self, event: &WindowEvent) -> Result<()> {
        let current_ws = get_focused_workspace(&mut self.connection).await?;
        let ws_num = current_ws
            .name
            .split(':')
            .next()
            .unwrap_or(&current_ws.name);

        if current_ws.focus.is_empty() {
            let cmd = format!("rename workspace to {}", ws_num);
            debug!("{}", cmd);
            self.connection.run_command(&cmd).await?;
            return Ok(());
        }

        let app_id =
            event
                .container
                .app_id
                .as_ref()
                .and_then(|id| if id.is_empty() { None } else { Some(id) });

        let name = event.container.name.as_ref().and_then(|name| {
            if name.is_empty() {
                None
            } else {
                name.split('|').next().map(|s| s.to_string())
            }
        });

        let class = event.container.window_properties.as_ref().and_then(|p| {
            p.class
                .as_ref()
                .and_then(|class| if class.is_empty() { None } else { Some(class) })
        });

        let app_name = app_id.or(class);
        let app_name = app_name.or(name.as_ref());
        let app_name = app_name.map(|n| {
            n.trim_start_matches('-')
                .trim_end_matches('-')
                .trim_end_matches(' ')
                .to_lowercase()
        });

        if let Some(app_name) = app_name {
            let newname = format!("{}: {}", ws_num, app_name);
            let cmd = format!("rename workspace to {}", newname);
            debug!("{}", cmd);
            self.connection.run_command(&cmd).await?;
        };
        Ok(())
    }
}
#[async_trait]
impl WindowEventHandler for WorkspaceRenamingHandler<'_> {
    async fn handle(&mut self, event: &Box<WindowEvent>) {
        match event.change {
            WindowChange::Focus => {
                if let Err(e) = self.rename_workspace(event).await {
                    error!("workspace rename err: {}", e);
                }
            }
            WindowChange::Close => {
                if let Err(e) = self.rename_workspace(event).await {
                    error!("workspace rename err: {}", e);
                }
            }

            _ => {}
        }
    }
}

struct NewWindowMarkerHandler<'a> {
    connection: &'a mut Connection,
}

impl<'a> NewWindowMarkerHandler<'a> {
    fn new(connection: &'a mut Connection) -> Self {
        Self { connection }
    }
}

#[async_trait]
impl WindowEventHandler for NewWindowMarkerHandler<'_> {
    async fn handle(&mut self, event: &Box<WindowEvent>) {
        match event.change {
            WindowChange::New => {
                if let Err(e) = self
                    .connection
                    .run_command(format!(
                        "[con_mark=_new] unmark _new; [con_id={}] mark --add _new",
                        event.container.id
                    ))
                    .await
                {
                    error!("error marking new window: {}", e);
                }
            }
            _ => {}
        }
    }
}

struct WindowFocusCommandHandler<'a> {
    connection: &'a mut Connection,
    previously_focused_id: Option<i64>,
    window_focus_cmd: Option<&'a String>,
    window_focus_leave_cmd: Option<&'a String>,
}

impl<'a> WindowFocusCommandHandler<'a> {
    fn new(
        connection: &'a mut Connection,
        window_focus_cmd: Option<&'a String>,
        window_focus_leave_cmd: Option<&'a String>,
    ) -> Self {
        Self {
            connection,
            window_focus_cmd,
            window_focus_leave_cmd,
            previously_focused_id: None,
        }
    }
}

#[async_trait]
impl WindowEventHandler for WindowFocusCommandHandler<'_> {
    async fn handle(&mut self, event: &Box<WindowEvent>) {
        match event.change {
            WindowChange::Focus => {
                if let Some(window_focus_leave_cmd) = &self.window_focus_leave_cmd {
                    if let Some(id) = self.previously_focused_id {
                        if let Err(e) = self
                            .connection
                            .run_command(format!("[con_id={id}] {}", window_focus_leave_cmd))
                            .await
                        {
                            error!("error running focus leave cmd: {}", e);
                        }
                    }
                }
                if let Some(window_focus_cmd) = &self.window_focus_cmd {
                    if let Err(e) = self.connection.run_command(window_focus_cmd).await {
                        error!("error running focus cmd: {}", e);
                    }
                }
                self.previously_focused_id = Some(event.container.id);
            }
            WindowChange::Close => {
                // run focus leave hook
                if let Some(window_focus_leave_cmd) = &self.window_focus_leave_cmd {
                    if let Some(id) = self.previously_focused_id {
                        if let Err(e) = self
                            .connection
                            .run_command(format!("[con_id={id}] {}", window_focus_leave_cmd))
                            .await
                        {
                            error!("error running focus leave cmd: {}", e);
                        }
                    }
                }
                self.previously_focused_id = None;
            }
            _ => {}
        }
    }
}

struct MasterStackController<'a> {
    connection: &'a mut Connection,
    auto_layout: &'a Option<Vec<WorkspaceLayout>>,
    default_layout: &'a WorkspaceLayout,
}

impl<'a> MasterStackController<'a> {
    fn new(
        connection: &'a mut Connection,
        auto_layout: &'a Option<Vec<WorkspaceLayout>>,
        default_layout: &'a WorkspaceLayout,
    ) -> Self {
        Self {
            connection,
            auto_layout,
            default_layout,
        }
    }

    async fn stack_focus_prev(&mut self) -> Result<Vec<Result<(), swayipc_async::Error>>> {
        let tree = self.connection.get_tree().await?;
        let ws = get_focused_workspace(&mut self.connection).await?;
        let wstree = tree.find_as_ref(|n| n.id == ws.id).unwrap();
        let stack_mark = get_stack_mark(ws.id);
        let master_mark = get_master_mark(ws.id);
        let wslayout = if let Some(auto_layout) = self.auto_layout.as_ref() {
            auto_layout
                .get((ws.num - 1) as usize)
                .unwrap_or_else(|| self.default_layout)
        } else {
            self.default_layout
        };

        if !matches!(*wslayout, WorkspaceLayout::MasterStack) {
            return Ok(Vec::new());
        }
        if let Some(stack) = wstree.find_as_ref(|n| n.marks.contains(&stack_mark)) {
            if stack.nodes.len() == 0 {
                return Ok(Vec::new());
            }
            let stack_visible = stack
                .find_as_ref(|n| n.visible.is_some() && n.visible.unwrap() == true)
                .unwrap();
            let mut prev_was_visible = false;
            for node in stack.nodes.iter().rev().cycle() {
                if prev_was_visible {
                    let cmd = format!(
                        "[con_id={}] focus; [con_mark={}] focus",
                        node.id, master_mark
                    );
                    debug!("{}", cmd);
                    return Ok(self.connection.run_command(cmd).await?);
                }
                prev_was_visible = node.id == stack_visible.id
            }
        }
        Ok(Vec::new())
    }

    async fn stack_focus_next(&mut self) -> Result<Vec<Result<(), swayipc_async::Error>>> {
        let tree = self.connection.get_tree().await?;
        let ws = get_focused_workspace(&mut self.connection).await?;
        let wstree = tree.find_as_ref(|n| n.id == ws.id).unwrap();
        let stack_mark = get_stack_mark(ws.id);
        let master_mark = get_master_mark(ws.id);

        let wslayout = if let Some(auto_layout) = self.auto_layout.as_ref() {
            auto_layout
                .get((ws.num - 1) as usize)
                .unwrap_or_else(|| self.default_layout)
        } else {
            self.default_layout
        };

        if !matches!(*wslayout, WorkspaceLayout::MasterStack) {
            return Ok(Vec::new());
        }

        if let Some(stack) = wstree.find_as_ref(|n| n.marks.contains(&stack_mark)) {
            if stack.nodes.len() == 0 {
                return Ok(Vec::new());
            }
            let stack_visible = stack
                .find_as_ref(|n| n.visible.is_some() && n.visible.unwrap() == true)
                .unwrap();
            let mut prev_was_visible = false;
            for node in stack.nodes.iter().cycle() {
                if prev_was_visible {
                    let cmd = format!(
                        "[con_id={}] focus; [con_mark={}] focus",
                        node.id, master_mark
                    );
                    debug!("{}", cmd);
                    return Ok(self.connection.run_command(cmd).await?);
                }
                prev_was_visible = node.id == stack_visible.id
            }
        }
        Ok(Vec::new())
    }

    async fn master_cycle_next(&mut self) -> Result<Vec<Result<(), swayipc_async::Error>>> {
        let tree = self.connection.get_tree().await?;
        let ws = get_focused_workspace(&mut self.connection).await?;
        let wstree = tree.find_as_ref(|n| n.id == ws.id).unwrap();
        let stack_mark = get_stack_mark(ws.id);
        let master_mark = get_master_mark(ws.id);

        let wslayout = if let Some(auto_layout) = self.auto_layout.as_ref() {
            auto_layout
                .get((ws.num - 1) as usize)
                .unwrap_or_else(|| self.default_layout)
        } else {
            self.default_layout
        };

        if !matches!(*wslayout, WorkspaceLayout::MasterStack) {
            return Ok(Vec::new());
        }

        if let Some(stack) = wstree.find_as_ref(|n| n.marks.contains(&stack_mark)) {
            if stack.nodes.len() == 0 {
                return Ok(Vec::new());
            }

            let stack_leaves = stack
                .into_linear_iter()
                .filter(|n| n.nodes.len() == 0 && n.node_type == NodeType::Con);

            let mut stack_leaves_next = stack_leaves.clone();
            stack_leaves_next.next();

            let mut veccmd = Vec::new();
            for node in stack_leaves {
                if let Some(next) = stack_leaves_next.next() {
                    let cmd = format!(
                        "[con_id={}] focus; swap container with con_id {}; ",
                        node.id, next.id
                    );
                    debug!("veccmd.push: {}", cmd);
                    veccmd.push(cmd);
                } else {
                    break;
                }
            }
            veccmd.push(format!(
                "[con_id={}] focus; [con_mark={}] focus; ",
                stack.nodes.last().unwrap().id,
                master_mark
            ));
            let cmd = veccmd.join("");
            debug!("master_cycle_next: {}", cmd);
            self.connection.run_command(cmd).await?;

            let tree = self.connection.get_tree().await?;
            let wstree = tree.find_as_ref(|n| n.id == ws.id).unwrap();
            let stack = wstree
                .find_as_ref(|n| n.marks.contains(&stack_mark))
                .unwrap();

            let master = wstree
                .into_linear_iter()
                .filter(|n| n.nodes.len() == 0 && n.node_type == NodeType::Con)
                .max_by(|x, y| x.rect.x.cmp(&y.rect.x))
                .expect("master node should be found");
            let stack_first = stack
                .into_linear_iter()
                .filter(|n| n.nodes.len() == 0 && n.node_type == NodeType::Con)
                .map(|n| n.id)
                .collect::<Vec<_>>()
                .into_iter()
                .next()
                .unwrap();

            let cmd = format!(
                "[con_id={}] focus; swap container with con_id {}; [con_id={}] focus; [con_mark={}] unmark {}; [con_id={}] mark --add {}",
                master.id, stack_first, stack_first,
                master_mark, master_mark, stack_first, master_mark
            );
            debug!("{}", cmd);
            return Ok(self.connection.run_command(cmd).await?);
        }
        Ok(Vec::new())
    }

    async fn swap_visible(&mut self) -> Result<Vec<Result<(), swayipc_async::Error>>> {
        let tree = self.connection.get_tree().await?;
        let ws = get_focused_workspace(&mut self.connection).await?;
        let wstree = tree.find_as_ref(|n| n.id == ws.id).unwrap();
        let stack_mark = get_stack_mark(ws.id);
        let master_mark = get_master_mark(ws.id);

        let wslayout = if let Some(auto_layout) = self.auto_layout.as_ref() {
            auto_layout
                .get((ws.num - 1) as usize)
                .unwrap_or_else(|| self.default_layout)
        } else {
            self.default_layout
        };

        if !matches!(*wslayout, WorkspaceLayout::MasterStack) {
            return Ok(Vec::new());
        }

        if let Some(stack) = wstree.find_as_ref(|n| n.marks.contains(&stack_mark)) {
            if stack.nodes.len() == 0 {
                return Ok(Vec::new());
            }
            let master = wstree
                .find_as_ref(|n| n.marks.contains(&master_mark))
                .expect("A master node to exist");
            let stack_visible = stack
                .find_as_ref(|n| {
                    n.nodes.len() == 0 && n.visible.is_some() && n.visible.unwrap() == true
                })
                .expect("Stack to contain a visible node");
            let cmd = format!(
                "[con_id={}] focus; swap container with con_id {}; [con_id={}] focus; [con_mark={}] unmark {}; [con_id={}] mark --add {}",
                master.id, stack_visible.id, stack_visible.id, master_mark, master_mark, stack_visible.id, master_mark
            );
            debug!("{}", cmd);
            return Ok(self.connection.run_command(cmd).await?);
        }
        Ok(Vec::new())
    }
}

async fn handle_signals(signals: Signals) {
    let mut signals = signals.fuse();
    let args = Cli::from_args();
    let on_exit = args.on_exit;
    while let Some(signal) = signals.next().await {
        match signal {
            SIGHUP | SIGINT | SIGQUIT | SIGTERM => {
                let mut commands = Connection::new().await.unwrap();
                if let Some(exit_cmd) = on_exit {
                    debug!("{}", exit_cmd);
                    commands.run_command(exit_cmd).await.unwrap();
                }
                exit(0)
            }
            _ => unreachable!(),
        }
    }
}

#[async_std::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let args = Cli::from_args();
    let on_window_focus = args.on_window_focus;
    let on_window_focus_leave = args.on_window_focus_leave;

    let signals = Signals::new(&[SIGHUP, SIGINT, SIGQUIT, SIGTERM])?;
    let handle = signals.handle();
    let signals_task = async_std::task::spawn(handle_signals(signals));

    let subs = [EventType::Window];
    let mut events = Connection::new().await?.subscribe(&subs).await?;

    let mut conn = Connection::new().await?;
    let window_focus_command_handler = WindowFocusCommandHandler::new(
        &mut conn,
        on_window_focus.as_ref(),
        on_window_focus_leave.as_ref(),
    );

    let mut conn = Connection::new().await?;
    let ws_spiral_layout_handler =
        WorkspaceSpiralLayoutHandler::new(&mut conn, &args.auto_layout, &args.default_layout);

    let mut conn = Connection::new().await?;
    let new_window_marker_handler = NewWindowMarkerHandler::new(&mut conn);

    let mut conn = Connection::new().await?;
    let ws_renaming_handler = WorkspaceRenamingHandler::new(&mut conn);

    let mut conn = Connection::new().await?;
    let ws_master_stack_handler =
        WorkspaceMasterStackLayoutHandler::new(&mut conn, &args.auto_layout, &args.default_layout);

    let mut window_handlers: Vec<Box<dyn WindowEventHandler>> = Vec::new();

    window_handlers.push(Box::new(ws_spiral_layout_handler));
    window_handlers.push(Box::new(ws_master_stack_handler));
    window_handlers.push(Box::new(window_focus_command_handler));

    if args.mark_new_windows {
        window_handlers.push(Box::new(new_window_marker_handler));
    }

    if args.workspace_renaming {
        window_handlers.push(Box::new(ws_renaming_handler));
    }

    let persway_input_file = args
        .socket_path
        .unwrap_or_else(|| String::from("/run/user/1337/persway"));

    let mkpipe = |name: &str| {
        if let Err(e) = std::fs::remove_file(name) {
            warn!("couldn't remove named pipe '{}': {}", name, e);
        }
        match unistd::mkfifo(name, stat::Mode::S_IRWXU) {
            Ok(_) => info!("created fifo {}", name),
            Err(err) => {
                if err == nix::errno::Errno::EEXIST {
                    warn!("{} pipe already present", name);
                } else {
                    panic!("couldn't create named pipe {}", name);
                }
            }
        };
    };

    mkpipe(&persway_input_file);

    let inputpipe = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(persway_input_file)
        .await?;

    let mut input = BufReader::new(&inputpipe);

    let mut conn = Connection::new().await?;
    let mut master_stack_controller =
        MasterStackController::new(&mut conn, &args.auto_layout, &args.default_layout);

    loop {
        let mut inputdata = String::from("");
        select! {
            event = events.next().fuse() => {
                if let Some(event) = event {
                  match event? {
                      Event::Window(event) => {
                          for handler in window_handlers.iter_mut() {
                              handler.handle(&event).await;
                          }
                      }
                      _ => unreachable!(),
                  }
                }
            }
            _ = input.read_line(&mut inputdata).fuse() => {
                match inputdata.trim().parse::<PerswayCommand>() {
                    Ok(cmd) => {
                        match cmd {
                            PerswayCommand::StackFocusNext => if let Err(e) = master_stack_controller.stack_focus_next().await {
                                error!("stack focus next failed: {}", e)
                            },
                            PerswayCommand::StackFocusPrev => if let Err(e) = master_stack_controller.stack_focus_prev().await {
                                error!("stack focus prev failed: {}", e)
                            },
                            PerswayCommand::SwapVisible => if let Err(e) = master_stack_controller.swap_visible().await {
                                error!("swap visible failed: {}", e)
                            }
                            PerswayCommand::MasterCycleNext => if let Err(e) = master_stack_controller.master_cycle_next().await {
                                error!("swap visible failed: {}", e)
                            }
                            _ => debug!("skipping command: {:?}", cmd)
                        }
                    },
                    Err(e) => error!("oops: {}", e),
                }
            }
            complete => break,
        }
    }

    handle.close();
    signals_task.await;
    Ok(())
}

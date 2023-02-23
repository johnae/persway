use anyhow::Result;
use env_logger::Env;
mod client;
mod commands;
mod layout;
mod node_ext;
mod server;
use clap::Parser;
mod utils;

#[derive(Parser, Debug)]
#[clap(about, version, author)]
/// I am Persway. An evil, scheming, friendly daemon.
///
/// I talk to the Sway Compositor and persuade it to do little evil things.
/// Give me an option and see what it brings. I also talk to myself.
struct Args {
    #[command(subcommand)]
    command: commands::PerswayCommand,
    /// Path to control socket. This option applies both to daemon and client.
    /// Defaults to <XDG_RUNTIME_DIR>/persway-<WAYLAND_DISPLAY>.sock
    #[arg(long, short = 's')]
    socket_path: Option<String>,
}

#[async_std::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let args = Args::parse();
    match args.command {
        commands::PerswayCommand::Daemon(daemon_args) => {
            server::daemon::Daemon::new(daemon_args, args.socket_path)
                .run()
                .await?
        }
        _ => {
            log::debug!("command: {:?}", args.command);
            client::send(
                args.socket_path,
                &std::env::args().into_iter().collect::<Vec<_>>().join(" "),
            )
            .await?
        }
    }
    Ok(())
}

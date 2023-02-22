use std::net::Shutdown;
use std::str;

use crate::utils;
use anyhow::Result;
use async_std::prelude::*;
use async_std::{io::ReadExt, os::unix::net::UnixStream};

pub async fn send(socket_path: Option<String>, msg: &str) -> Result<()> {
    log::debug!("sending message: '{}'", msg);
    let socket_path = utils::get_socket_path(socket_path);
    let mut stream = UnixStream::connect(&socket_path).await?;
    stream.write_all(msg.as_bytes()).await?;
    stream.shutdown(Shutdown::Write)?;
    let mut response = String::new();
    stream.read_to_string(&mut response).await?;
    stream.shutdown(Shutdown::Read)?;
    log::info!("-> {}", response);
    Ok(())
}

use std::{
    io,
    net::IpAddr,
    num::NonZeroU16,
    path::PathBuf,
    time::{Duration, Instant},
};

use bytes::{Bytes, BytesMut};
use futures::{
    channel::mpsc::{self, Receiver, Sender},
    SinkExt, Stream,
};
use irc::{connection, proto::command, BytesCodec, Connection};
use sha2::{Digest, Sha256};
use thiserror::Error;
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt},
    task::JoinHandle,
    time,
};
use tokio_stream::StreamExt;

use super::Id;
use crate::{dcc, server, user::Nick};

/// 16 KiB
pub const BUFFER_SIZE: usize = 16 * 1024;

pub struct Handle {
    sender: Sender<Action>,
    task: JoinHandle<()>,
}

impl Handle {
    pub fn approve(&mut self, save_to: PathBuf) {
        let _ = self.sender.try_send(Action::Approve { save_to });
    }

    pub fn confirm_reverse(&mut self, host: IpAddr, port: NonZeroU16) {
        let _ = self
            .sender
            .try_send(Action::ReverseConfirmed { host, port });
    }

    pub fn port_available(&mut self, port: NonZeroU16) {
        let _ = self.sender.try_send(Action::PortAvailable { port });
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        self.task.abort();
    }
}

pub enum Task {
    Receive {
        id: Id,
        dcc_send: dcc::Send,
        server_handle: server::Handle,
        remote_user: Nick,
    },
    Send {
        id: Id,
        path: PathBuf,
        sanitized_filename: String,
        remote_user: Nick,
        reverse: bool,
        server_handle: server::Handle,
    },
}

impl Task {
    pub fn receive(
        id: Id,
        dcc_send: dcc::Send,
        remote_user: Nick,
        server_handle: server::Handle,
    ) -> Self {
        Self::Receive {
            id,
            dcc_send,
            remote_user,
            server_handle,
        }
    }

    pub fn send(
        id: Id,
        path: PathBuf,
        sanitized_filename: String,
        remote_user: Nick,
        reverse: bool,
        server_handle: server::Handle,
    ) -> Self {
        Self::Send {
            id,
            path,
            sanitized_filename,
            remote_user,
            reverse,
            server_handle,
        }
    }

    pub fn spawn(
        self,
        server: Option<Server>,
        timeout: Duration,
    ) -> (Handle, impl Stream<Item = Update>) {
        let (action_sender, action_receiver) = mpsc::channel(1);
        let (update_sender, update_receiver) = mpsc::channel(100);

        let task = tokio::spawn(async move {
            let mut update = update_sender.clone();

            match self {
                Task::Receive {
                    id,
                    dcc_send,
                    remote_user,
                    server_handle,
                } => {
                    if let Err(error) = receive(
                        id,
                        dcc_send,
                        remote_user,
                        server_handle,
                        action_receiver,
                        update_sender,
                        server,
                        timeout,
                    )
                    .await
                    {
                        let _ = update.send(Update::Failed(id, error.to_string())).await;
                    }
                }
                Task::Send {
                    id,
                    path,
                    sanitized_filename,
                    remote_user,
                    reverse,
                    server_handle,
                } => {
                    if let Err(error) = send(
                        id,
                        path,
                        sanitized_filename,
                        remote_user,
                        reverse,
                        server_handle,
                        action_receiver,
                        update_sender,
                        server,
                        timeout,
                    )
                    .await
                    {
                        let _ = update.send(Update::Failed(id, error.to_string())).await;
                    }
                }
            }
        });

        (
            Handle {
                sender: action_sender,
                task,
            },
            update_receiver,
        )
    }
}

pub enum Action {
    Approve { save_to: PathBuf },
    ReverseConfirmed { host: IpAddr, port: NonZeroU16 },
    PortAvailable { port: NonZeroU16 },
}

#[derive(Debug)]
pub enum Update {
    Metadata(Id, u64),
    Queued(Id),
    Ready(Id),
    Progress {
        id: Id,
        transferred: u64,
        elapsed: Duration,
    },
    Finished {
        id: Id,
        elapsed: Duration,
        sha256: String,
    },
    Failed(Id, String),
}

pub struct Server {
    pub public_address: IpAddr,
    pub bind_address: IpAddr,
}

async fn receive(
    id: Id,
    dcc_send: dcc::Send,
    remote_user: Nick,
    mut server_handle: server::Handle,
    mut action: Receiver<Action>,
    mut update: Sender<Update>,
    server: Option<Server>,
    timeout: Duration,
) -> Result<(), Error> {
    // Wait for approval
    let Some(Action::Approve { save_to }) = action.next().await else {
        return Ok(());
    };

    let (host, port, filename, size, reverse) = match dcc_send {
        dcc::Send::Direct {
            host,
            port,
            filename,
            size,
            ..
        } => (host, port, filename, size, false),
        dcc::Send::Reverse {
            filename,
            size,
            token,
            ..
        } => {
            let server = server.ok_or(Error::ReverseReceiveNoServerConfig)?;

            let _ = update.send(Update::Queued(id)).await;

            let Some(Action::PortAvailable { port }) = action.next().await else {
                unreachable!();
            };

            let _ = server_handle
                .send(
                    dcc::Send::Reverse {
                        filename: filename.clone(),
                        host: server.public_address,
                        port: Some(port),
                        size,
                        token,
                    }
                    .encode(&remote_user),
                )
                .await;

            (server.bind_address, port, filename, size, true)
        }
    };

    let started_at = Instant::now();

    let _ = update.send(Update::Ready(id)).await;

    let mut connection = if reverse {
        time::timeout(
            timeout,
            Connection::listen_and_accept(
                host,
                port.get(),
                connection::Security::Unsecured,
                BytesCodec::new(),
            ),
        )
        .await
        .map_err(|_| Error::TimeoutConnection)??
    } else {
        Connection::new(
            connection::Config {
                server: &host.to_string(),
                port: port.get(),
                security: connection::Security::Unsecured,
            },
            BytesCodec::new(),
        )
        .await?
    };

    let mut file = File::create(&save_to).await?;
    let mut hasher = Sha256::new();

    let mut transferred = 0;
    let mut last_progress = started_at;

    while transferred < size {
        if let Some(bytes) = connection.next().await {
            let bytes = bytes?;

            transferred += bytes.len() as u64;

            // Update hasher
            hasher.update(&bytes);

            // Write bytes to file
            file.write_all(&bytes).await?;

            let ack = Bytes::from_iter(((transferred & 0xFFFFFFFF) as u32).to_be_bytes());
            let _ = connection.send(ack).await;

            // Send progress at 60fps
            if last_progress.elapsed() >= Duration::from_millis(16) {
                let _ = update
                    .send(Update::Progress {
                        id,
                        elapsed: started_at.elapsed(),
                        transferred,
                    })
                    .await;
                last_progress = Instant::now();
            }
        }
    }

    let _ = connection.shutdown().await;

    let sha256 = hex::encode(hasher.finalize());

    let _ = server_handle
        .send(command!(
            "PRIVMSG",
            remote_user.to_string(),
            format!("Finished receiving \"{filename}\", sha256: {sha256}")
        ))
        .await;

    let _ = update
        .send(Update::Finished {
            id,
            elapsed: started_at.elapsed(),
            sha256,
        })
        .await;

    Ok(())
}

async fn send(
    id: Id,
    path: PathBuf,
    sanitized_filename: String,
    remote_user: Nick,
    reverse: bool,
    mut server_handle: server::Handle,
    mut action: Receiver<Action>,
    mut update: Sender<Update>,
    server: Option<Server>,
    timeout: Duration,
) -> Result<(), Error> {
    let mut file = File::open(path).await?;
    let size = file.metadata().await?.len();

    let _ = update.send(Update::Metadata(id, size)).await;

    let mut connection = if reverse {
        // Host doesn't matter for reverse connection
        let host = IpAddr::V4([127, 0, 0, 1].into());
        let token = u16::from(id).to_string();

        let _ = server_handle
            .send(
                dcc::Send::Reverse {
                    filename: sanitized_filename.clone(),
                    host,
                    port: None,
                    size,
                    token,
                }
                .encode(&remote_user),
            )
            .await;

        let Some(Action::ReverseConfirmed { host, port }) = time::timeout(timeout, action.next())
            .await
            .map_err(|_| Error::TimeoutPassive)?
        else {
            unreachable!();
        };

        let _ = update.send(Update::Ready(id)).await;

        Connection::new(
            connection::Config {
                server: &host.to_string(),
                port: port.get(),
                security: connection::Security::Unsecured,
            },
            BytesCodec::new(),
        )
        .await?
    } else {
        let server = server.ok_or(Error::NonPassiveSendNoServerConfig)?;

        let _ = update.send(Update::Queued(id)).await;

        let Some(Action::PortAvailable { port }) = action.next().await else {
            unreachable!();
        };

        let _ = server_handle
            .send(
                dcc::Send::Direct {
                    filename: sanitized_filename.clone(),
                    host: server.public_address,
                    port,
                    size,
                }
                .encode(&remote_user),
            )
            .await;

        let _ = update.send(Update::Ready(id)).await;

        time::timeout(
            timeout,
            Connection::listen_and_accept(
                server.bind_address,
                port.get(),
                connection::Security::Unsecured,
                BytesCodec::new(),
            ),
        )
        .await
        .map_err(|_| Error::TimeoutConnection)??
    };

    let started_at = Instant::now();

    let mut buffer = BytesMut::with_capacity(BUFFER_SIZE);
    let mut hasher = Sha256::new();

    let mut transferred = 0;
    let mut last_progress = started_at;

    while transferred < size {
        // Read bytes from file
        let n = file.read_buf(&mut buffer).await?;

        // Update hasher
        hasher.update(&buffer);

        // Send bytes
        connection.send(buffer.split().freeze()).await?;

        transferred += n as u64;

        buffer.reserve(BUFFER_SIZE);

        // Send progress at 60fps
        if last_progress.elapsed() >= Duration::from_millis(16) {
            let _ = update
                .send(Update::Progress {
                    id,
                    elapsed: started_at.elapsed(),
                    transferred,
                })
                .await;
            last_progress = Instant::now();
        }
    }

    // Ensure we receive ack
    'ack: while let Some(bytes) = connection.next().await {
        let bytes = bytes?;
        for chunk in bytes.chunks(4) {
            if chunk.len() == 4 {
                let ack = u32::from_be_bytes(chunk.try_into().unwrap());
                if ack == (size & 0xFFFFFFFF) as u32 {
                    break 'ack;
                }
            }
        }
    }

    let _ = connection.shutdown().await;

    let sha256 = hex::encode(hasher.finalize());

    let _ = server_handle
        .send(command!(
            "PRIVMSG",
            remote_user.to_string(),
            format!("Finished sending \"{sanitized_filename}\", sha256: {sha256}")
        ))
        .await;

    let _ = update
        .send(Update::Finished {
            id,
            elapsed: started_at.elapsed(),
            sha256,
        })
        .await;

    Ok(())
}

#[derive(Debug, Error)]
enum Error {
    #[error("sender requested passive send but [file_transfer.server] is not configured")]
    ReverseReceiveNoServerConfig,
    #[error("[file_transfer.server] must be configured to send a file when passive is disabled")]
    NonPassiveSendNoServerConfig,
    #[error("connection error: {0}")]
    Connection(#[from] connection::Error),
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("timed out waiting for remote to connect")]
    TimeoutConnection,
    #[error("timed out waiting for remote to confirm passive request")]
    TimeoutPassive,
}

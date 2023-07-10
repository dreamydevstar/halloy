use std::time::Duration;

use futures::channel::mpsc;
use futures::never::Never;
use futures::{stream, SinkExt, StreamExt};
use irc::proto::Capability;
use tokio::time::{self, Instant, Interval};

use crate::client::Connection;
use crate::server::Server;
use crate::{message, server};

pub type Result<T = Update, E = Error> = std::result::Result<T, E>;

#[derive(Debug)]
pub enum Error {
    Connection(irc::error::Error),
}

#[derive(Debug)]
pub enum Update {
    Connected {
        server: Server,
        connection: Connection,
        is_initial: bool,
    },
    Disconnected {
        server: Server,
        is_initial: bool,
        error: Option<String>,
    },
    ConnectionFailed {
        server: Server,
        error: String,
    },
    MessagesReceived(Server, Vec<message::Encoded>),
}

enum State {
    Disconnected {
        last_retry: Option<Instant>,
    },
    Connected {
        stream: irc::client::ClientStream,
        batch: Batch,
    },
}

enum Input {
    IrcMessage(Result<irc::proto::Message, irc::error::Error>),
    Batch(Vec<message::Encoded>),
}

pub async fn run(server: server::Entry, mut sender: mpsc::Sender<Update>) -> Never {
    const RECONNECT_DELAY: Duration = Duration::from_secs(10);

    let server::Entry { server, config } = server;

    let mut is_initial = true;
    let mut state = State::Disconnected { last_retry: None };
    // Notify app of initial disconnected state
    let _ = sender
        .send(Update::Disconnected {
            server: server.clone(),
            is_initial,
            error: None,
        })
        .await;

    loop {
        match &mut state {
            State::Disconnected { last_retry } => {
                if let Some(last_retry) = last_retry.as_ref() {
                    let remaining = RECONNECT_DELAY.saturating_sub(last_retry.elapsed());

                    if !remaining.is_zero() {
                        time::sleep(remaining).await;
                    }
                }

                match connect(config.clone()).await {
                    Ok((stream, connection)) => {
                        log::info!("[{server}] connected");

                        let _ = sender
                            .send(Update::Connected {
                                server: server.clone(),
                                connection,
                                is_initial,
                            })
                            .await;

                        is_initial = false;

                        state = State::Connected {
                            stream,
                            batch: Batch::new(),
                        };
                    }
                    Err(e) => {
                        log::warn!("[{server}] connection failed: {e}");

                        let _ = sender
                            .send(Update::ConnectionFailed {
                                server: server.clone(),
                                error: e.to_string(),
                            })
                            .await;

                        *last_retry = Some(Instant::now());
                    }
                }
            }
            State::Connected { stream, batch } => {
                let input = stream::select(stream.map(Input::IrcMessage), batch.map(Input::Batch))
                    .next()
                    .await
                    .expect("stream input");

                match input {
                    Input::IrcMessage(Ok(message)) => {
                        batch.messages.push(message.into());
                    }
                    Input::IrcMessage(Err(e)) => {
                        log::warn!("[{server}] disconnected: {e}");
                        let _ = sender
                            .send(Update::Disconnected {
                                server: server.clone(),
                                is_initial,
                                error: Some(e.to_string()),
                            })
                            .await;
                        state = State::Disconnected {
                            last_retry: Some(Instant::now()),
                        };
                    }
                    Input::Batch(messages) => {
                        let _ = sender
                            .send(Update::MessagesReceived(server.clone(), messages))
                            .await;
                    }
                }
            }
        }
    }
}

async fn connect(
    config: server::Config,
) -> Result<(irc::client::ClientStream, Connection), irc::error::Error> {
    use irc::proto::{CapSubCommand, Command};

    let mut client = irc::client::Client::from_config((*config).clone()).await?;
    let mut stream = client.stream()?;

    // Negotiate capbilities
    if client
        .send_cap_ls(irc::proto::NegotiationVersion::V302)
        .is_ok()
    {
        let mut str_caps = String::new();
        let mut caps = vec![];

        while let Some(Ok(message)) = stream.next().await {
            log::trace!("Message received => {:?}", message);

            if let Command::CAP(_, CapSubCommand::LS, a, b) = message.command {
                let (cap_str, asterisk) = match (a, b) {
                    (Some(cap_str), None) => (cap_str, None),
                    (Some(asterisk), Some(cap_str)) => (cap_str, Some(asterisk)),
                    // Unreachable?
                    (None, None) | (None, Some(_)) => break,
                };

                str_caps = format!("{str_caps} {cap_str}");

                if asterisk.is_none() {
                    break;
                }
            }
        }

        let server_caps = str_caps.split(' ').collect::<Vec<_>>();

        if server_caps.contains(&"server-time") {
            caps.push(Capability::ServerTime);
        }
        if server_caps.contains(&"batch") {
            caps.push(Capability::Batch);
        }
        // We require both so we can properly tag
        // echo-messages
        if server_caps.contains(&"echo-message") && server_caps.contains(&"labeled-response") {
            caps.push(Capability::EchoMessage);
            caps.push(Capability::Custom("labeled-response"));
        }

        let _ = client.send_cap_req(&caps);
    }

    client.identify()?;

    Ok((stream, Connection::new(client)))
}

struct Batch {
    interval: Interval,
    messages: Vec<message::Encoded>,
}

impl Batch {
    const INTERVAL_MILLIS: u64 = 50;

    fn new() -> Self {
        Self {
            interval: time::interval_at(
                Instant::now() + Duration::from_millis(Self::INTERVAL_MILLIS),
                Duration::from_millis(Self::INTERVAL_MILLIS),
            ),
            messages: vec![],
        }
    }
}

impl futures::Stream for Batch {
    type Item = Vec<message::Encoded>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let batch = self.get_mut();

        match batch.interval.poll_tick(cx) {
            std::task::Poll::Ready(_) => {
                let messages = std::mem::take(&mut batch.messages);

                if messages.is_empty() {
                    std::task::Poll::Pending
                } else {
                    std::task::Poll::Ready(Some(messages))
                }
            }
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}

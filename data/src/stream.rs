use futures::stream::{self, BoxStream};
use futures::{FutureExt, StreamExt};
use tokio::sync::mpsc;

use crate::client::Connection;
use crate::server::Server;
use crate::{message, server};

pub type Result<T = Event, E = Error> = std::result::Result<T, E>;

#[derive(Debug)]
pub enum Error {
    Connection(irc::error::Error),
}

#[derive(Debug)]
pub enum Event {
    Ready(mpsc::Sender<Message>),
    Connected(Server, Connection),
    MessageReceived(Server, message::Message),
}

#[derive(Debug, Clone)]
pub enum Message {
    Connect(String, server::Config),
}

enum State {
    Disconnected,
    Ready {
        receiver: mpsc::Receiver<Message>,
    },
    Connected {
        receiver: mpsc::Receiver<Message>,
        servers: Vec<ServerData>,
    },
}

struct ServerData {
    name: String,
    config: server::Config,
    stream: irc::client::ClientStream,
}
enum Input {
    Message(Option<Message>),
    IrcMessage(usize, Result<irc::proto::Message, irc::error::Error>),
}

pub fn run() -> BoxStream<'static, Result> {
    stream::unfold(State::Disconnected, move |state| async move {
        match state {
            State::Disconnected => {
                let (sender, receiver) = mpsc::channel(20);

                Some((Ok(Event::Ready(sender)), State::Ready { receiver }))
            }
            State::Ready { mut receiver } => loop {
                if let Some(Message::Connect(name, config)) = receiver.recv().await {
                    match connect(config.clone()).await {
                        Ok((stream, connection)) => {
                            let servers = vec![ServerData {
                                name: name.clone(),
                                config: config.clone(),
                                stream,
                            }];
                            let server =
                                Server::new(name, config.server.as_ref().expect("server hostname"));

                            return Some((
                                Ok(Event::Connected(server, connection)),
                                State::Connected { receiver, servers },
                            ));
                        }
                        Err(e) => {
                            return Some((Err(Error::Connection(e)), State::Ready { receiver }));
                        }
                    }
                }
            },
            State::Connected {
                mut receiver,
                mut servers,
            } => loop {
                let input = {
                    let mut select = stream::select(
                        stream::select_all(servers.iter_mut().enumerate().map(|(idx, server)| {
                            (&mut server.stream).map(move |result| Input::IrcMessage(idx, result))
                        })),
                        receiver.recv().map(Input::Message).into_stream().boxed(),
                    );

                    select.next().await.expect("Await stream input")
                };

                match input {
                    Input::Message(Some(message)) => match message {
                        Message::Connect(name, config) => match connect(config.clone()).await {
                            Ok((stream, connection)) => {
                                servers.push(ServerData {
                                    name: name.clone(),
                                    config: config.clone(),
                                    stream,
                                });
                                let server = Server::new(
                                    name,
                                    config.server.as_ref().expect("server hostname"),
                                );

                                return Some((
                                    Ok(Event::Connected(server, connection)),
                                    State::Connected { receiver, servers },
                                ));
                            }
                            Err(e) => {
                                return Some((
                                    Err(Error::Connection(e)),
                                    State::Ready { receiver },
                                ));
                            }
                        },
                    },
                    Input::IrcMessage(idx, Ok(message)) => {
                        let server = &servers[idx];
                        let server = Server::new(
                            &server.name,
                            server.config.server.as_ref().expect("server hostname"),
                        );

                        return Some((
                            Ok(Event::MessageReceived(
                                server,
                                message::Message::Received(message),
                            )),
                            State::Connected { receiver, servers },
                        ));
                    }
                    Input::Message(None) => {}
                    Input::IrcMessage(_, Err(_)) => {} // TODO: Handle?
                }
            },
        }
    })
    .boxed()
}

async fn connect(
    config: server::Config,
) -> Result<(irc::client::ClientStream, Connection), irc::error::Error> {
    let mut client = irc::client::Client::from_config((*config).clone()).await?;
    client.identify()?;

    Ok((client.stream()?, Connection::new(client)))
}

use chrono::{DateTime, Local, Utc};
use irc::proto;
use irc::proto::ChannelExt;
use serde::{Deserialize, Serialize};

use crate::time::{self, Posix};
use crate::user::Nick;
use crate::User;

pub type Raw = irc::proto::Message;
pub type Channel = String;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Source {
    Server,
    Channel(Channel, Sender),
    Query(Nick, Sender),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Sender {
    User(User),
    Server,
}

impl Sender {
    pub fn user(&self) -> Option<&User> {
        match self {
            Sender::User(user) => Some(user),
            Sender::Server => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Direction {
    Sent,
    Received,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub datetime: DateTime<Utc>,
    pub direction: Direction,
    pub source: Source,
    pub text: String,
}

impl Message {
    pub fn is_server(&self) -> bool {
        matches!(self.source, Source::Server)
    }

    pub fn formatted_datetime(&self, fmt: &str) -> String {
        self.datetime.with_timezone(&Local).format(fmt).to_string()
    }

    pub fn channel(&self) -> Option<&str> {
        if let Source::Channel(channel, _) = &self.source {
            Some(channel)
        } else {
            None
        }
    }

    pub fn sent_by(&self) -> Option<&User> {
        match &self.source {
            Source::Server => None,
            Source::Channel(_, kind) => kind.user(),
            Source::Query(_, kind) => kind.user(),
        }
    }

    pub fn received(proto: proto::Message, our_nick: &Nick) -> Option<Message> {
        let datetime = datetime(&proto);
        let text = text(&proto, our_nick)?;
        let source = source(proto, our_nick)?;

        Some(Message {
            datetime,
            direction: Direction::Received,
            source,
            text,
        })
    }
}

fn user(proto: &proto::Message) -> Option<User> {
    fn not_empty(s: &str) -> Option<&str> {
        (!s.is_empty()).then_some(s)
    }

    let prefix = proto.clone().prefix?;
    match prefix {
        proto::Prefix::Nickname(nickname, username, hostname) => Some(User::new(
            Nick::from(nickname.as_str()),
            not_empty(&username),
            not_empty(&hostname),
        )),
        _ => None,
    }
}

fn source(message: irc::proto::Message, our_nick: &Nick) -> Option<Source> {
    let user = user(&message);

    match message.command {
        // Channel
        proto::Command::TOPIC(channel, _)
        | proto::Command::PART(channel, _)
        | proto::Command::ChannelMODE(channel, _)
        | proto::Command::KICK(channel, _, _)
        | proto::Command::SAJOIN(_, channel)
        | proto::Command::JOIN(channel, _, _) => Some(Source::Channel(channel, Sender::Server)),
        proto::Command::Response(
            proto::Response::RPL_TOPIC | proto::Response::RPL_TOPICWHOTIME,
            params,
        ) => {
            let channel = params.get(1)?.clone();
            Some(Source::Channel(channel, Sender::Server))
        }
        proto::Command::PRIVMSG(target, text) | proto::Command::NOTICE(target, text) => {
            let is_action = is_action(&text);
            let sender = |user| {
                if is_action {
                    Sender::Server
                } else {
                    Sender::User(user)
                }
            };

            match (target.is_channel_name(), user) {
                (true, Some(user)) => Some(Source::Channel(target, sender(user))),
                (false, Some(user)) => {
                    let target = User::try_from(target.as_str()).ok()?.nickname();

                    (&target == our_nick).then(|| Source::Query(user.nickname(), sender(user)))
                }
                _ => None,
            }
        }

        // Server
        proto::Command::SANICK(_, _)
        | proto::Command::SAMODE(_, _, _)
        | proto::Command::PASS(_)
        | proto::Command::NICK(_)
        | proto::Command::USER(_, _, _)
        | proto::Command::OPER(_, _)
        | proto::Command::UserMODE(_, _)
        | proto::Command::SERVICE(_, _, _, _, _, _)
        | proto::Command::QUIT(_)
        | proto::Command::SQUIT(_, _)
        | proto::Command::NAMES(_, _)
        | proto::Command::LIST(_, _)
        | proto::Command::INVITE(_, _)
        | proto::Command::MOTD(_)
        | proto::Command::LUSERS(_, _)
        | proto::Command::VERSION(_)
        | proto::Command::STATS(_, _)
        | proto::Command::LINKS(_, _)
        | proto::Command::TIME(_)
        | proto::Command::CONNECT(_, _, _)
        | proto::Command::TRACE(_)
        | proto::Command::ADMIN(_)
        | proto::Command::INFO(_)
        | proto::Command::SERVLIST(_, _)
        | proto::Command::SQUERY(_, _)
        | proto::Command::WHO(_, _)
        | proto::Command::WHOIS(_, _)
        | proto::Command::WHOWAS(_, _, _)
        | proto::Command::KILL(_, _)
        | proto::Command::PING(_, _)
        | proto::Command::PONG(_, _)
        | proto::Command::ERROR(_)
        | proto::Command::AWAY(_)
        | proto::Command::REHASH
        | proto::Command::DIE
        | proto::Command::RESTART
        | proto::Command::SUMMON(_, _, _)
        | proto::Command::USERS(_)
        | proto::Command::WALLOPS(_)
        | proto::Command::USERHOST(_)
        | proto::Command::ISON(_)
        | proto::Command::SAPART(_, _)
        | proto::Command::NICKSERV(_)
        | proto::Command::CHANSERV(_)
        | proto::Command::OPERSERV(_)
        | proto::Command::BOTSERV(_)
        | proto::Command::HOSTSERV(_)
        | proto::Command::MEMOSERV(_)
        | proto::Command::CAP(_, _, _, _)
        | proto::Command::AUTHENTICATE(_)
        | proto::Command::ACCOUNT(_)
        | proto::Command::METADATA(_, _, _)
        | proto::Command::MONITOR(_, _)
        | proto::Command::BATCH(_, _, _)
        | proto::Command::CHGHOST(_, _)
        | proto::Command::Response(_, _)
        | proto::Command::Raw(_, _)
        | proto::Command::SAQUIT(_, _) => Some(Source::Server),
    }
}

fn datetime(message: &irc::proto::Message) -> DateTime<Utc> {
    message
        .tags
        .as_ref()
        .and_then(|tags| tags.iter().find(|tag| tag.0 == "time"))
        .and_then(|tag| tag.1.clone())
        .and_then(|rfc3339| DateTime::parse_from_rfc3339(&rfc3339).ok())
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(Utc::now)
}

fn text(message: &irc::proto::Message, our_nick: &Nick) -> Option<String> {
    let user = user(message);
    match &message.command {
        proto::Command::TOPIC(_, topic) => {
            let user = user?;
            let topic = topic.as_ref()?;

            Some(format!(" ∙ {user} changed topic to {topic}"))
        }
        proto::Command::PART(_, text) => {
            let user = user?;
            let text = text
                .as_ref()
                .map(|text| format!(" ({text})"))
                .unwrap_or_default();

            Some(format!("⟵ {user}{text} has left the channel"))
        }
        proto::Command::JOIN(_, _, _) | proto::Command::SAJOIN(_, _) => {
            let user = user?;

            (&user.nickname() != our_nick).then(|| format!("⟶ {user} has joined the channel"))
        }
        proto::Command::ChannelMODE(_, modes) => {
            let user = user?;
            let modes = modes
                .iter()
                .map(|mode| mode.to_string())
                .collect::<Vec<_>>()
                .join(" ");

            Some(format!(" ∙ {user} sets mode {modes}"))
        }
        proto::Command::PRIVMSG(_, text) => {
            // Check if a synthetic action message
            if let Some(nick) = user.as_ref().map(User::nickname) {
                if let Some(action) = action_text(&nick, text) {
                    return Some(action);
                }
            }

            Some(text.clone())
        }
        proto::Command::NOTICE(_, text) => Some(text.clone()),
        proto::Command::Response(proto::Response::RPL_TOPIC, params) => {
            let topic = params.get(2)?;

            Some(format!(" ∙ topic is {topic}"))
        }
        proto::Command::Response(proto::Response::RPL_TOPICWHOTIME, params) => {
            let nick = params.get(2)?;
            let datetime = params
                .get(3)?
                .parse::<u64>()
                .ok()
                .map(Posix::from_seconds)
                .as_ref()
                .and_then(Posix::datetime)?
                .to_rfc2822();

            Some(format!(" ∙ topic set by {nick} at {datetime}"))
        }
        proto::Command::Response(_, responses) => Some(
            responses
                .iter()
                .map(|s| s.as_str())
                .skip(1)
                .collect::<Vec<_>>()
                .join(" "),
        ),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Limit {
    Top(usize),
    Bottom(usize),
    Since(time::Posix),
}

impl Limit {
    pub const DEFAULT_STEP: usize = 50;
    const DEFAULT_COUNT: usize = 500;

    pub fn top() -> Self {
        Self::Top(Self::DEFAULT_COUNT)
    }

    pub fn bottom() -> Self {
        Self::Bottom(Self::DEFAULT_COUNT)
    }

    fn value_mut(&mut self) -> Option<&mut usize> {
        match self {
            Limit::Top(i) => Some(i),
            Limit::Bottom(i) => Some(i),
            Limit::Since(_) => None,
        }
    }

    pub fn increase(&mut self, n: usize) {
        if let Some(value) = self.value_mut() {
            *value += n;
        }
    }
}

fn is_action(text: &str) -> bool {
    text.starts_with("\u{1}ACTION ") && text.ends_with("\u{1}")
}

pub fn action_text(nick: &Nick, text: &str) -> Option<String> {
    let action = text.strip_prefix("\u{1}ACTION ")?.strip_suffix("\u{1}")?;
    Some(format!(" ∙ {nick} {action}"))
}

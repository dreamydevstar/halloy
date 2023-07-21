use itertools::Itertools;

use crate::{Message, Tag};

pub fn message(message: Message) -> String {
    let command = message.command.command();

    let params = parameters(message.command.parameters());

    let tags = tags(message.tags);

    if tags.is_empty() {
        format!("{command} {params}\r\n")
    } else {
        format!("@{tags} {command} {params}\r\n")
    }
}

fn tags(tags: Vec<Tag>) -> String {
    tags.into_iter().map(tag).join(";")
}

fn tag(tag: Tag) -> String {
    match tag.value {
        Some(value) => {
            let mappings = [
                ('\\', r"\\"),
                (';', r"\:"),
                (' ', r"\s"),
                ('\r', r"\r"),
                ('\n', r"\n"),
            ];

            let escaped = mappings
                .into_iter()
                .fold(value, |value, (from, to)| value.replace(from, to));

            format!("{}={escaped}", tag.key)
        }
        None => tag.key,
    }
}

fn parameters(parameters: Vec<String>) -> String {
    let params_len = parameters.len();
    parameters
        .into_iter()
        .enumerate()
        .map(|(index, param)| {
            if index == params_len - 1 {
                trailing(param)
            } else {
                param
            }
        })
        .join(" ")
}

fn trailing(parameter: String) -> String {
    if parameter.contains(' ') || parameter.is_empty() || parameter.starts_with(':') {
        format!(":{parameter}")
    } else {
        parameter
    }
}

#[cfg(test)]
mod test {
    use crate::{command, format, Tag};

    #[test]
    fn commands() {
        let tests = [
            command!("CAP", "LS", "302"),
            command!("privmsg", "#a", "nospace"),
            command!("privmsg", "b", "spa ces"),
            command!("quit", "nocolon"),
            command!("quit", ":startscolon"),
            command!("quit", "not:starting"),
            command!("quit", "not:starting space"),
            command!("notice", ""),
            command!("notice", " "),
            command!("USER", "test", "test"),
        ];
        let expected = [
            "CAP LS 302\r\n",
            "PRIVMSG #a nospace\r\n",
            "PRIVMSG b :spa ces\r\n",
            "QUIT nocolon\r\n",
            "QUIT ::startscolon\r\n",
            "QUIT not:starting\r\n",
            "QUIT :not:starting space\r\n",
            "NOTICE :\r\n",
            "NOTICE : \r\n",
            "USER test 0 * test\r\n",
        ];

        for (test, expected) in tests.into_iter().zip(expected) {
            let formatted = format::message(test);
            assert_eq!(formatted, expected);
        }
    }

    #[test]
    fn tags() {
        let test = vec![
            Tag {
                key: "tag".into(),
                value: Some("as\\; \r\n".into()),
            },
            Tag {
                key: "id".into(),
                value: Some("234AB".into()),
            },
            Tag {
                key: "test".into(),
                value: None,
            },
        ];
        let expected = r"tag=as\\\:\s\r\n;id=234AB;test";

        let tags = super::tags(test);
        assert_eq!(tags, expected);
    }
}

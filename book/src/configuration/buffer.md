# Buffer

## `[buffer]` Section

## `[buffer.nickname]` Section 

```toml
[buffer.nickname]
color = "unique" | "solid"
brackets = { left = "<string>", right = "<string>" }
```

| Key        | Description                                      | Default                     |
| ---------- | ------------------------------------------------ | --------------------------- |
| `color`    | Nickname colors. Can be `"unique"` or `"solid"`. | `"unique"`                  |
| `brackets` | Brackets for nicknames.                          | `{ left = "", right = "" }` |


## `[buffer.timestamp]` Section

```toml
[buffer.timestamp]
format = "<string>"
brackets = { left = "<string>", right = "<string>" }
```

| Key        | Description                                                                                                                                     | Default                     |
| ---------- | ----------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------- |
| `format`   | Format expected is  [strftime]( https://pubs.opengroup.org/onlinepubs/007908799/xsh/strftime.html ). To disable, simply pass empty string `""`. | `"%R"`                      |
| `brackets` | Brackets for nicknames                                                                                                                          | `{ left = "", right = "" }` |

## `[buffer.text_input]` Section

```toml
[buffer.text_input]
visibility = "always" | "focused"
```

| Key          | Description                                              | Default    |
| ------------ | -------------------------------------------------------- | ---------- |
| `visibility` | Text input visibility. Can be `"always"` or `"focused"`. | `"always"` |

## `[buffer.channel]` Section

### `[buffer.channel.nicklist]` Section

```toml
[buffer.channel.nicklist]
enabled = true | false
position = "left" | "right"
color = "unique" | "solid"
```

| Key        | Description                                      | Default    |
| ---------- | ------------------------------------------------ | ---------- |
| `enabled`  | Control if nicklist should be shown or not       | `true`     |
| `position` | Nicklist position. Can be `"left"` or `"right"`. | `"right"`  |
| `color`    | Nickname colors. Can be `"unique"` or `"solid"`. | `"unique"` |

### `[buffer.channel.topic]` Section

```toml
[buffer.channel.topic]
enabled = true | false
max_lines = <integer>
```

| Key         | Description                                                        | Default |
| ----------- | ------------------------------------------------------------------ | ------- |
| `enabled`   | Control if topic banner should be shown or not                     | `false` |
| `max_lines` | Amount of visible lines before you have to scroll in topic banner. | `2`     |

## `[buffer.server_messages]` Section

```toml
[buffer.server_messages.join]
enabled = true | false
smart = <integer>
username_format = "full" | "short"
```

```toml
[buffer.server_messages.part]
enabled = true | false
smart = <integer>
username_format = "full" | "short"
```

```toml
[buffer.server_messages.quit]
enabled = true | false
smart = <integer>
username_format = "full" | "short"
```

```toml
[buffer.server_messages.topic]
enabled = true | false
```

| Key                                                      | Description                                                                                                                                                      | Default   |
| -------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------- |
| `enabled`                                                | Contr                                                                                                                                                            |
| ol if the server message should appear in buffers or not | `true`                                                                                                                                                           |
| `smart`                                                  | Only show server message if the user has sent a message in the given time interval (seconds) prior to the server message.                                        | `not set` |
| `username-format`                                        | Adjust how the username should look. Can be `"full"` (shows the longest username available (nickname, username and hostname) or `"short"` (only shows nickname). | `"full"`  |
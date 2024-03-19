# Connect with ZNC

To connect with a **ZNC**[^1] bouncer, the configuration below can be used as a template. Simply change so it fits your credentials.

```toml
[servers.libera]
nickname = "<znc-user>/<znc-network>"
server = "znc.example.com"
port = 6667
password = "<your-password>"
use_tls = true
```

[^1]: [https://wiki.znc.in/ZNC](https://wiki.znc.in/ZNC)
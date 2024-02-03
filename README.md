# feed2imap

Load RSS/ATOM feeds and store them in a IMAP mailbox, allowing one to read its
feeds from a mail client compatible with IMAP.

## build

feed2imap is a rust crate, build it using:

```bash
cargo build
# or
cargo install
```

## configuration

feed2imap expects to find a configuration file at `~/.config/feed2imap.toml`, or
by passing it by a command line option `--option`. 

The format for configuration is TOML, and must ressemble this:
```toml
[imap]
host = "mail.example.com"
port = 993
username = "test"
password = "azerty123"
name = "John Smith"
email = "test@example.com"

[[feeds]]
url = "http://example.org/rss"

[[feeds]]
url = "http://another.org/atom"
```

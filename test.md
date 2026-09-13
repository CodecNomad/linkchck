# [Radio1 Cli](https://crates.io/crates/radio1-cli)
# [Radio1 Cli](https://intentionallydeadlink.io/crates/radio1-cli)

## Most important
- Type "radio1-cli" and hit enter to run it
- Type "h" in the interactive cli and hit enter to learn how to use it
- Everything is configured in the config file which you can find by typing cfg

## For more:
Just read the source, it's less than 200 loc

## Default config:
```toml
# 1.0 means 100% so 0.6 would mean 60%
volume = 1.0

# How many bytes to pre-fetch
prefetch = 0

# User agent, worth having because they usually block without one
software_ua_header = "Mozilla/5.0 (X11; Linux x86_64; rv:147.0) Gecko/20100101 Firefox/147.0"

# Any Shoutcast / Icecast works as long as it uses mp3
mp3_stream_url = "https://icast.connectmedia.hu/5201/live.mp3"
```

# rss-webhook-trigger

Trigger webhooks from rss feeds.

Send a `POST` request to a webhook every time an rss feed changes.

Note that this will only detect changes made while the program is running, it is not able to detect changes made to
the feeds on program start.

### Configuration

```toml
interval = 600 # optional, defaults to 30 minutes

[[feed]]
feed = "https://example.com/feed1.xml"
hook = "https://hook.example.com/hook1/call"

[[feed]]
feed = "https://example.com/feed2.xml"
hook = "https://hook.example.com/hook2/call"
headers = { authorization = "...." }
body = { event_type = "build" }

# trigger on docker hub updates instead of rss feed update
[[feed]]
feed = "docker-hub://matrixdotorg/synapse"
hook = "https://hook.example.com/hook2/call"
```
# rss-webhook-trigger

Trigger webhooks from rss feeds

### Configuration

```toml
interval = 600 # optional, defaults to 30 minutes

[[feed]]
feed = "https://example.com/feed1.xml"
hook = "https://hook.example.com/hook1/call"

[[feed]]
feed = "https://example.com/feed2.xml"
hook = "https://hook.example.com/hook2/call"
```
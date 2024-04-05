# rss-webhook-trigger

Trigger webhooks from rss/atom feeds.

Send a `POST` request to a webhook every time an rss/atom feed changes.

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

# you can load header values from external files to keep your screts separate
[[feed]]
feed = "https://example.com/feed3.xml"
hook = "https://hook.example.com/hook3/call"
headers = { authorization = "/run/secrets/hook-auth" }
body = { event_type = "build" }

# trigger on docker hub updates instead of rss feed update
[[feed]]
feed = "docker-hub://matrixdotorg/synapse"
hook = "https://hook.example.com/hook2/call"
```

### Usage in NixOS

A NixOS module is included and can be used like this:

```nix
{
  inputs.rss-webhook-trigger.url = "github:icewind1991/rss-webhook-trigger";

  outputs = { self, nixpkgs, rss-webhook-trigger }: {
    nixosConfigurations.my-machine = nixpkgs.lib.nixosSystem {
      modules =
        [ 
          rss-webhook-trigger.nixosModules.default
          {
            services.rss-webhook-trigger = {
              enable = true;
              hooks = [
                {
                  feed = "https://example.com/feed1.xml";
                  hook = "https://hook.example.com/hook1/call";
                }
              ];
            };
          }
          # ... other configuration ...
        ];
    };
  };
}
```
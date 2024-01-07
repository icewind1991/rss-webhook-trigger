{
  config,
  lib,
  pkgs,
  ...
}:
with lib; let
  cfg = config.services.rss-webhook-trigger;
  format = pkgs.formats.toml {};
  configFile = format.generate "trigger.toml" {
    feed = cfg.hooks;
  };
in {
  options.services.rss-webhook-trigger = {
    enable = mkEnableOption "Enables the rss-webhook-trigger service";

    hooks = mkOption rec {
      description = "Hook configuration";
      type = types.listOf (types.submodule {
        options = {
          feed = mkOption {
            type = types.str;
            description = "Source feed";
          };
          hook = mkOption {
            type = types.str;
            description = "hook url";
          };
          headers = mkOption {
            type = types.attrs;
            default = {};
            description = "headers to send";
          };
          body = mkOption {
            type = types.attrs;
            default = {};
            description = "body to send";
          };
        };
      });
    };

    package = mkOption {
      type = types.package;
      description = "package to use";
    };
  };

  config = mkIf cfg.enable {
    systemd.services."rss-webhook-trigger" = {
      wantedBy = ["multi-user.target"];
      script = "${cfg.package}/bin/rss-webhook-trigger ${configFile}";

      serviceConfig = {
        Restart = "on-failure";
        DynamicUser = true;
        PrivateTmp = true;
        ProtectSystem = "strict";
        ProtectHome = true;
        NoNewPrivileges = true;
        PrivateDevices = true;
        ProtectClock = true;
        CapabilityBoundingSet = true;
        ProtectKernelLogs = true;
        ProtectControlGroups = true;
        SystemCallArchitectures = "native";
        ProtectKernelModules = true;
        RestrictNamespaces = true;
        MemoryDenyWriteExecute = true;
        ProtectHostname = true;
        LockPersonality = true;
        ProtectKernelTunables = true;
        RestrictAddressFamilies = "AF_INET AF_INET6";
        RestrictRealtime = true;
        ProtectProc = "noaccess";
        SystemCallFilter = ["@system-service" "~@resources" "~@privileged"];
        IPAddressDeny = "localhost link-local multicast";
      };
    };
  };
}

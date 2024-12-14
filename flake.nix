{
  inputs = {
    nixpkgs.url = "nixpkgs/nixos-24.11";
    flakelight = {
      url = "github:nix-community/flakelight";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    mill-scale = {
      url = "github:icewind1991/mill-scale";
      inputs.flakelight.follows = "flakelight";
    };
  };
  outputs = { mill-scale, ... }: mill-scale ./. {
    nixosModules = { outputs, ... }: {
      default =
        { pkgs
        , config
        , lib
        , ...
        }: {
          imports = [ ./nix/module.nix ];
          config = lib.mkIf config.services.rss-webhook-trigger.enable {
            nixpkgs.overlays = [ outputs.overlays.default ];
            services.rss-webhook-trigger.package = lib.mkDefault pkgs.rss-webhook-trigger;
          };
        };
    };
  };
}

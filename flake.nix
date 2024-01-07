{
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "nixpkgs/nixos-23.11";
    naersk.url = "github:nix-community/naersk";
    naersk.inputs.nixpkgs.follows = "nixpkgs";
    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
    rust-overlay.inputs.flake-utils.follows = "flake-utils";
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
    naersk,
    rust-overlay,
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        overlays = [
          (import rust-overlay)
          (import ./overlay.nix)
        ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        inherit (pkgs) lib rust-bin callPackage;
        inherit (builtins) fromTOML readFile;

        msrv = (fromTOML (readFile ./Cargo.toml)).package.rust-version;
        toolchain = rust-bin.stable.latest.default;
        msrvToolchain = rust-bin.stable."${msrv}".default;

        naersk' = callPackage naersk {
          rustc = toolchain;
          cargo = toolchain;
        };
        msrvNaersk = callPackage naersk {
          rustc = msrvToolchain;
          cargo = msrvToolchain;
        };

        nearskOpt = {
          pname = "rss-webhook-trigger";
          root = pkgs.rss-webhook-trigger.src;
        };
      in rec {
        packages = rec {
          rss-webhook-trigger = pkgs.rss-webhook-trigger;
          check = naersk'.buildPackage (nearskOpt
            // {
              mode = "check";
            });
          clippy = naersk'.buildPackage (nearskOpt
            // {
              mode = "clippy";
            });
          msrv = msrvNaersk.buildPackage (nearskOpt
            // {
              mode = "check";
            });
          docker = callPackage ./docker.nix {};
          default = rss-webhook-trigger;
        };

        devShells.default = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [rustc cargo bacon cargo-edit cargo-outdated cargo-msrv];
        };
      }
    )
    // {
      overlays.default = import ./overlay.nix;
      nixosModules.default = {
        pkgs,
        config,
        lib,
        ...
      }: {
        imports = [./module.nix];
        config = lib.mkIf config.services.rss-webhook-trigger.enable {
          nixpkgs.overlays = [self.overlays.default];
          services.palantir.package = lib.mkDefault pkgs.rss-webhook-trigger;
        };
      };
    };
}

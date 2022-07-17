{
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nix-community/naersk";
  };

  outputs = { self, nixpkgs, flake-utils, naersk }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        pkgs = nixpkgs.legacyPackages."${system}";
        naersk-lib = naersk.lib."${system}";
      in
        rec {
          # `nix build`
          packages.rss-webhook-trigger = naersk-lib.buildPackage {
            pname = "rss-webhook-trigger";
            root = ./.;
          };
          defaultPackage = packages.rss-webhook-trigger;
          defaultApp = packages.rss-webhook-trigger;

          # `nix develop`
          devShell = pkgs.mkShell {
            nativeBuildInputs = with pkgs; [ rustc cargo bacon cargo-edit cargo-outdated ];
          };
        }
    );
}

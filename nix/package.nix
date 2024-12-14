{ rustPlatform
, lib
}:
let
  inherit (lib.sources) sourceByRegex;
  inherit (builtins) fromTOML readFile;
  src = sourceByRegex ../. [ "Cargo.*" "(src)(/.*)?" ];
  cargoPackage = (fromTOML (readFile ../Cargo.toml)).package;
in
rustPlatform.buildRustPackage rec {
  inherit (cargoPackage) version;
  pname = cargoPackage.name;

  inherit src;

  cargoLock = {
    lockFile = ../Cargo.lock;
  };
}

{
  lib,

  rustPlatform,
  gitignore,

  scipopt-scip,
  clang,
  libclang,
}:

let
  inherit (gitignore.lib) gitignoreSource;

  src = gitignoreSource ./.;
  cargo-toml = lib.importTOML "${src}/Cargo.toml";
in
rustPlatform.buildRustPackage {
  pname = cargo-toml.package.name;
  version = cargo-toml.package.version;

  inherit src;

  cargoLock.lockFile = "${src}/Cargo.lock";

  nativeBuildInputs = [
    clang
  ];

  buildInputs = [ ];

  LIBCLANG_PATH = lib.makeLibraryPath [ libclang.lib ];
  SCIPOPTDIR = scipopt-scip.out;

  meta = {
    inherit (cargo-toml.package) homepage;
    mainProgram = "metro-speedrun";
  };
}

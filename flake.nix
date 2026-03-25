{
  inputs = {
    nixpkgs.url = "https://channels.nixos.org/nixos-unstable/nixexprs.tar.xz";

    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";

    gitignore.url = "github:hercules-ci/gitignore.nix";
    gitignore.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      gitignore,
    }:
    let
      inherit (nixpkgs.lib) genAttrs;

      forAllSystems = genAttrs [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
      ];
      forAllPkgs = function: forAllSystems (system: function pkgs.${system});

      pkgs = forAllSystems (
        system:
        import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        }
      );
    in
    {
      formatter = forAllPkgs (pkgs: pkgs.nixfmt-tree);

      packages = forAllPkgs (pkgs: rec {
        default = metro-speedrun;
        metro-speedrun = pkgs.callPackage ./package.nix { inherit gitignore; };
      });

      devShells = forAllPkgs (
        pkgs:
        let
          file-rust-toolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
          rust-toolchain = file-rust-toolchain.override { extensions = [ "rust-analyzer"  "rust-src" ]; };
        in
        {
          default = pkgs.mkShell {
            packages = with pkgs; [
              pkg-config
              rust-toolchain
              lldb

              csvlens
              graphviz
              glpk

              clang
              scipopt-scip
            ];

            shellHook = ''
              echo -e "\nRead the README if you encounter any library linking issue."
            '';

            LIBCLANG_PATH = pkgs.lib.makeLibraryPath [ pkgs.libclang.lib ];
            SCIPOPTDIR = pkgs.scipopt-scip.out;
          };
        }
      );
    };
}

{
  description = "rustowl-mcp: Model Context Protocol server for RustOwl lifetime & borrow inspection";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        rustPackage = pkgs.rustPlatform.buildRustPackage {
          pname = "rustowl-mcp";
          version = "0.1.0";
          src = ./.;
          cargoLock = {
            lockFile = ./Cargo.lock;
          };
          nativeBuildInputs = [ pkgs.pkg-config ];
        };
      in
      {
        packages.default = rustPackage;

        apps.default = flake-utils.lib.mkApp {
          drv = rustPackage;
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            cargo
            rustc
            clippy
            rustfmt
          ];
        };
      }
    );
}

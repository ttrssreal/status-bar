{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };
  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        runtimeDeps = pkgs.lib.makeBinPath (with pkgs; [
          alsa-utils
          networkmanager
        ]);
      in {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "status_bar";
          version = "0.1";

          src = ./.;

          nativeBuildInputs = with pkgs; [ makeWrapper ];

          cargoHash = "sha256-mtC0/KKfc/Yz69PoWElAxTi3qQnufCLwDQm4wXOVnII=";

          postInstall = ''
            wrapProgram $out/bin/status_bar --prefix PATH : ${runtimeDeps}
          '';
        };
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rust-bin.stable.latest.default
          ];
        };
      }
    );
}

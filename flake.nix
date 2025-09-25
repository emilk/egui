{
  description = "Environment for a GUI Timer.";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      nixpkgs,
      rust-overlay,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };

        rust-build = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" ];
        };

      in
      {
        devShells.default =
          with pkgs;
          mkShell {
            buildInputs = [
              rust-build
              bacon
            ];

            LD_LIBRARY_PATH =
              let
                libPath =
                  with pkgs;
                  lib.makeLibraryPath [
                    libGL
                    libxkbcommon
                    wayland
                    xorg.libX11
                    xorg.libXcursor
                    xorg.libXi
                    xorg.libXrandr
                  ];
              in
              libPath;
          };
      }
    );
}

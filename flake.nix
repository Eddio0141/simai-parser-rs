{
  inputs = {
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nixpkgs.url = "github:NixOS/nixpkgs";
    flake-parts.url = "github:hercules-ci/flake-parts";
  };

  outputs =
    {
      flake-parts,
      nixpkgs,
      rust-overlay,
      ...
    }@inputs:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "x86_64-linux"
      ];
      perSystem =
        {
          system,
          pkgs,
          ...
        }:
        let
          rust = pkgs.rust-bin.stable.latest.default;

          # the correct rust-doc command is made, no need to modify this
          rust-doc = pkgs.writeShellApplication {
            name = "rust-doc";
            text = ''
              xdg-open "${rust}/share/doc/rust/html/index.html"
            '';
          };
        in
        {
          _module.args.pkgs = import nixpkgs {
            inherit system;
            overlays = [
              (import rust-overlay)
            ];
          };

          devShells.default = pkgs.mkShell {
            packages = with pkgs; [
              (rust.override {
                extensions = [
                  "rust-analyzer"
                  "rust-src"
                ];
              })
              rust-doc
              pkg-config
              # Audio (Linux only)
              alsa-lib
              # Cross Platform 3D Graphics API
              vulkan-loader
              # For debugging around vulkan
              vulkan-tools
              # Other dependencies
              libudev-zero
              libx11
              libxcursor
              libxi
              libxrandr
              libxkbcommon
              wayland
            ];
            LD_LIBRARY_PATH =
              with pkgs;
              pkgs.lib.makeLibraryPath [
                vulkan-loader
                libx11
                libxi
                libxcursor
                libxkbcommon
                wayland
              ];
          };
        };
    };
}

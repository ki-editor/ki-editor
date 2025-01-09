{
  description = "Flake for Ki Editor";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };
  outputs =
    { self
    , nixpkgs
    , crane
    , flake-utils
    , rust-overlay
    }:
    let
      # Normalizes Rust version string to ensure it has a patch version.
      # - Returns version as-is if it matches "X.Y.Z" format (e.g. "1.83.0")
      # - Appends ".0" if version matches "X.Y" format (e.g. "1.83" -> "1.83.0")
      # - Throws an error for any other format
      normalizeRustVersion = version:
        let
          majorMinorPatch = builtins.match "([0-9]+)[.]([0-9]+)[.]([0-9]+)" version;
          majorMinor = builtins.match "([0-9]+)[.]([0-9]+)" version;
        in
        if majorMinorPatch != null then version
        else if majorMinor != null then version + ".0"
        else throw "Invalid Rust version format: ${version}. Expected format: X.Y.Z or X.Y";
    in
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };
        craneLib = crane.mkLib pkgs;
        cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
        rustVersion = normalizeRustVersion cargoToml.package.rust-version;
        rustToolchain = pkgs.rust-bin.stable.${rustVersion}.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };
        commonArgs = {
          src = ./.;
          strictDeps = true;
          doCheck = false;
          buildInputs = with pkgs; [
            openssl
          ] ++ lib.optionals stdenv.isDarwin [
            darwin.apple_sdk.frameworks.Security
            darwin.apple_sdk.frameworks.SystemConfiguration
            darwin.apple_sdk.frameworks.CoreServices
            darwin.apple_sdk.frameworks.CoreFoundation
            darwin.apple_sdk.frameworks.CoreGraphics
            darwin.apple_sdk.frameworks.AppKit
          ];
          nativeBuildInputs = with pkgs; [
            pkg-config
            git
            rustToolchain
          ];
        };
        ki-editor = craneLib.buildPackage (
          commonArgs
          // {
            cargoArtifacts = craneLib.buildDepsOnly commonArgs;
          }
        );
      in
      {
        packages.default = ki-editor;
        devShells.default = craneLib.devShell {
          packages = [ pkgs.rust-analyzer ];
        };
      }
    );
}

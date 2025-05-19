{
  description = "Flake for Ki Editor";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
  outputs = { self, nixpkgs, crane, flake-utils, rust-overlay, ... }:
    flake-utils.lib.eachDefaultSystem (localSystem:
      let
        pkgs = import nixpkgs {
          inherit localSystem;
          overlays = [ (import rust-overlay) ];
        };

        # Function to create a Rust toolchain with specific targets
        mkRustToolchain = targets: pkgs.rust-bin.stable."1.83.0".default.override {
          inherit targets;
        };

        # Create a hook to generate the VERSION file
        createVersionFile = ''
          echo "Creating VERSION file..."
          echo "0.1.0" > $PWD/VERSION
        '';

        # Function to fix Darwin binaries to use system libiconv
        fixDarwinBinary = name: binary: pkgs.runCommand name {} ''
          mkdir -p $out/bin
          cp ${binary}/bin/ki $out/bin/
          chmod +w $out/bin/ki
          ${pkgs.darwin.cctools}/bin/install_name_tool -change "/nix/store/phzzjrksk8nnmjsbrpbkvv4pr383ab6v-libiconv-109/lib/libiconv.2.dylib" "/usr/lib/libiconv.2.dylib" $out/bin/ki
        '';

        # Function to build for a specific target
        mkCrossPackage = {
          targetSystem,
          rustTarget,
          extraBuildInputs ? [],
          extraNativeBuildInputs ? [],
          extraRustFlags ? [],
          extraEnv ? {}
        }:
          let
            # Set up the toolchain for the target
            crossToolchain = mkRustToolchain [ rustTarget ];

            # Create a crane lib instance with the cross toolchain
            crossCraneLib = (crane.mkLib pkgs).overrideToolchain crossToolchain;

            # Determine if we're building for Windows
            isWindows = builtins.match ".*windows.*" rustTarget != null;

            # Determine if we're building for Darwin
            isDarwin = builtins.match ".*darwin.*" rustTarget != null;

            # Determine if we're building for Linux
            isLinux = builtins.match ".*linux.*" rustTarget != null;

            # Get the cross pkgs for the target
            crossPkgs = if isWindows then
              pkgs.pkgsCross.mingwW64
            else if rustTarget == "x86_64-unknown-linux-musl" then
              pkgs.pkgsCross.musl64
            else
              pkgs;

            # Common arguments for all builds
            crossArgs = {
              # Use a custom source filtering to include necessary files
              src = pkgs.lib.cleanSourceWith {
                src = ./.;
                filter = path: type:
                  (crossCraneLib.filterCargoSources path type) ||
                  (builtins.match ".*contrib/emoji-icon-theme.json$" path != null) ||
                  (builtins.match ".*tree_sitter_quickfix/src/.*$" path != null);
              };

              # Add a preBuild phase to create the VERSION file
              preBuildPhases = [ "createVersionPhase" ];
              createVersionPhase = createVersionFile;

              strictDeps = true;
              doCheck = false;

              # Explicitly disable --locked flag
              cargoExtraArgs = "";
              cargoCheckExtraArgs = "";

              # Set the cargo build target
              CARGO_BUILD_TARGET = rustTarget;

              # Static linking environment variables
              OPENSSL_STATIC = "1";
              LIBICONV_STATIC = "1";
            } // extraEnv;

            # Platform-specific arguments
            platformArgs = crossArgs // {
              # Static linking environment variables
              OPENSSL_LIB_DIR = if isWindows then
                "${crossPkgs.openssl.out}/lib"
              else if isLinux then
                "${crossPkgs.pkgsStatic.openssl.out}/lib"
              else
                "${pkgs.pkgsStatic.openssl.out}/lib";

              OPENSSL_INCLUDE_DIR = if isWindows then
                "${crossPkgs.openssl.dev}/include"
              else if isLinux then
                "${crossPkgs.pkgsStatic.openssl.dev}/include"
              else
                "${pkgs.pkgsStatic.openssl.dev}/include";

              # Static linking flags
              CARGO_BUILD_RUSTFLAGS = [
                "-C" "target-feature=+crt-static"
              ] ++ (if isDarwin then [
                "-C" "link-arg=-static-libgcc"
                # Use system libiconv instead of trying to statically link it
                "-C" "link-arg=-Wl,-search_paths_first"
                "-C" "link-arg=-Wl,-dead_strip"
                "-C" "link-arg=-Wl,-rpath,/usr/lib"
                "-C" "link-arg=-liconv"
              ] else if isLinux then [
                "-C" "link-arg=-static"
                "-C" "link-arg=-latomic"
                "-C" "linker=${crossPkgs.stdenv.cc}/bin/${crossPkgs.stdenv.cc.targetPrefix}cc"
              ] else if isWindows then [
                "-C" "link-arg=-static"
                "-C" "link-arg=-static-libgcc"
                "-C" "linker=${crossPkgs.stdenv.cc}/bin/${crossPkgs.stdenv.cc.targetPrefix}cc"
              ] else []) ++ extraRustFlags;

              # Native build inputs common to all targets
              nativeBuildInputs = with pkgs; [
                pkg-config
                git
                nodejs
              ] ++ extraNativeBuildInputs;

              # Build inputs
              buildInputs = (if isDarwin then with pkgs; [
                openssl
                darwin.apple_sdk.frameworks.Security
                darwin.apple_sdk.frameworks.SystemConfiguration
                darwin.apple_sdk.frameworks.CoreServices
                darwin.apple_sdk.frameworks.CoreFoundation
              ] else if isLinux then with crossPkgs; [
                openssl.dev
                openssl.out
                pkgsStatic.libiconv
              ] else if isWindows then with crossPkgs; [
                openssl.dev
                openssl.out
                windows.pthreads
              ] else []) ++ extraBuildInputs;
            };

            # Windows-specific linker settings
            windowsArgs = if isWindows then {
              "CC_${builtins.replaceStrings ["-"] ["_"] rustTarget}" =
                "${crossPkgs.stdenv.cc}/bin/${crossPkgs.stdenv.cc.targetPrefix}cc";
              "CARGO_TARGET_${pkgs.lib.toUpper (builtins.replaceStrings ["-"] ["_"] rustTarget)}_LINKER" =
                "${crossPkgs.stdenv.cc}/bin/${crossPkgs.stdenv.cc.targetPrefix}cc";
            } else {};

            # Linux musl-specific linker settings
            muslArgs = if rustTarget == "x86_64-unknown-linux-musl" then {
              "CC_x86_64_unknown_linux_musl" = "${crossPkgs.stdenv.cc}/bin/${crossPkgs.stdenv.cc.targetPrefix}cc";
              "CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER" = "${crossPkgs.stdenv.cc}/bin/${crossPkgs.stdenv.cc.targetPrefix}cc";
            } else {};

            # Combine all arguments
            finalArgs = platformArgs // windowsArgs // muslArgs;

            # Build dependencies first
            crossDeps = crossCraneLib.buildDepsOnly finalArgs;
          in
          crossCraneLib.buildPackage (finalArgs // {
            cargoArtifacts = crossDeps;
          });

        # Build for native architecture
        ki-editor = mkCrossPackage {
          targetSystem = localSystem;
          rustTarget = if pkgs.stdenv.isDarwin then
            if pkgs.stdenv.hostPlatform.isAarch64 then "aarch64-apple-darwin" else "x86_64-apple-darwin"
          else if pkgs.stdenv.isLinux then
            "x86_64-unknown-linux-gnu"
          else
            throw "Unsupported native system";
        };

        # Build for aarch64-darwin
        aarch64-darwin-ki = mkCrossPackage {
          targetSystem = "aarch64-darwin";
          rustTarget = "aarch64-apple-darwin";
        };

        # Build for x86_64-linux-musl
        x86_64-linux-musl-ki = mkCrossPackage {
          targetSystem = "x86_64-linux";
          rustTarget = "x86_64-unknown-linux-musl";
        };

        # Build for x86_64-windows-gnu
        x86_64-windows-gnu-ki = mkCrossPackage {
          targetSystem = "x86_64-windows";
          rustTarget = "x86_64-pc-windows-gnu";
          extraNativeBuildInputs = [
            pkgs.pkgsCross.mingwW64.stdenv.cc
          ];
        };
      in
      {
        packages = {
          default = if pkgs.stdenv.isDarwin then
            fixDarwinBinary "ki-fixed-default" ki-editor
          else
            ki-editor;
          "aarch64-darwin" = fixDarwinBinary "ki-fixed" aarch64-darwin-ki;
          "x86_64-linux-musl" = x86_64-linux-musl-ki;
          "x86_64-windows-gnu" = x86_64-windows-gnu-ki;
        };

        devShells.default = (crane.mkLib pkgs).devShell {
          packages = with pkgs; [
            rust-analyzer
            pkg-config
            openssl
          ];
        };
      }
    );
}

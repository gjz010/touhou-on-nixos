{
  description = "A very basic flake";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
    gitignore = {
      url = "github:hercules-ci/gitignore.nix";
      # Use the same nixpkgs
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, gitignore }: 
  let 
    pkgs = import nixpkgs {system = "x86_64-linux"; crossSystem = nixpkgs.lib.systems.examples.mingw32; overlays = [rust-overlay.overlays.default]; };
    pkgsNative = import nixpkgs {system = "x86_64-linux";overlays = [rust-overlay.overlays.default];};
    inherit (gitignore.lib) gitignoreSource;
  in 
  {
    packages.x86_64-linux.thcrap2nix = pkgsNative.callPackage ({rust-bin}:
    let rustPlatform = pkgs.makeRustPlatform {
      cargo = pkgs.pkgsBuildHost.rust-bin.stable.latest.complete;
      rustc = pkgs.pkgsBuildHost.rust-bin.stable.latest.complete;
    };
    stdenv = pkgs.stdenv;
    jansson = pkgs.jansson;
    windows = pkgs.windows;
    in rustPlatform.buildRustPackage rec {
      name = "thcrap2nix";
      src = gitignoreSource ./thcrap2nix;
      cargoLock = {
        lockFile = ./thcrap2nix/Cargo.lock;
      };
      #buildPhase = ''
      #  cargo build --release --config .cargo/config.toml --verbose
      #'';
      nativeBuildInputs = [ pkgsNative.libclang pkgsNative.winePackages.staging];
      buildInputs = [ windows.pthreads windows.mcfgthreads stdenv.cc.libc jansson];
      LIBCLANG_PATH="${pkgsNative.libclang.lib}/lib";
    }
    ) {};
    packages.x86_64-linux.hello = nixpkgs.legacyPackages.x86_64-linux.hello;

    packages.x86_64-linux.default = self.packages.x86_64-linux.hello;
    devShells.x86_64-linux.default = 
      pkgs.callPackage ({mkShell, stdenv, rust-bin, windows, jansson}: mkShell {
          #buildInputs = [pkgs.rust-bin.stable.latest.minimal];
          #CARGO_TARGET_I686_PC_WINDOWS_GNU_LINKER = "${stdenv.cc.targetPrefix}cc";
          nativeBuildInputs = [ pkgs.pkgsBuildHost.rust-bin.stable.latest.complete pkgsNative.libclang pkgsNative.winePackages.staging];
          buildInputs = [ windows.pthreads windows.mcfgthreads stdenv.cc.libc jansson];
          LIBCLANG_PATH="${pkgsNative.libclang.lib}/lib";
          WINEPATH="${jansson}/bin;${windows.mcfgthreads}/bin;../thcrap/bin";
          HOST_SSL_CERT_FILE="${pkgsNative.cacert}/etc/ssl/certs/ca-bundle.crt";
      }) {};

  };
}

{
  description = "A very basic flake";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }: {

    packages.x86_64-linux.hello = nixpkgs.legacyPackages.x86_64-linux.hello;

    packages.x86_64-linux.default = self.packages.x86_64-linux.hello;
    devShells.x86_64-linux.default = 
      let 
      pkgs = import nixpkgs {system = "x86_64-linux"; crossSystem = nixpkgs.lib.systems.examples.mingw32; overlays = [rust-overlay.overlays.default]; };
      pkgsNative = import nixpkgs {system = "x86_64-linux";};
      in 
      pkgs.callPackage ({mkShell, stdenv, rust-bin, windows, jansson}: mkShell {
          #buildInputs = [pkgs.rust-bin.stable.latest.minimal];
          CARGO_TARGET_I686_PC_WINDOWS_GNU_LINKER = "${stdenv.cc.targetPrefix}cc";
          nativeBuildInputs = [ rust-bin.stable.latest.complete pkgsNative.libclang pkgsNative.winePackages.staging];
          buildInputs = [ windows.pthreads windows.mcfgthreads stdenv.cc.libc jansson];
          LIBCLANG_PATH="${pkgsNative.libclang.lib}/lib";
      }) {};

  };
}

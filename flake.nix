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
    pkgsWin = import nixpkgs {system = "x86_64-linux"; crossSystem = nixpkgs.lib.systems.examples.mingw32; overlays = [rust-overlay.overlays.default]; };
    pkgs = import nixpkgs {system = "x86_64-linux";overlays = [rust-overlay.overlays.default];};
    inherit (gitignore.lib) gitignoreSource;
  in 
  {
    packages.x86_64-linux = rec {
      thcrap2nix = pkgsWin.callPackage ./thcrap2nix {winePackageNative = pkgs.winePackages.staging; inherit gitignoreSource; };
      touhouTools = rec {
        thcrap = pkgs.callPackage ({stdenvNoCC, unzip, fetchurl}:
          stdenvNoCC.mkDerivation {
            name = "thcrap-bin";
            version = "2023-08-30";
            src = fetchurl {
              url = "https://github.com/thpatch/thcrap/releases/download/2023-08-30/thcrap.zip";
              sha256 = "XdJTmVNTa16gcq7gipP7AeYxvD1+K9n4u4kJafeXv5c=";
            };
            nativeBuildInputs = [unzip];
            unpackPhase = ''
              unzip $src
            '';
            installPhase = ''
              mkdir -p $out
              cp -r ./bin ./repos $out
            '';
          }) {};
        thcrapPatches = {
          lang_zh-hans = {repo_id = "thpatch"; patch_id = "lang_zh-hans";};
        };
        thcrapDown = { sha256? "", patches, games}: 
          let cfg = {patches = patches thcrapPatches; inherit games;}; 
          cfgFile = pkgs.writeText "thcrap2nix.json" (builtins.toJSON cfg);
          in
          pkgs.stdenvNoCC.mkDerivation {
            name = "thcrap-config";
            nativeBuildInputs = [pkgs.wine];
            outputHashMode = "recursive";
            outputHashAlgo = "sha256";
            outputHash = sha256;
            phases = ["buildPhase"];
            buildPhase = ''
              export BUILD=$PWD
              mkdir .wine
              export WINEPREFIX=$BUILD/.wine
              mkdir -p $BUILD/bin
              for i in ${thcrap}/bin/*; do
                ln -s $i $BUILD/bin/
              done
              cp -r ${thcrap}/repos $BUILD
              chmod -R 777 $BUILD/repos
              for i in ${thcrap2nix}/bin/*; do
                ln -s $i $BUILD/bin/
              done
              ln -s ${pkgsWin.jansson}/bin/libgcc* $BUILD/bin/
              wine wineboot
              echo "Wineboot finished."
              ls $BUILD/bin -alh
              export RUST_LOG=trace
              wine $BUILD/bin/thcrap2nix.exe ${cfgFile}
              mkdir -p $out/config
              cp -r $BUILD/repos $out
              cp $BUILD/thcrap2nix.js $out/config
            ''; 
        };
        thcrapDownExample = thcrapDown {
          patches = (p: with p; [lang_zh-hans]);
          games = ["th16"];
          sha256 = "xHX3FIjaG5epe+N3oLkyP4L7h01eYjiHjTXU39QuSpA=";
        };
      };
    };

    packages.x86_64-linux.hello = nixpkgs.legacyPackages.x86_64-linux.hello;

    packages.x86_64-linux.default = self.packages.x86_64-linux.hello;
    devShells.x86_64-linux.default = 
      pkgsWin.callPackage ({mkShell, stdenv, rust-bin, windows, jansson}: mkShell {
          #buildInputs = [pkgs.rust-bin.stable.latest.minimal];
          #CARGO_TARGET_I686_PC_WINDOWS_GNU_LINKER = "${stdenv.cc.targetPrefix}cc";
          nativeBuildInputs = [ pkgsWin.pkgsBuildHost.rust-bin.stable.latest.complete pkgs.libclang pkgs.winePackages.staging];
          buildInputs = [ windows.pthreads windows.mcfgthreads stdenv.cc.libc jansson];
          LIBCLANG_PATH="${pkgs.libclang.lib}/lib";
          WINEPATH="${jansson}/bin;${windows.mcfgthreads}/bin;../thcrap/bin";
          HOST_SSL_CERT_FILE="${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt";
      }) {};

  };
}

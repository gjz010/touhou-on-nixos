{jansson,
windows,
stdenv,
makeRustPlatform,
winePackageNative,
pkgsBuildBuild,
pkgsBuildHost,
gitignoreSource}:
let rustPlatform = makeRustPlatform {
  cargo = pkgsBuildHost.rust-bin.stable.latest.complete;
  rustc = pkgsBuildHost.rust-bin.stable.latest.complete;
};
in
rustPlatform.buildRustPackage rec {
    name = "thcrap2nix";
    src = gitignoreSource ./.;
    cargoLock = {
      lockFile = ./Cargo.lock;
    };
    #buildPhase = ''
    #  cargo build --release --config .cargo/config.toml --verbose
    #'';
    nativeBuildInputs = [ pkgsBuildBuild.libclang winePackageNative];
    buildInputs = [ windows.pthreads windows.mcfgthreads stdenv.cc.libc jansson];
    LIBCLANG_PATH="${pkgsBuildBuild.libclang.lib}/lib";
}
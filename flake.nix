{
  description = "A very basic flake";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs";
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
        makeWinePrefix = {
          defaultFont? "Noto Sans CJK SC",
          fontPackage?  pkgs.noto-fonts-cjk-sans
        }:
        (pkgs.callPackage ({stdenvNoCC, wine, pkgsCross, bash}:  stdenvNoCC.mkDerivation {
          name = "touhou-wineprefix";
          nativeBuildInputs = [wine];
          phases = ["installPhase"];
          installPhase = ''
          export WINEPREFIX=$out/share/wineprefix
          mkdir -p $WINEPREFIX
          wineboot -i
          wineserver --wait || true
          echo Setting up dxvk.
          dxvk32_dir=${pkgsCross.mingw32.dxvk_2}/bin mcfgthreads32_dir=${pkgsCross.mingw32.windows.mcfgthreads_pre_gcc_13}/bin ${bash}/bin/bash ${./setup_dxvk.sh}
          echo dxvk installed.
          wineserver --wait || true
          echo "${defaultFont}" > $out/share/wineprefix/default_font.txt
          find ${fontPackage} -type f -name "*.ttc" -exec cp {} $out/share/wineprefix/drive_c/windows/Fonts/ \;
          '';
        }) {});
        defaultWinePrefix = makeWinePrefix {};
        makeTouhou = {
          thVersion,
          name? thVersion,
          enableVpatch? true,
          enableThprac? true,
          thcrapPatches? null,
          thcrapSha256? "",
          baseDrv? null,
          winePrefix? defaultWinePrefix
        }: 

        pkgs.callPackage ({stdenvNoCC, lib, bash, makeWrapper, writeScript, wine, bubblewrap, iconv, dxvk}: 
        let pkgname = "${name}-wrapper";
        in
        stdenvNoCC.mkDerivation {
          name = pkgname;
          gameExe = "${thVersion}.exe";
          inherit thVersion;
          phases = ["installPhase"];
          nativeBuildInputs = [makeWrapper];
          thcrapPath = if thcrapPatches != null then thcrap else "";
          thcrapConfigPath = if thcrapPatches != null then thcrapDown {
            sha256 = thcrapSha256;
            patches = thcrapPatches;
            games = [thVersion];
          } else "";
          thpracPath = if enableThprac then thprac else "";
          vpatchPath = if enableVpatch then vpatch else "";
          baseDrv = if baseDrv!=null then baseDrv else "";
          inherit enableThprac;
          inherit enableVpatch;
          enableThcrap = thcrapPatches != null;
          enableBase = baseDrv!=null;
          launcherScriptBwrap = writeScript "${pkgname}-script-bwrap" ''
          #!${bash}/bin/bash
          export PATH=${wine}/bin:$PATH
          touhouRoot="$wrapperRoot/base"
          mutableBase="$HOME/.config/.touhou-on-nixos/${name}"
          if [ -z "$enableBase" ]; then
            touhouRoot="$PWD"
          fi
          wineprefixMount="--bind $WINEPREFIX /opt/wineprefix"
          if [ -z "$WINEPREFIX" ]; then
            WINEPREFIX="${winePrefix}/share/wineprefix"
            export COPY_WINEPREFIX=1
            wineprefixMount="--ro-bind $WINEPREFIX /opt/wineprefix"
          fi
          mkdir -p "$mutableBase"
          touch "$mutableBase/score.dat"
          touch "$mutableBase/${thVersion}.cfg"
          thcrapMount=""
          vpatchMount=""
          thpracMount=""
          if ! [ -z $enableThcrap ]; then
            mkdir "$mutableBase/thcrap-logs"
            thcrapMount="--ro-bind \"$wrapperRoot/thcrap\" /opt/thcrap/ --bind \"$mutableBase/thcrap-logs\" /opt/thcrap/logs"
          fi
          if ! [ -z $enableVpatch ]; then
            touch "$mutableBase/vpatch.ini"
            vpatchMount="--ro-bind \"$wrapperRoot/vpatch.exe\" /opt/touhou/vpatch.exe --ro-bind \"$wrapperRoot/vpatch_${thVersion}.dll\" /opt/touhou/vpatch_${thVersion}.dll --bind \"$mutableBase/vpatch.ini\" /opt/touhou/vpatch.ini"
          fi
          if ! [ -z $enableThprac ]; then
            thpracMount="--ro-bind \"$wrapperRoot/thprac.exe\" /opt/touhou/thprac.exe"
          fi
          touhouBaseMount=""
          for f in "$touhouRoot/"*; do
            fbase=$(basename "$f")
            touhouBaseMount="--ro-bind \"$f\" \"/opt/touhou/$fbase\" $touhouBaseMount"
          done
          mutableMount="--bind \"$mutableBase/score.dat\" /opt/touhou/score.dat --bind \"$mutableBase/${thVersion}.cfg\" /opt/touhou/${thVersion}.cfg"
          if [ ${thVersion} == "th18" ]; then
            mkdir -p "$mutableBase/appdata"
            mutableMount="--bind \"$mutableBase/appdata\" /opt/ShanghaiAlice/th18"
          fi
          bash -c "LAUNCH_WITH_BWRAP=1 XAUTHORITY=/opt/.Xauthority WINEPREFIX=/opt/wineprefix ${bubblewrap}/bin/bwrap \
            --ro-bind /nix /nix --proc /proc --dev-bind /dev /dev --bind /sys /sys --tmpfs /tmp --tmpfs /opt \
            $wineprefixMount \
            --ro-bind $XAUTHORITY /opt/.Xauthority \
            --ro-bind /tmp/.X11-unix /tmp/.X11-unix \
            --bind /run /run --ro-bind /var /var --ro-bind /bin /bin \
            $touhouBaseMount $thcrapMount $thpracMount $vpatchMount $mutableMount \
            --chdir /opt/touhou \
            $wrapperPath/bin/${pkgname}-raw"
          '';
          launcherScript = writeScript "${pkgname}-script" ''
          #!${bash}/bin/bash
          LAUNCHPATH=$PWD
          # Note: copying thprac and vpatch is for debugging purpose only!
          if ! [ -z $enableThprac ]; then
            if ! [ -e "$LAUNCHPATH/thprac.exe" ]; then
              echo Copying thprac.exe
              ln -s $wrapperRoot/thprac.exe "$LAUNCHPATH/thprac.exe"
            fi
          fi
          if ! [ -z $enableVpatch ]; then
            if ! [ -e "$LAUNCHPATH/vpatch.exe" ]; then
              echo Copying vpatch.exe
              ln -s $wrapperRoot/vpatch.exe "$LAUNCHPATH/vpatch.exe"
              ln -s $wrapperRoot/vpatch*.dll "$LAUNCHPATH/"
            fi
          fi
          if ! [ -z $COPY_WINEPREFIX ]; then
            echo "Copying wineprefix."
            cp -r $WINEPREFIX /tmp/wineprefix
            chmod -R 777 /tmp/wineprefix
            WINEPREFIX=/tmp/wineprefix
            ls -alh $WINEPREFIX
            if [ -e $WINEPREFIX/default_font.txt ]; then
              font=$(cat $WINEPREFIX/default_font.txt)
              echo Setting font to $font
              regc="REGEDIT4

[HKEY_CURRENT_USER\\Software\\Wine\\Fonts\\Replacements]
\"PMingLiU\"=\"$font\"
\"ＭＳ ゴシック\"=\"$font\"
"

              echo -e -n "\xff\xfe" > "/tmp/thcfg.reg"
              ${iconv}/bin/iconv --from-code UTF-8 --to-code UTF-16LE <(echo "$regc") >> "/tmp/thcfg.reg"

              ${wine}/bin/wine regedit "/tmp/thcfg.reg"
            fi
            #echo Setting up dxvk.
            #${dxvk}/bin/setup_dxvk.sh install
            #echo dxvk installed.
          fi
          # Set executable.
          if ! [ -z $enableThprac ]; then
            gameExe="thprac.exe" # thprac.exe can find vpatch on its own.
          elif ! [ -z $enableVpatch ]; then
            gameExe="vpatch.exe"
          fi
          if ! [ -e "$LAUNCHPATH/$gameExe" ]; then
            echo "gameExe not found: $gameExe"
            exit 1
          fi
          if ! [ -z $enableThcrap ]; then
            if ! [ -z $LAUNCH_WITH_BWRAP ]; then
              cd /opt/thcrap
            else
              cd "$wrapperRoot/thcrap"
            fi
            ${wine}/bin/wine bin/thcrap_loader.exe thcrap2nix.js "$LAUNCHPATH/$gameExe"
          else
            ${wine}/bin/wine "$LAUNCHPATH/$gameExe"
          fi
          '';
          installPhase = ''
          mkdir -p $out/bin
          mkdir -p $out/share/thcrap-wrapper
          wrapperRoot=$out/share/thcrap-wrapper
          echo Linking all files in base derivation.
          if ! [ -z $baseDrv ]; then
            ln -s $baseDrv $wrapperRoot/base
          else
            echo Base derivation is empty.
          fi
          if ! [ -z $vpatchPath ]; then
            echo Applying vpatch.
            if [ -e $vpatchPath/bin/vpatch_$thVersion.dll ] ; then
              ln -s $vpatchPath/bin/vpatch_$thVersion.dll $wrapperRoot
              ln -s $vpatchPath/bin/vpatch.exe $wrapperRoot
            else
              echo Corresponding Vpatch not found!
              enableVpatch=""
            fi
          fi
          if ! [ -z $thcrapPath ]; then
            echo Applying thcrap.
            mkdir -p $wrapperRoot/thcrap/bin
            mkdir -p $wrapperRoot/thcrap/logs
            ln -s $thcrapPath/bin/* $wrapperRoot/thcrap/bin/
            ln -s $thcrapConfigPath/* $wrapperRoot/thcrap/
            rm $wrapperRoot/thcrap/bin/thcrap_update.dll
          fi
          if ! [ -z $thpracPath ]; then
            echo Applying thprac.
            ln -s $thpracPath $wrapperRoot/thprac.exe
          fi
          echo Creating wrapper script.
          ln -s $launcherScript $out/bin/$name-raw
          wrapProgram $out/bin/$name-raw --set enableThprac "$enableThprac" --set enableVpatch "$enableVpatch" --set enableThcrap "$enableThcrap" --set gameExe "$gameExe" --set wrapperRoot "$wrapperRoot"
          ln -s $launcherScriptBwrap $out/bin/$name 
          wrapProgram $out/bin/$name --set wrapperPath "$out" --set wrapperRoot "$wrapperRoot" \
            --set enableThprac "$enableThprac" --set enableVpatch "$enableVpatch" --set enableThcrap "$enableThcrap" \
            --set enableBase "$enableBase"
          echo Done.
          '';
        }) { };
        vpatch = pkgs.callPackage ({stdenvNoCC, unzip, fetchurl}:
          stdenvNoCC.mkDerivation{
            name = "vsyncpatch-bin";
            version = "2015-11-28";
            src = fetchurl {
              url = "https://maribelhearn.com/mirror/VsyncPatch.zip";
              sha256 = "sha256-XVmbdzF6IIpRWQiKAujWzy6cmA8llG34jkqUb29Ec44=";
              # https://web.archive.org/web/20220824223436if_/https://maribelhearn.com/mirror/VsyncPatch.zip
            };
            nativeBuildInputs = [unzip];
            unpackPhase = ''
              unzip $src
            '';
            installPhase = ''
              mkdir -p $out/bin
              cp vpatch/vpatch_rev4/vpatch.exe $out/bin
              cp vpatch/vpatch_rev4/*.dll $out/bin
              cp vpatch/vpatch_rev7/*.dll $out/bin
              cp vpatch/vpatch_th12.8/*.dll $out/bin
              cp vpatch/vpatch_th13/*.dll $out/bin
              cp vpatch/vpatch_th14/*.dll $out/bin
              cp vpatch/vpatch_th15/*.dll $out/bin
            '';
          }
        ) {};
        makeTouhouOverlay = args: makeTouhou (args // {baseDrv = null;});
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
        thprac = pkgs.fetchurl {
          url = "https://github.com/touhouworldcup/thprac/releases/download/v2.2.1.4/thprac.v2.2.1.4.exe";
          sha256 = "sha256-eIfkABD0Wfg0/NjtfMO+yjfZFvF7oLfUjOaR0pkv1FM=";
        };
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
            impureEnvVars = [ "http_proxy" "https_proxy" ];
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
              export RUST_LOG=trace
              export http_proxy=garbage://site
              export https_proxy=garbage://site
              export NO_PROXY="thpatch.net,thpatch.rcopky.top"
              wine $BUILD/bin/thcrap2nix.exe ${cfgFile}
              mkdir -p $out/config
              cp -r $BUILD/repos $out
              cp $BUILD/thcrap2nix.js $out/config
            ''; 
        };


      };
      examples = {
        thcrapDownExample = touhouTools.thcrapDown {
          patches = (p: with p; [lang_zh-hans]);
          games = ["th16"];
          sha256 = "xHX3FIjaG5epe+N3oLkyP4L7h01eYjiHjTXU39QuSpA=";
        };
        th07 = touhouTools.makeTouhouOverlay {
          thVersion = "th07";
          thcrapPatches = (p: with p; [lang_zh-hans]);
          thcrapSha256 = "6Z32LxSWnAZRe7zeCsABQUNSfXOoLoaKdnpZrg4a9Fc=";
        };
        th18 = touhouTools.makeTouhouOverlay {
          thVersion = "th18";
          thcrapPatches = (p: with p; [lang_zh-hans]);
          thcrapSha256 = "U6ZmBefxTsRm+kuzga/KzQN5FAg381d9/CZMczY59ss=";
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

Touhou on NixOS
==================

Packaging Touhou project and utilities into deterministic derivation.

- Wineprefix with Noto fonts and DXVK carefully bubblewrapped. All saves/config stored nice in `~/.config/touhou-on-nixos/`. This effectively makes game root readonly.
- Nixified thcrap patches.
- Thprac and vpatch support.


Usage
------------------

1. Bring your own Touhou game, for example `th06`. Note that you should use a clean copy, e.g. the one you use for thcrap. Your Touhou game will not go into Nix store, but will be bind-mounted into bubblewrapped environment.
2. Switch to game root and `nix run github:gjz010/touhou-on-nixos#zh_CN.th06`.


Known issues
------------------

### TODO list

- Wayland/X11 smart switching.

### Thcrap patches from `thpatch.net`

- `thpatch.net` does not provide permalinks/content-addresses links to patch assets, making it impossible to create a deterministic derivation.
- Mirrors of `thpatch.net` also have the same issue and are not always synced. For example, I get completely different hashes when switching from mirrors back and forth. Sometimes, some `thpatch.net` mirrors even host broken metadata.
- The repo discovery behaviour is also weird: repo discovery starts from `thpatch.net` to GitHub, only to find a mirror exists on `thpatch.net`, which totally defeating the purpose of mirroring. Therefore we only allow (probably auto-generated) downloading from `thpatch.net` and `thpatch.rcopky.top` by dirty hacking: providing an invalid `http_proxy` and a whitelist of `no_proxy`.
- Every user has to run `thcrap2nix` (which is a bad name since it does not produce Nix, but fixed output derivation) to download patches. The utility is dirtyly linked to `thcrap` itself and is therefore 32bit Wine only.



(20241011 notes: while `lilywhite.cc` provides fast downloading for Chinese users, some important metadata files like `tpZHCNex/tsa/patch.js` adds it as "first server of patch", which is then used to fetch `patch.js`. We have no way but to add `lilywhite.cc` to the whitelist.)

One possible solution to all the dirty issues above is to create and maintain a content-addressable mirror (e.g. like a git repository) by ourselves. We could even create our own metadata during sync with thpatch, such that `thcrap2nix` is only needed to be run by CI, and `touhou-on-nixos` users can use generated thcrap patch derivations.

Possible problems:
- Current dirty `thcrap2nix` is still too dirty. We may want to rollout our own thcrap patch downloader.
- `thpatch.net` repo is large. I once tried downloading entire `mirrors.thpatch.net` and it consumed ~20 GB of total size.
- Creating `thpatch.net` mirror may face legal issues like copyright infringement.
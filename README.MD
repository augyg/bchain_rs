Instructions:

this is ideally built to also show off nix, this is easy to download:

Linux:
$ sh <(curl -L https://nixos.org/nix/install) --no-daemon

Mac:
$ sh <(curl -L https://nixos.org/nix/install) 

Windows: 
:(



Then you can install this package by running

```bash
cd bchain/B_Cli
nix-build && nix-env -i ./result
b_cli <your command> # per email instructions
```

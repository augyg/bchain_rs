let
  systemPkgs = import <nixpkgs> {};
  pinnedNixpkgs = systemPkgs.fetchFromGitHub {
    owner = "NixOS";
    repo = "nixpkgs";
    # 23.11; https://github.com/NixOS/nixpkgs/commit/383ffe076d9b633a2e97b6e4dd97fc15fcf30159
    rev = "383ffe076d9b633a2e97b6e4dd97fc15fcf30159";
    sha256 = "sha256-Q4ddhb5eC5pwci0QhAapFwnsc8X8H9ZMQiWpsofBsDc="; # result of >> systemPkgs.lib.fakeSha256;
  };
  pkgs = import pinnedNixpkgs {};

in pkgs.mkShell {
  buildInputs = [ pkgs.cargo pkgs.rustc pkgs.openssl ];
}

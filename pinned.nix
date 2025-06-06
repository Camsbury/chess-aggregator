let
  inherit (import <nixpkgs> {}) fetchFromGitHub;
in
  import (fetchFromGitHub {
    owner = "NixOS";
    repo  = "nixpkgs";
    rev = "102a39bfee44";
    hash = "sha256-Q4vhtbLYWBUnjWD4iQb003Lt+N5PuURDad1BngGKdUs=";
})

let
  inherit (import <nixpkgs> {}) fetchFromGitHub;
in
  import (fetchFromGitHub {
    owner = "NixOS";
    repo  = "nixpkgs";
    rev = "1b1f50645af2a70dc93eae18bfd88d330bfbcf7f";
    hash = "sha256-L0UZl/u2H3HGsrhN+by42c5kNYeKtdmJiPzIRvEVeiM=";
})

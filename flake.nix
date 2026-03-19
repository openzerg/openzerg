{
  description = "OpenZerg - Agent for Zerg Swarm";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.11";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
  };

  outputs = { self, nixpkgs, flake-utils, crane, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        craneLib = crane.mkLib pkgs;

        src = craneLib.cleanCargoSource ./.;

        commonArgs = {
          inherit src;
          nativeBuildInputs = [ pkgs.pkg-config ];
          buildInputs = with pkgs; [ openssl ];
        };

        cargoArtifacts = craneLib.buildDepsOnly (commonArgs // {
          pname = "openzerg-deps";
        });

        openzerg = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
          pname = "openzerg";
          cargoExtraArgs = "--bin openzerg";
          doCheck = false;
        });

      in
      {
        packages = {
          inherit openzerg;
          default = openzerg;
        };

        devShells.default = craneLib.devShell {
          inherit src;
          inputsFrom = [ openzerg ];
          packages = with pkgs; [
            rust-analyzer
            cargo-watch
            cargo-llvm-cov
          ];
          shellHook = ''
            export LLVM_COV="${pkgs.llvmPackages_19.llvm}/bin/llvm-cov"
            export LLVM_PROFDATA="${pkgs.llvmPackages_19.llvm}/bin/llvm-profdata"
          '';
        };
      }
    ) // {
      overlays.default = final: prev: {
        openzerg = self.packages.${final.system}.openzerg;
      };

      nixosModules.default = { config, lib, pkgs, ... }: {
        imports = [ ./modules/openzerg.nix ];
        config = lib.mkIf config.services.openzerg.enable {
          services.openzerg.package = lib.mkDefault self.packages.${pkgs.system}.openzerg;
        };
      };
    };
}
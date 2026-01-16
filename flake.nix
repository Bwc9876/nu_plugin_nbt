{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flakelight.url = "github:nix-community/flakelight";
    flakelight.inputs.nixpkgs.follows = "nixpkgs";
    crane.url = "github:ipetkov/crane";
    fenix.url = "github:nix-community/fenix";
  };
  outputs = {
    self,
    nixpkgs,
    flakelight,
    crane,
    fenix,
    ...
  } @ inputs: let
    selectToolchain = fenix: fenix.default;
    mkCrane = pkgs: let
      inherit (selectToolchain pkgs.fenix) toolchain;
      craneLib = (crane.mkLib nixpkgs.legacyPackages.${pkgs.system}).overrideToolchain toolchain;
      rawSrc = ./.;
      src = craneLib.cleanCargoSource rawSrc;
      commonArgs = {
        inherit src;
        strictDeps = true;
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;
      };
      crate = craneLib.buildPackage (
        commonArgs
        // {
          inherit (craneLib.crateNameFromCargoToml {inherit src;}) version;
          doCheck = false;
        }
      );
    in {
      inherit crate craneLib commonArgs;
    };
  in
    flakelight ./. {
      inherit inputs;
      pname = "nu_plugin_nbt";
      nixpkgs.overlays = [fenix.overlays.default];
      package = pkgs: (mkCrane pkgs).crate;
      formatters = pkgs: let
        alejandra = "${pkgs.lib.getExe pkgs.alejandra} .";
        rustfmt = "${(selectToolchain pkgs.fenix).rustfmt}/bin/rustfmt .";
        taplo = "${pkgs.lib.getExe pkgs.taplo} fmt .";
      in {
        "*.nix" = alejandra;
        "*.rs" = rustfmt;
        "*.toml" = taplo;
      };
      checks = pkgs: let
        inherit (mkCrane pkgs) craneLib commonArgs;
      in {
        clippy = craneLib.cargoClippy (
          commonArgs
          // {
            cargoClippyExtraArgs = "--all-targets -- --deny warnings";
          }
        );
        test = craneLib.cargoNextest (
          commonArgs
          // {
            partitions = 1;
            partitionType = "count";
            cargoNextestPartitionsExtraArgs = "--no-tests=pass";
          }
        );
      };
      devShell = pkgs:
        (mkCrane pkgs).craneLib.devShell {
          checks = self.checks.${pkgs.system};

          packages = with pkgs; [cargo-nextest];
        };
    };
}

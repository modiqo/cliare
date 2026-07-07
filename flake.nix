{
  description = "CLI agent-readiness measurement, command-shape inference, and CI scorecards";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

  outputs = { self, nixpkgs }: let
    version = "0.1.9";

    assets = {
      x86_64-linux = {
        file = "cliare-x86_64-unknown-linux-gnu.tar.gz";
        sha256 = "sha256-Z0E8ss/NcOTWHtVXxNZ0oeUdYAbCt11bRIHognmUxcY=";
      };
      aarch64-linux = {
        file = "cliare-aarch64-unknown-linux-gnu.tar.gz";
        sha256 = "sha256-Y0C1rN34V9SvW1CfUd3sADv6X7zpEVr1bNiYrKAbGyI=";
      };
      x86_64-darwin = {
        file = "cliare-x86_64-apple-darwin.tar.gz";
        sha256 = "sha256-wzp+vk/rwHaNWfKcPQzPwfcrdj1MvNhsJlsMYQZVZ68=";
      };
      aarch64-darwin = {
        file = "cliare-aarch64-apple-darwin.tar.gz";
        sha256 = "sha256-L5fToCamXIreMmKYmW3K+4Pz8Mw40eYLXJhwj/Mz9e4=";
      };
    };

    systems = builtins.attrNames assets;
    forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f system);

    projectFor = system: let
      pkgs = nixpkgs.legacyPackages.${system};
      asset = assets.${system};
    in pkgs.stdenv.mkDerivation {
      pname = "cliare";
      inherit version;

      src = pkgs.fetchurl {
        url = "https://github.com/modiqo/cliare/releases/download/v${version}/${asset.file}";
        sha256 = asset.sha256;
      };

      sourceRoot = ".";

      nativeBuildInputs = pkgs.lib.optionals pkgs.stdenv.isLinux [ pkgs.autoPatchelfHook ];
      buildInputs = pkgs.lib.optionals pkgs.stdenv.isLinux [ pkgs.stdenv.cc.cc.lib ];

      dontConfigure = true;
      dontBuild = true;

      installPhase = ''
        runHook preInstall
        install -Dm755 cliare "$out/bin/cliare"
        runHook postInstall
      '';

      meta = with pkgs.lib; {
        description = "CLI agent-readiness measurement, command-shape inference, and CI scorecards";
        homepage = "https://github.com/modiqo/cliare";
        downloadPage = "https://github.com/modiqo/cliare/releases";
        license = licenses.asl20;
        mainProgram = "cliare";
        platforms = systems;
        sourceProvenance = [ sourceTypes.binaryNativeCode ];
      };
    };
  in {
    packages = forAllSystems (system: rec {
      cliare = projectFor system;
      default = cliare;
    });

    apps = forAllSystems (system: let
      # WARNING: do NOT replace this `let` binding with `rec` referencing the
      # `packages` attrset above. A `rec { default = { program = "${cliare}/bin/..."; }; }`
      # that names the binding `cliare` shadows the `let`-bound derivation, so
      # `${cliare}` interpolates the app attrset (a set, not a store path) and
      # throws "cannot coerce a set to a string" at `nix run` / `nix flake check`.
      # The separate `let cliarePkg = projectFor system;` binding keeps the
      # derivation in scope as a store path.
      cliarePkg = projectFor system;
    in {
      cliare = {
        type = "app";
        program = "${cliarePkg}/bin/cliare";
      };
      default = {
        type = "app";
        program = "${cliarePkg}/bin/cliare";
      };
    });
  };
}

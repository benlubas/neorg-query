{
  description = "Flake for Development";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/release-24.05";
    flake-utils.url = "github:numtide/flake-utils";

    gen-luarc.url = "github:mrcjkb/nix-gen-luarc-json";
    gen-luarc.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs =
    {
      nixpkgs,
      flake-utils,
      gen-luarc,
      ...
    }:

    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ gen-luarc.overlays.default ];
        };
      in
      {
        devShells.default = pkgs.mkShell {
          name = "Neorg-Query DevShell";

          shellHook =
            let
              luarc = pkgs.mk-luarc-json {
                plugins = with pkgs; [
                  vimPlugins.neorg
                  lua51Packages.pathlib-nvim
                  lua51Packages.nvim-nio
                ];
              };
            in
            # bash
            ''
              ln -fs ${luarc} .luarc.json
            '';

          packages =
            with pkgs;
            [
              lua-language-server
              stylua
              nil
              lua5_1
              rlwrap
              sqlite
            ]
            ++ (pkgs.lib.optionals pkgs.stdenv.isDarwin [ libiconv-darwin darwin.Security ]);
        };
      }
    );
}

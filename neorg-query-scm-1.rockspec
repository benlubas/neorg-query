local MODREV, SPECREV = "scm", "-1"
rockspec_format = "3.0"
package = "neorg-query"
version = MODREV .. SPECREV

description = {
    summary = "A libsql database for your notes",
    labels = { "neovim" },
    homepage = "https://github.com/benluas/neorg-query",
    license = "MIT",
}

source = {
    url = "http://github.com/benlubas/neorg-query/archive/v" .. MODREV .. ".zip",
}

if MODREV == "scm" then
    source = {
        url = "git://github.com/benlubas/neorg-query",
    }
end

dependencies = {
    "neorg ~> 9",
    "lua >= 5.1",
}

build_dependencies = {
  "luarocks-build-rust-mlua",
}

build = {
    type = "rust-mlua",
    modules = {
        ["libneorg_query"] = "neorg_query",
    },
    install = {
        lua = {
            ["neorg_query.api"] = "lua/neorg_query/api.lua",
            ["neorg.modules.external.query.module"] = "lua/neorg/modules/external/query/module.lua",
        },
    },
    copy_directories = $copy_directories,
}

--[[
    file: Query-Module
    title: Query for items in your workspace
    summary: Allows for dynamic queries that can populate lists of TODO items
    internal: false
    ---

Basically poor man's DataView.

--]]

local neorg = require("neorg.core")
local modules = neorg.modules
local log = neorg.log

local module = modules.create("external.query")
local Path = require("pathlib")

---@type libneorg_query.api
local query
---@type core.dirman
local dirman

module.config.public = {
    index_on_launch = true,
}

module.setup = function()
    local ok, res = pcall(require, "libneorg_query.api")
    if ok then
        query = res
    else
        log.error("[Neorg Search] Failed to load `libneorg_query`.\n" .. res)
    end
    return {
        success = ok,
        requires = {
            "core.dirman",
            "core.neorgcmd",
            "core.ui.text_popup",
        },
    }
end

module.load = function()
    log.info("loaded search module")
    module.required["core.neorgcmd"].add_commands_from_table({
        query = {
            min_args = 0,
            max_args = 1,
            name = "query",
            subcommands = {
                run = {
                    args = 0,
                    name = "query.run",
                },
                index = {
                    args = 0,
                    name = "query.index",
                },
            },
        },
    })

    dirman = module.required["core.dirman"] ---@type core.dirman

    if module.config.public.index_on_launch then
        module.private["query.index"]()
    end
end

---@class external.query
module.public = {}

module.events.subscribed = {
    ["core.neorgcmd"] = {
        ["query.run"] = true,
        ["query.index"] = true,
    },
}

module.on_event = function(event)
    if module.private[event.split_type[2]] then
        module.private[event.split_type[2]](event)
    end
end

module.private["query.run"] = function(_)
    require("neorg_query.init").open_telescope_picker("fulltext")
end

module.private["query.index"] = function(_)
    local ws = dirman.get_current_workspace()

    local db_path = Path(vim.fn.stdpath("data")) / "neorg" / "query"
    -- make the directory if it doesn't exist
    db_path:mkdir(Path.permission("rwxr-xr-x"), true)

    query.greet("ben", function (res)
        print(res)
    end)
    -- query.init(tostring(db_path / "db.sqlite"), tostring(ws[2]), function(success)
    --     if success then
    --         print("Yay! we made it through the entire indexing process!")
    --     end
    -- end)
end

return module

--[[
    file: Query-Module
    title: Query for items in your workspace
    summary: Allows for dynamic queries that can populate lists of TODO items
    internal: false
    ---

TOTALLY NON FUNCTIONAL AT THE MOMENT

--]]

local neorg = require("neorg.core")
local modules = neorg.modules
local log = neorg.log

local module = modules.create("external.query")
local Path = require("pathlib")

---@type libneorg_query.api
local query
local dirman

module.config.public = {
    --- Index the workspace on launch
    index_on_launch = true,

    --- Update the db entry when a file is written
    update_on_change = true,
}

module.setup = function()
    local ok, res = pcall(require, "neorg_query.api")
    if ok then
        query = res
    else
        log.error("[Neorg Query] Failed to load `libneorg_query`.\n" .. res)
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
    log.info("loaded query module")
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

    ---@type core.dirman
    dirman = module.required["core.dirman"]

    local ws = dirman.get_current_workspace()
    ---@type PathlibPath
    local ws_path = ws[2]

    local db_path = Path(vim.fn.stdpath("data")) / "neorg" / "query"
    if not db_path:exists() then
        db_path:mkdir(Path.permission("rwxr-xr-x"), true)
    end

    -- initialize the database connection, perform an initial index operation if requested
    query.init(
        tostring(db_path / ("%s.sqlite"):format(ws[1])),
        tostring(ws_path),
        module.config.public.index_on_launch,
        function(success)
            if success then
                vim.notify("[Neorg-Query] Done Indexing!")
            else
                vim.notify("[Neorg-Query] Error on initial workspace index", vim.log.levels.ERROR)
            end
        end
    )

    -- Setup autocommands
    module.private.augroup = vim.api.nvim_create_augroup("neorg-query", { clear = true })
    vim.api.nvim_create_autocmd("BufWrite", {
        pattern = "*.norg",
        group = module.private.augroup,
        callback = function(e)
            if not dirman.in_workspace(Path(e.file)) then return end

            query.index(e.file, function(success)
                if success then
                    log.trace("Indexed file:" .. e.file)
                else
                    log.error("Failed to index file ".. e.file)
                end
            end)
        end
    })
end

---@class external.query
module.public = {
    ---List all the categories, and return them to the function that we require
    ---@param cb fun(res: string[])
    list_categories = function(cb)
        query.all_categories(cb)
    end,
}

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
    query.category_query({ "nvim" }, vim.print)
end

---Index the current workspace
---@param _ neorg.event?
module.private["query.index"] = function(_)
    local ws = dirman.get_current_workspace()

    query.index(
        tostring(ws[2]),
        function(success)
            if success then
                vim.notify("[Neorg-Query] Done Indexing!")
            else
                vim.notify("[Neorg-Query] Error while indexing workspace", vim.log.levels.ERROR)
            end
        end
    )
end

return module

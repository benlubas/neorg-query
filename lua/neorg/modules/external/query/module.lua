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

local formatter
---@type libneorg_query.api
local neorq_rs
---@type core.dirman
local dirman
---@type core.integrations.treesitter
local ts
---@type core.esupports.indent
local indent

-- in the future we'll have "query" too
--- "sql" is the potentially multiline syntax in `|neorq`
--- "sql_inline" is the single line statement that comes after `#sql`
---@alias neorq.type "sql" | "sql_inline"

---@class neorq.tag
---@field start number
---@field end number either the end of the content, or the end of the "setup"
---@field content_start number?
---@field type neorq.type
---@field sql string
---@field format string

module.config.public = {
    --- Index the workspace on launch
    index_on_launch = true,

    --- Update the db entry when a file is written
    update_on_change = true,
}

module.setup = function()
    local ok, res = pcall(require, "neorg_query.api")
    if ok then
        neorq_rs = res
    else
        log.error("[Neorg Query] Failed to load `libneorg_query`.\n" .. res)
        return { success = false }
    end

    return {
        success = ok,
        requires = {
            "core.dirman",
            "core.neorgcmd",
            "core.ui.text_popup",
            "core.integrations.treesitter",
            "core.esupports.indent",
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
                clear = {
                    args = 0,
                    name = "query.clear",
                }
            },
        },
    })

    ---@type core.dirman
    dirman = module.required["core.dirman"]
    ---@type core.integrations.treesitter
    ts = module.required["core.integrations.treesitter"]
    ---@type core.esupports.indent
    indent = module.required["core.esupports.indent"]
    formatter = require("neorg_query.formatter")

    local ws = dirman.get_current_workspace()
    ---@type PathlibPath
    local ws_path = ws[2]

    local db_path = Path(vim.fn.stdpath("data")) / "neorg" / "query"
    if not db_path:exists() then
        db_path:mkdir(Path.permission("rwxr-xr-x"), true)
    end

    -- initialize the database connection, perform an initial index operation if requested
    neorq_rs.init(
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
            if not dirman.in_workspace(Path(e.file)) then
                return
            end

            neorq_rs.index(e.file, function(success)
                if success then
                    log.trace("Indexed file:" .. e.file)
                else
                    log.error("Failed to index file " .. e.file)
                end
            end)
        end,
    })
end

---@class external.query
module.public = {
    --- provide a list of all the categories in the workspace to a callback function
    ---@param cb fun(res: string[])
    list_categories = function(cb)
        neorq_rs.all_categories(cb)
    end,
}

module.events.subscribed = {
    ["core.neorgcmd"] = {
        ["query.run"] = true,
        ["query.index"] = true,
        ["query.clear"] = true,
    },
}

module.on_event = function(event)
    if module.private[event.split_type[2]] then
        module.private[event.split_type[2]](event)
    end
end

--- find `#sql` tags, and the range around them
--- NOTE: we have to use regex to search b/c the TS parser for norg is so broken there's no
--- reasonable way to use TS
---@param buf number
---@return neorq.tag[]
module.private.find_sql_tags = function(buf)
    local query_str = [[(strong_carryover name: ((tag_name) @name (#eq? @name "sql")))]]
    local lines = vim.api.nvim_buf_get_lines(buf, 0, -1, false)

    local neorq_group = {}
    ts.execute_query(query_str, function(_query, _id, node, _meta)
        local range = ts.get_node_range(node)
        local lnr = range.row_start + 1

        local tags = {}
        while lnr > 0 do
            local name, args = lines[lnr]:match("^%s*#(%w+)%s?(.*)")
            if not name then
                tags["start"] = lnr + 1
                break
            end
            tags[name] = args
            lnr = lnr - 1
        end

        lnr = range.row_start + 1
        while lnr <= #lines do
            local name, args = lines[lnr]:match("^%s*#(%w+)%s?(.*)")
            if not name then
                break
            end
            tags[name] = args
            lnr = lnr + 1
        end

        if tags["sql"] == "" then
            local verbatim = ts.get_first_node_on_line(buf, lnr - 1, "^verbatim$")
            if not verbatim then
                return
            end
            local verbatim_range = ts.get_node_range(verbatim)
            local content = ts.get_node_text(verbatim, buf)

            tags["sql"] = content:match("`|(.*)|`") or ""
            lnr = verbatim_range.row_end + 2
        else
            tags["sql"] = tags["sql"]:match("`|(.*)|`")
        end

        if tags["sql"] == "" then
            return
        end

        if lnr <= #lines then
            if lines[lnr]:match("^%s*___$") then
                tags["content_start"] = lnr
                lnr = lnr + 1
                while lnr <= #lines do
                    if lines[lnr]:match("^%s*___$") then
                        break
                    end
                    lnr = lnr + 1
                end
                tags["end"] = lnr
            else
                tags["end"] = lnr - 1
            end
        else
            tags["end"] = #lines
        end

        tags["format"] = tags["format"]:match("`|(.*)|`")
        table.insert(neorq_group, tags)
    end, buf)

    return neorq_group
end

---Run the query, and return the formatted result via callback
---@param query string
---@param format string
---@param buf number
---@param lnr number
---@param cb fun(res: string[])
module.private.sql_query = function(query, format, buf, lnr, cb)
    local line_indent = (" "):rep(indent.indentexpr(buf, lnr) or 0)
    local ws = dirman.get_current_workspace()[2]
    neorq_rs.user_query(query, {}, function(res)
        local lines = {}
        for _, row in ipairs(res) do
            local line = line_indent
                .. format:gsub("${(.-)}", function(name)
                    return formatter.format_col(ws, name, row)
                end)
            table.insert(lines, line)
        end
        cb(lines)
    end)
end

---find the sql tag that the cursor is in, if it's in one
---@param lnr number
---@param buf number
---@return neorq.tag?
local function current_tag(lnr, buf)
    local tags = module.private.find_sql_tags(buf)

    for _, t in ipairs(tags) do
        if lnr >= t["start"] and lnr <= t["end"] then
            return t
        end
    end
end

---@param event neorg.event
module.private["query.run"] = function(event)
    local tag = current_tag(event.cursor_position[1], event.buffer)
    if not tag then
        log.error("Not in a neorq block")
        return
    end

    module.private.sql_query(tag.sql, tag.format, event.buffer, tag.start, function(res)
        vim.schedule(function()
            if tag.content_start then
                vim.api.nvim_buf_set_lines(event.buffer, tag.content_start, tag["end"] - 1, false, res)
            else
                local leading_whitespace = (" "):rep(indent.indentexpr(event.buffer, tag.start) or 0)
                table.insert(res, 1, leading_whitespace .. "___")
                table.insert(res, leading_whitespace .. "___")
                vim.api.nvim_buf_set_lines(event.buffer, tag["end"], tag["end"], false, res)
            end
        end)
    end)
end

---Index the current workspace
---@param _ neorg.event?
module.private["query.index"] = function(_)
    local ws = dirman.get_current_workspace()

    neorq_rs.index(tostring(ws[2]), function(success)
        if success then
            vim.notify("[Neorg-Query] Done Indexing!")
        else
            vim.notify("[Neorg-Query] Error while indexing workspace", vim.log.levels.ERROR)
        end
    end)
end

---@param event neorg.event
module.private["query.clear"] = function(event)
    local row = event.cursor_position[1]
    local tag = current_tag(row, event.buffer)
    if not tag then
        return
    end

    if tag.content_start and tag["end"] then
        vim.api.nvim_buf_set_lines(event.buffer, tag.content_start - 1, tag["end"], false, {})
        if row > tag.content_start then
            vim.api.nvim_win_set_cursor(event.window, { tag.content_start - 1, event.cursor_position[2] })
        end
    end
end

return module

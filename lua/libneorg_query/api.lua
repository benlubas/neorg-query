--[[
    imports and wraps async functions from rust so they can be called here without issue
--]]

local query = require("libneorg_query")

---@class libneorg_query.api
local M = {}

local PENDING = (coroutine.wrap(query.PENDING))()

---Wrap an async rust function in a coroutine that neovim will poll. Return a function that takes
---function args and a callback function
---@param async_fn any
---@return fun(args: ...)
local wrap = function(async_fn)
    return function(...)
        local args = { ... }
        local cb = args[#args]
        args[#args] = nil

        local thread = coroutine.wrap(async_fn)
        local exec
        exec = function()
            local res = thread(unpack(args))
            if res == PENDING then
                vim.defer_fn(exec, 10)
            else
                cb(res)
            end
        end
        vim.schedule(exec)
    end
end

---@type fun(database_path: string, workspace_path: string, do_index: boolean, callback: fun(success: boolean))
M.init = wrap(query.init)

---@type fun(path: string, callback: fun(success: boolean))
M.index = wrap(query.index)

---@class CategoryQueryResponse
---@field path string
---@field title string | nil
---@field description string | nil
---@field created string | nil
---@field updated string | nil

---Query for all documents that have all the categories listed
---@type fun(categories: string[], callback: fun(res: CategoryQueryResponse[]))
M.category_query = wrap(query.category_query)

---Return a list of all the categories
---@type fun(callback: fun(res: string[]))
M.all_categories = wrap(query.all_categories)

return M

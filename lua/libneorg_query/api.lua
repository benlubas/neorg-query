--[[
    imports and wraps async functions from rust so they can be called here without issue
--]]

local query = require("libneorg_query")

---@class libneorg_query.api
local M = {}

M.PENDING = (coroutine.wrap(query.PENDING))()

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
            if res == M.PENDING then
                vim.schedule(exec)
            else
                cb(res)
            end
        end
        vim.schedule(exec)
    end
end

---@type fun(database_path: string, workspace_path: string, callback: fun(success: boolean))
M.init = wrap(query.init)

---@class CategoryQueryResponse
---@field path string
---@field title string | nil
---@field description string | nil
---@field created string | nil
---@field updated string | nil

---@type fun(categories: string[], callback: fun(res: CategoryQueryResponse[]))
M.query_category = wrap(query.query_category)

---@type fun(name: string, callback: fun(res: string))
M.greet = wrap(query.greet)

M.test_callbacks = wrap(query.greet)

return M

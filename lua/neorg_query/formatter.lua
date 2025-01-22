local M = {}

local Path = require("pathlib")

---Format the variable according to any modifiers or fallback rules
---@param ws PathlibPath
---@param expr string
---@param row table<string, string>
---@return string
M.format_col = function(ws, expr, row)
    for _, n in ipairs(vim.split(expr, "|")) do
        local col, mod_string = n:match("^(.-):(.*)$")
        if not col then
            col = n
        end
        if row[col] then
            if mod_string then
                return M.apply_mods(ws, mod_string, col, row[col])
            else
                return row[n]
            end
        end
    end

    return row[expr] or ""
end

---Apply modifiers
---@param ws PathlibPath workspace path
---@param mod_string string group of modifiers like ":$:t"
---@param name string column name
---@param value string column value
---@return string
M.apply_mods = function(ws, mod_string, name, value)
    local mods = vim.split(mod_string, ":")
    for _, mod in ipairs(mods) do
        if name == "path" then
            if mod == "$" then
                value = "$/" .. Path(value):relative_to(ws):with_suffix("")
            elseif mod == "t" then
                value = tostring(Path(value):stem())
            end
        end
    end
    return value
end

---Return a string representation of the task's detached modifier extensions
---@param task table
M.task_extensions = function(task)
    local strs = {}
    if task.status then
        table.insert(strs, ({
            ["Undone"] = " ",
            ["Done"] = "x",
            ["NeedsClarification"] = "?",
            ["Pending"] = "-",
            ["Canceled"] = "_",
            ["Urgent"] = "!",
            ["Paused"] = "=",
            -- NOTE: recurring is a special case that we're not supporting right now
        })[task.status])
    end
    if task.priority then
        table.insert(strs, "# " .. task.priority)
    end
    local function time(val)
        if type(val) == "number" then
            local d = os.date("!*t", val)
            local date = require("neorg.modules.core.tempus.module").public.to_date(d)
            return tostring(date)
        end
        return val -- if we get a string, we're not going to deal with it
    end
    if task.starts then
        table.insert(strs, "> " .. time(task.starts))
    end
    if task.due then
        table.insert(strs, "< " .. time(task.due))
    end
    if task.timestamp then
        table.insert(strs, "@ " .. time(task.timestamp))
    end

    return "("..table.concat(strs, "|")..")"
end

return M

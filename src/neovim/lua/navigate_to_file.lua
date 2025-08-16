-- Function to navigate to file with DocumentIdentifier support
-- Supports buffer_id, project_relative_path, and absolute_path
-- If file is already open in a buffer, switch to it; otherwise use :edit
-- Then jump to specified line and center the view

local document_identifier_json, line_number = unpack({ ... })
local document_identifier = vim.json.decode(document_identifier_json)
line_number = line_number or 1

local function navigate_to_file()
    local filepath = nil
    local target_buffer = nil

    -- Handle different DocumentIdentifier types
    if document_identifier.buffer_id then
        -- BufferId case - switch directly to buffer
        target_buffer = document_identifier.buffer_id
        if not vim.api.nvim_buf_is_valid(target_buffer) then
            return vim.json.encode({
                err_msg = string.format("Buffer ID %d is not valid", target_buffer),
            })
        end
        filepath = vim.api.nvim_buf_get_name(target_buffer)
    elseif document_identifier.project_relative_path then
        -- ProjectRelativePath case - resolve to absolute path
        local cwd = vim.fn.getcwd()
        filepath = vim.fn.resolve(cwd .. "/" .. document_identifier.project_relative_path)
    elseif document_identifier.absolute_path then
        -- AbsolutePath case - use directly
        filepath = document_identifier.absolute_path
    else
        return vim.json.encode({
            err_msg = "Invalid DocumentIdentifier: must have buffer_id, project_relative_path, or absolute_path",
        })
    end

    -- If we don't have a target buffer yet, check if file is already open
    if not target_buffer and filepath then
        local buffers = vim.api.nvim_list_bufs()
        for _, buf in ipairs(buffers) do
            if vim.api.nvim_buf_is_loaded(buf) then
                local buf_name = vim.api.nvim_buf_get_name(buf)
                if buf_name == filepath then
                    target_buffer = buf
                    break
                end
            end
        end
    end

    -- Switch to existing buffer or edit new file
    if target_buffer then
        vim.api.nvim_set_current_buf(target_buffer)
    elseif filepath then
        vim.cmd("edit " .. vim.fn.fnameescape(filepath))
    else
        return vim.json.encode({
            err_msg = "Could not determine file path",
        })
    end

    -- Jump to line and center
    vim.api.nvim_win_set_cursor(0, { line_number, 0 })
    vim.cmd("normal! zz")

    -- Get the actual filepath for response
    local actual_filepath = vim.api.nvim_buf_get_name(0)

    return vim.json.encode({
        result = string.format("Navigated to %s at line %d", actual_filepath, line_number),
    })
end

return navigate_to_file()

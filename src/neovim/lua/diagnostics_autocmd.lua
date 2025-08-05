local group = vim.api.nvim_create_augroup("NVIM_MCP_DiagnosticsChanged", { clear = true })
vim.api.nvim_create_autocmd("DiagnosticChanged", {
	group = group,
	callback = function(args)
		vim.rpcnotify(0, "NVIM_MCP_DiagnosticsChanged", {
			buf = args.buf,
			diagnostics = args.data.diagnostics,
		})
	end,
})
vim.api.nvim_create_autocmd("LspAttach", {
	group = group,
	callback = function(args)
		vim.rpcnotify(0, "NVIM_MCP_LspAttach", args.data.diagnostics)
	end,
})
vim.rpcnotify(0, "NVIM_MCP", "setup diagnostics changed autocmd")

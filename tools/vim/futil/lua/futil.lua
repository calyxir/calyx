local autocmd = vim.api.nvim_create_autocmd
autocmd("FileType", {
    pattern = "futil",
    callback = function()
        local root_dir = vim.fs.dirname(
            vim.fs.find({ '.git' }, { upward = true })[1]
        )
        local client = vim.lsp.start({
            name = 'calyx-lsp',
            cmd = { 'calyx-lsp' },
            root_dir = root_dir,
        })
        vim.lsp.buf_attach_client(0, client)
    end
})

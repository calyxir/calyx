local autocmd = vim.api.nvim_create_autocmd

local M = {}

function M.setup(settings)
   -- setup a function that runs when a .futil file is opened
   autocmd("FileType", {
              pattern = "futil",
              callback = function()
                 local root_dir = vim.fs.dirname(
                    vim.fs.find({ '.git' }, { upward = true })[1]
                 )

                 -- set default settings
                 if settings == nil then
                    settings = {
                       calyxLsp = {
                          libraryPaths = {
                             "~/.calyx"
                          }
                       }
                    }
                 end

                 -- start the lsp client
                 local client = vim.lsp.start({
                       name = 'calyx-lsp',
                       cmd = { 'calyx-lsp' },
                       root_dir = root_dir,
                       settings = settings
                 })

                 -- attach the lsp client to the current buffer
                 vim.lsp.buf_attach_client(0, client)
              end
   })
end

return M



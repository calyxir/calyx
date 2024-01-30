# Calyx README

Calyx VSCode Extension. I've separated this from the previous one for now, just because it was easier to generate from scratch from the `yo` tool and I don't know how these things work.

I'm experimenting integrating the Calyx language server with this extension. At the moment, jump to definition works.

## Trying this out

You need to clone and build the language server: https://github.com/calyxir/calyx-lsp

And then you need to edit the language server path in `src/extension.ts` and then build the extension with `npm run compile`. You can test out the extension with `code --extensionDevelopmentPath=$PWD` from the extension directory. Reloading it, picks up future changes.

# Themes

Themes from [Zed Editor](https://zed.dev/) can be imported into Ki, examples of imported themes[^1]:

1. Gruvbox
2. Ayu
3. One

## Syntax highlighting

Syntax highlighting is powered by Tree-sitter, and for that to work it needs:

1. Tree-sitter grammar (for generating the parser)
2. Highlight queries (for determining code sections to highlight)

Tree-sitter grammars is not usually the problem, because there are many open-source tree-sitter grammars out there.

However, the highlight queries are the problem, the grammar author usually provides only barebone highlight queries for their language, and the maintenance of highlight queries is delegated to editor-specific community.

I think this is a tragedy because these highlight queries should not be editor-specific, why should every editor maintain their highlight queries? These wheels should not be reinvented over and over.

Currently, the largest of such communities are Neovim and Helix.

To avoid further fragmentation, Ki currently downloads highlight queries from [nvim-treesitter](https://github.com/nvim-treesitter/nvim-treesitter)[^2], until there's a standardized editor-agnostic highlight queries repository.

[^1]: See more at [Zed default themes](https://github.com/zed-industries/zed/tree/main/assets/themes)
[^2]: Why not from Helix? Because Helix [precedence ordering](https://github.com/helix-editor/helix/issues/9436) is not compatible with the [tree-sitter-highlight](https://github.com/tree-sitter/tree-sitter/tree/master/highlight) library yet.

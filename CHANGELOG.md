# Changelog

All notable changes to this project will be documented in this file.

## [0.1.1] - 2026-03-22

### Added
- feat(completion): add word-based completion fallback
- feat(editor): execute im-select on exit from insert mode
- feat(insert): Delete MoL
- feat(theme): save selected theme to ~/.config/ki/.current_theme

### Fixed
- fix(lsp): response missing field "jsonrpc"
- fix(editor/save_all): dirty indicators should be cleared
- fix(list/grep): replace not working if dirty bit was false
- fix(selection_mode/LineTrimmed): issues around the last line

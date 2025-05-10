# VSCode-Neovim Architecture Analysis

This document analyzes the architecture of the VSCode-Neovim extension to understand how it integrates Neovim with
VSCode. The goal is to learn from its design and apply relevant patterns to our Ki-VSCode integration.

## 1. IPC Messages

VSCode-Neovim uses Neovim's msgpack-RPC protocol for communication. Here are the key message types:

### 1.1 Neovim to VSCode Messages

| Message Type       | Purpose                               | Source Location                                        |
| ------------------ | ------------------------------------- | ------------------------------------------------------ |
| `redraw`           | UI updates (cursor, grid, mode, etc.) | Handled in `eventBus.on("redraw")` in various managers |
| `mode_change`      | Mode changes (normal, insert, visual) | Handled in `CursorManager.handleRedraw`                |
| `win_viewport`     | Viewport updates                      | Handled in `ViewportManager.handleRedraw`              |
| `grid_cursor_goto` | Cursor position updates               | Handled in `CursorManager.handleRedraw`                |
| `grid_line`        | Buffer content updates                | Handled in `DocumentChangeManager`                     |
| `cmdline_show`     | Command line display                  | Handled in `CommandLineManager`                        |
| `cmdline_hide`     | Command line hide                     | Handled in `CommandLineManager`                        |
| `vscode-action`    | Execute VSCode commands               | Handled in `MainController.onNeovimNotification`       |
| `vscode-neovim`    | Custom extension events               | Handled in `MainController.onNeovimNotification`       |

### 1.2 VSCode to Neovim Messages

| Message Type          | Purpose                | Source Location                                      |
| --------------------- | ---------------------- | ---------------------------------------------------- |
| `nvim_win_set_cursor` | Update cursor position | Called in `CursorManager.updateNeovimCursorPosition` |
| `nvim_buf_set_lines`  | Update buffer content  | Called in `DocumentChangeManager`                    |
| `nvim_buf_set_name`   | Set buffer name        | Called in `BufferManager.initBufferForDocument`      |
| `nvim_buf_set_option` | Set buffer options     | Called in `BufferManager.initBufferForDocument`      |
| `nvim_buf_set_var`    | Set buffer variables   | Called in `BufferManager.initBufferForDocument`      |
| `nvim_create_buf`     | Create a new buffer    | Called in `BufferManager.syncVisibleEditors`         |
| `nvim_open_win`       | Create a new window    | Called in `BufferManager.createNeovimWindow`         |
| `nvim_win_close`      | Close a window         | Called in `BufferManager.cleanupWindowsAndBuffers`   |
| `nvim_command`        | Execute Vim commands   | Called in various places                             |
| `nvim_input`          | Send keystrokes        | Called in `TypingManager.sendKeys`                   |

### 1.3 Custom Events

| Event Type         | Purpose                 | Source Location                                 |
| ------------------ | ----------------------- | ----------------------------------------------- |
| `mode-changed`     | Notify mode changes     | Emitted in `ModeManager`                        |
| `window-scroll`    | Notify window scrolling | Emitted in `ViewportManager`                    |
| `open-file`        | Request to open a file  | Handled in `BufferManager.handleOpenFile`       |
| `external-buffer`  | Handle external buffers | Handled in `BufferManager.handleExternalBuffer` |
| `notify-recording` | Macro recording status  | Handled in `ModeManager`                        |

## 2. VSCode Events Handled

VSCode-Neovim listens to many VSCode events to keep the editors in sync:

| Event                                       | Purpose                     | Handler                                      |
| ------------------------------------------- | --------------------------- | -------------------------------------------- |
| `window.onDidChangeVisibleTextEditors`      | Track visible editors       | `BufferManager.onEditorLayoutChanged`        |
| `window.onDidChangeActiveTextEditor`        | Track active editor         | `BufferManager.onEditorLayoutChanged`        |
| `window.onDidChangeTextEditorOptions`       | Track editor options        | `BufferManager.onDidChangeEditorOptions`     |
| `window.onDidChangeTextEditorSelection`     | Track selection changes     | `CursorManager.onSelectionChanged`           |
| `window.onDidChangeTextEditorVisibleRanges` | Track viewport changes      | `ViewportManager.onDidChangeVisibleRange`    |
| `workspace.onDidChangeTextDocument`         | Track document changes      | `DocumentChangeManager.onChangeTextDocument` |
| `workspace.onDidCloseTextDocument`          | Track document closing      | `BufferManager.onEditorLayoutChanged`        |
| `workspace.onDidChangeConfiguration`        | Track configuration changes | Various managers                             |

## 3. Viewport Management

VSCode-Neovim handles viewport synchronization through the `ViewportManager` class:

### 3.1 Key Components

- `gridViewport`: Maps grid IDs to viewport data (topline, botline, cursor position)
- `getViewport`: Retrieves viewport data for a grid
- `getCursorFromViewport`: Gets cursor position from viewport data
- `scrollNeovim`: Sends scroll commands to Neovim

### 3.2 Synchronization Flow

1. VSCode viewport changes → `onDidChangeVisibleRange` → debounced `scrollNeovim`
2. Neovim viewport changes → `win_viewport` event → update `gridViewport`
3. Cursor position retrieval uses viewport data for accuracy

### 3.3 Challenges Addressed

- Debouncing scroll events to reduce jitter
- Handling smooth scrolling settings
- Coordinating with cursor updates

## 4. Multiple Cursor Support

VSCode-Neovim handles multiple cursors through the `CursorManager` class:

### 4.1 Key Components

- `neovimCursorPosition`: Maps editors to their cursor positions in Neovim
- `updateCursorPosInEditor`: Updates VSCode cursor based on Neovim events
- `updateNeovimCursorPosition`: Updates Neovim cursor based on VSCode events

### 4.2 Synchronization Flow

1. VSCode selection changes → `onSelectionChanged` → `updateNeovimCursorPosition`
2. Neovim cursor changes → `grid_cursor_goto` event → `updateCursorPosInEditor`
3. Visual mode selections are created from Neovim visual mode data

### 4.3 Challenges Addressed

- Converting between VSCode and Neovim coordinate systems
- Handling different selection types (char, line, block)
- Avoiding feedback loops in cursor updates

## 5. Buffer Synchronization

VSCode-Neovim handles buffer synchronization through the `BufferManager` and `DocumentChangeManager` classes:

### 5.1 Key Components

- `textDocumentToBufferId`: Maps VSCode documents to Neovim buffer IDs
- `textEditorToWinId`: Maps VSCode editors to Neovim window IDs
- `bufferSkipTicks`: Tracks change ticks to avoid feedback loops
- `documentChangeLock`: Mutex for preventing concurrent modifications

### 5.2 Synchronization Flow

#### VSCode to Neovim:

1. Document changes → `onChangeTextDocument` → calculate diff
2. Convert diff to Neovim coordinates
3. Update Neovim buffer via `nvim_buf_set_lines`
4. Increment `bufferSkipTicks` to prevent feedback

#### Neovim to VSCode:

1. Buffer changes → `onBufferEvent` → queue changes
2. Check `bufferSkipTicks` to avoid feedback
3. Apply changes to VSCode document
4. Update `documentContentInNeovim` to track state

### 5.3 Challenges Addressed

- Avoiding feedback loops in bidirectional sync
- Handling coordinate system differences
- Batching changes for better performance
- Coordinating with cursor updates

## 6. Architecture Patterns

### 6.1 Manager Pattern

Each aspect of the integration is handled by a dedicated manager:

- `BufferManager`: Manages buffer and window mapping
- `CursorManager`: Manages cursor and selection sync
- `ViewportManager`: Manages viewport sync
- `ModeManager`: Manages mode sync
- `DocumentChangeManager`: Manages document content sync
- `TypingManager`: Manages keyboard input

### 6.2 Event Bus

The `eventBus` provides a central event system:

- Decouples event producers from consumers
- Simplifies event handling and debugging
- Allows for easy event subscription and unsubscription

### 6.3 Synchronization Locks

Various locks and promises ensure operations happen in the correct order:

- `documentChangeLock`: Prevents concurrent document modifications
- `textDocumentChangePromise`: Signals completion of document changes
- `syncEditorLayoutPromise`: Signals completion of layout changes
- `cursorUpdatePromise`: Signals completion of cursor updates

### 6.4 Bidirectional Mapping

The extension maintains bidirectional mappings between VSCode and Neovim:

- VSCode document ↔ Neovim buffer
- VSCode editor ↔ Neovim window
- VSCode selection ↔ Neovim cursor/visual selection
- VSCode viewport ↔ Neovim viewport

## 7. Lessons for Ki-VSCode Integration

### 7.1 Message Types to Consider

Based on VSCode-Neovim's approach, our Ki-VSCode integration should include these message types:

1. **Buffer Events**:

    - `buffer.diff`: Buffer content changes
    - `buffer.open`: Buffer opened
    - `buffer.close`: Buffer closed

2. **Cursor and Selection Events**:

    - `cursor.update`: Cursor position updates
    - `selection.update`: Selection changes

3. **Mode Events**:

    - `mode.change`: Mode changes (normal, insert, visual)

4. **Viewport Events**:

    - `viewport.update`: Viewport changes

5. **Command Events**:
    - `command.execute`: Execute commands
    - `editor.action`: Perform editor actions

### 7.2 Architecture Recommendations

1. **Clear Separation of Concerns**:

    - Use a manager-based architecture similar to VSCode-Neovim
    - Each manager should handle a specific aspect of the integration
    - Use an event bus or channel for internal communication

2. **Robust Synchronization**:

    - Use locks and promises to coordinate operations
    - Track versions and change ticks to avoid feedback loops
    - Batch operations for better performance

3. **Bidirectional Communication**:

    - Define clear protocols for communication in both directions
    - Handle coordinate system differences
    - Map concepts between the two editors

4. **Performance Considerations**:
    - Debounce high-frequency events
    - Batch operations when possible
    - Use optimistic updates for better responsiveness

## 8. Conclusion

The VSCode-Neovim extension provides valuable insights for our Ki-VSCode integration. Its manager-based architecture,
event system, and synchronization mechanisms offer a proven approach to handling the complexities of editor integration.
By adapting these patterns to our specific needs, we can create a more robust and maintainable integration between Ki
and VSCode.

# Ki VSCode Integration Plan

## Overview

Write a VSCode plugin that allows Ki-editor to drive the VSCode editor. The VSCode editor should behave as if it were
Ki-editor (modal, multi-cursor, AST-aware editing, advanced movement and selection modes, selection mode -> movement ->
action workflow).

### Major Components

1. Websocket IPC for communication between VSCode and Ki
2. Protocol defined in `ki-protocol-types/src/lib.rs`
3. VSCodeApp struct which wraps the Ki editor App, manages IPC, and hooks into Ki's event loop

## Current Architecture

Currently, the integration is tightly coupled with Ki's core:

- The majority of the integration is handled by `handle_dispatch_editor_custom` in `app.rs`
- VSCode-specific code is scattered throughout Ki's codebase using `#[cfg(feature = "vscode")]`
- Ki directly sends notifications to VSCode via a notification channel (`vscode_notification_sender` and
  `from_app_receiver`)
- VSCode-specific dispatch variants exist in Ki's core Dispatch enum (like `Dispatch::BufferEditTransaction`)
- The `VSCodeApp` has multiple communication channels with different responsibilities:
    - `app_sender`/`app_message_receiver` for sending messages to the App
    - `from_app_receiver` for receiving notifications from the App (to be replaced)
    - `ipc_handler` for communication with VSCode
- Communication with Ki is done through the `AppMessage::ExternalDispatch` variant, which allows sending any `Dispatch`
  to Ki without directly locking the App mutex (avoiding deadlocks)

This architecture has several drawbacks:

- Tight coupling between Ki and VSCode
- Scattered VSCode-specific code makes maintenance difficult
- Difficult to add support for other integrations
- Complex message flow is hard to trace and debug
- Multiple overlapping communication channels with unclear responsibilities

## Lessons from VSCode-Neovim

After analyzing the VSCode-Neovim extension, we've identified several architectural patterns and approaches that could
benefit our integration:

### 1. Manager-Based Architecture

VSCode-Neovim uses a manager pattern where each manager is responsible for a specific aspect of the integration:

- `BufferManager`: Manages buffer and window mapping
- `CursorManager`: Manages cursor and selection sync
- `ViewportManager`: Manages viewport sync
- `ModeManager`: Manages mode sync
- `DocumentChangeManager`: Manages document content sync

This clear separation of concerns makes the codebase more maintainable and easier to extend.

### 2. Robust Synchronization Mechanisms

VSCode-Neovim uses several techniques to ensure robust synchronization:

- Mutex locks to prevent concurrent modifications
- Completion promises to coordinate operations
- Version tracking to avoid feedback loops
- Debouncing for high-frequency events

### 3. External Buffer Handling

VSCode-Neovim handles "external buffers" (buffers created by Neovim that don't correspond to real files) through:

- A `TextDocumentContentProvider` implementation
- Special tracking of external documents
- Custom URI scheme for external buffers
- Buffer event listeners for content updates

This approach could be adapted for Ki's temporary buffers, such as help buffers, command output, and other non-file
buffers.

### 4. Comprehensive Event Types

VSCode-Neovim handles a wide range of events:

- Buffer events (content changes, open/close)
- Cursor and selection events
- Mode changes
- Viewport changes
- Command execution

Our integration should include similar event types to ensure complete functionality.

## Proposed Architecture: Integration Event Channel

Based on our analysis and lessons from VSCode-Neovim, we propose a cleaner architecture that decouples Ki from VSCode:

### 1. Integration Event Channel

Replace the current VSCode-specific notification channel (`vscode_notification_sender` and `from_app_receiver`) with a
generic integration channel that emits events relevant to external integrations:

```rust
// In App struct
integration_event_sender: Option<Sender<IntegrationEvent>>,

// New enum for integration events
enum IntegrationEvent {
    // Buffer events
    BufferChanged {
        component_id: ComponentId,
        path: CanonicalizedPath,
        transaction: EditTransaction
    },
    BufferOpened {
        component_id: ComponentId,
        path: CanonicalizedPath,
        language_id: Option<String>
    },
    BufferClosed {
        component_id: ComponentId,
        path: CanonicalizedPath
    },
    BufferSaved {
        component_id: ComponentId,
        path: CanonicalizedPath
    },
    BufferActivated {
        component_id: ComponentId,
        path: CanonicalizedPath
    },

    // Editor state events
    ModeChanged {
        component_id: ComponentId,
        mode: Mode,
        selection_mode: SelectionMode
    },
    SelectionChanged {
        component_id: ComponentId,
        selections: Vec<Selection>
    },
    CursorUpdate {
        component_id: ComponentId,
        anchors: Vec<Position>,
        actives: Vec<Position>
    },
    ViewportChanged {
        component_id: ComponentId,
        start_line: usize,
        end_line: usize
    },

    // External buffer events
    ExternalBufferCreated {
        component_id: ComponentId,
        buffer_id: String,
        content: String
    },
    ExternalBufferUpdated {
        component_id: ComponentId,
        buffer_id: String,
        changes: Vec<TextChange>
    },

    // Other events
    CommandExecuted {
        command: String,
        success: bool
    }
}
```

### 2. VSCodeApp as Event Translator

VSCodeApp would:

- Receive events from the integration channel (replacing `from_app_receiver`)
- Translate them to VSCode-specific messages
- Send them to VSCode via the IPC channel
- Continue to use `app_sender`/`app_message_receiver` for sending messages to the App

### 3. Manager-Based TypeScript Architecture

Inspired by VSCode-Neovim, we'll refactor the TypeScript side to use a manager-based architecture:

- `BufferManager`: Handles buffer synchronization and mapping
- `CursorManager`: Handles cursor and selection synchronization
- `ModeManager`: Handles mode changes and keyboard input
- `ViewportManager`: Handles viewport synchronization
- `CommandManager`: Handles command execution

### 4. Minimal Ki Core Modifications

- Make `BufferEditTransaction` a generic dispatch type (not VSCode-specific)
- Add the integration event channel to send events out of Ki
- Remove VSCode-specific code from Ki's core (including `#[cfg(feature = "vscode")]` blocks)
- Remove the VSCode-specific notification channel

### Benefits

- **Clean Separation**: Ki doesn't need to know about VSCode or any other integration
- **Minimal Changes**: We don't need to modify Ki's API signatures
- **Extensibility**: The same channel could be used for other integrations in the future
- **Clarity**: The flow of information is clear and easy to follow
- **Robustness**: Better synchronization mechanisms based on proven patterns

## Implementation Plan

Based on our analysis and the lessons from VSCode-Neovim, we've developed a phased implementation plan:

### Phase 1: Rust-Side Refactoring

1. **Define IntegrationEvent Enum**: COMPLETE

    - Create a comprehensive enum that covers all necessary event types
    - Include fields for all required information
    - Ensure the enum is well-documented

2. **Add Integration Channel to Ki**: COMPLETE

    - Add an optional sender to App that sends IntegrationEvents
    - Identify key points in Ki's code to emit events
    - Implement event emission at these points

3. **Update Protocol Types**: COMPLETE

    - Enhance protocol types to align with Ki's event types
    - Ensure protocol types are comprehensive and well-documented
    - Generate TypeScript types from protocol definitions

4. **Modify VSCodeApp**: COMPLETE

    - Add a receiver for IntegrationEvents
    - Implement translation from IntegrationEvents to protocol messages
    - Update IPC handling to use the new protocol types
    - Update handlers to handle new events to match functionality of current implementation. Leave stubs for any that we
      do not yet implement.

5. **Cleanup**: COMPLETE
    - Remove vscode specific notification channel from app.rs
    - Remove vscode specific code from Ki's core. We need to keep some of our modifications such as returning an
      EditTransaction from Buffer::apply_edit_transaction and the BufferEditTransaction dispatch. Most or all of what
      needs to be removed is in app.rs
    - Final check for any remaining vscode specific code that is not strictly necessary

### Phase 2: TypeScript-Side Refactoring

1. **Implement Manager-Based Architecture**:

    - Create manager classes for different aspects of the integration
    - Define clear responsibilities for each manager
    - Implement communication between managers

2. **Enhance Buffer Synchronization**:

    - Implement robust buffer diff application
    - Add version tracking to avoid feedback loops
    - Handle external buffers using a TextDocumentContentProvider

3. **Improve Cursor and Selection Handling**:

    - Implement better cursor position tracking
    - Handle multiple selections
    - Coordinate cursor updates with buffer changes

4. **Implement Viewport Synchronization**:

    - Track visible ranges
    - Implement scrolling synchronization
    - Debounce high-frequency events

5. **Cleanup**:
    - Remove files no longer in use
    - Remove imports no longer in use
    - Refactor as necessary to keep the code clean

### Phase 3: Feature Completion

1. **External Buffer Support**:

    - Implement support for Ki's temporary buffers
    - Create a custom URI scheme for external buffers
    - Handle content updates for external buffers

2. **Mode and Input Handling**:

    - Improve mode synchronization
    - Enhance keyboard input handling
    - Support for all Ki modes

3. **Command Execution**:

    - Implement command execution flow
    - Handle command results
    - Support for all Ki commands

4. **Performance Optimizations**:
    - Batch operations where possible
    - Optimize high-frequency events
    - Reduce unnecessary communication

## TODO

### Current Event Flow

Let's trace through the current flow of events when we press undo:

1. Send editor action to backend
2. Backend creates an `AppMessage::ExternalDispatch(Dispatch::ToEditor(DispatchEditor::Undo))` and sends it to Ki via
   `app_sender`
3. Ki processes the message in its event loop via `process_message`, which calls `handle_dispatch` internally
4. This eventually flows to `handle_dispatch_editor_custom` where it calls `handle_dispatch_editor` on the Component
5. This in turn calls `handle_dispatch_editor` on the Editor
6. Matches to Undo eventually calling `undo_or_redo`
7. `undo_or_redo` calls `undo` on Buffer, resulting in a `selection_set` and `applied_transaction`
8. Here in VSCode we create a `BufferEditTransaction` and return it with the dispatches
9. Now we're all the way back in `handle_dispatch_editor_custom` on App where we have new dispatches
10. We call `self.handle_dispatches` which likely contains `DocumentDidChange`, `SelectionUpdate`,
    `BufferEditTransaction`
11. `Dispatch::BufferEditTransaction` matches, we call `send_vscode_notification` with a `BufferDiff`
12. Rest of VSCode-specific logic in `handle_dispatch_editor_custom` runs, sending extra events for mode change,
    selection mode change, cursor update, and selection change

### Proposed Event Flow

With the new architecture, the flow would be:

1. Send editor action to backend
2. Backend creates an `AppMessage::IntegrationDispatch(Dispatch::ToEditor(DispatchEditor::Undo))` and sends it to Ki via
   `app_sender`
3. Ki processes the message in its event loop via `process_message`, which calls `handle_dispatch` internally
4. At key points, Ki emits IntegrationEvents (e.g., `BufferChanged`, `SelectionChanged`, `ModeChanged`, `CursorUpdate`)
   through the integration channel
5. VSCodeApp receives these events from the integration channel (instead of `from_app_receiver`)
6. VSCodeApp translates these events to VSCode-specific protocol messages (e.g., `buffer.diff`, `selection.update`,
   `mode.change`, `cursor.update`)
7. VSCodeApp sends the messages to VSCode via the IPC channel

This flow is cleaner, more decoupled, and easier to understand and maintain. Key improvements:

1. **Removal of VSCode-specific code from Ki**: No more `#[cfg(feature = "vscode")]` blocks in Ki
2. **Simplified communication**: One clear channel for integration events instead of scattered notification points
3. **Better separation of concerns**: Ki focuses on editor functionality, VSCodeApp handles VSCode integration
4. **Extensibility**: The same integration channel could be used for other integrations in the future

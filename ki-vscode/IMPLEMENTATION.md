# Ki VSCode integration plan

## Overview

Write a vscode plugin that allows ki-editor to drive the vscode editor. The vscode editor should behave as if it were
ki-editor (modal, multi cursor, AST aware editing, advanced movement and selection modes, selection mode -> movement ->
action workflow)

## Guidance

1.  We want to avoid making changes to the core ki-editor _where possible_. However, enabling bidirectional
    communication might require minimal additions (like an outgoing notification channel). Changes like making items
    `pub` are acceptable.
2.  NO MATTER WHAT do not implement "fixes" that replicate ki-editor functionality in the vscode side of the plugin.
    This is just masking problems instead of solving them and defeats the purpose of the project.

## Rust side IPC (`src/vscode/`)

The `VSCodeApp` wrapper on the Rust side should:

1.  Manage the IPC connection (WebSocket) with the VSCode extension.
2.  Receive `InputMessage`s from VSCode.
3.  Translate `InputMessage`s into appropriate calls to the wrapped `ki-editor::App` instance (e.g., calling
    `handle_event` or `handle_dispatch`).
4.  Receive notifications about state changes from the core `App` instance via a dedicated channel.
5.  Translate these internal notifications into `OutputMessage`s and send them back to VSCode via the WebSocket.
6.  Avoid running the core `App::run` method; instead, drive the `App` directly via method calls.

## Rust -> VSCode State Synchronization

A critical requirement is notifying the VSCode extension when the internal state of the core `ki-editor::App` changes
(e.g., cursor moves, buffer is edited, mode changes). Since `VSCodeApp` drives the core `App` directly rather than
running its event loop, this requires an explicit notification mechanism:

1.  **Outgoing Channel:** Introduce a dedicated channel (e.g., `std::sync::mpsc`) for the core `App` to send state
    change notifications _outwards_.
2.  **Core `App` Modification:** Modify the core `App` (`src/app.rs`):
    -   Store the `Sender` end of this new channel.
    -   Instrument the relevant state-modifying methods (primarily within `handle_dispatch` and the functions it calls)
        to send specific notification messages (e.g., `AppNotification::CursorUpdated`,
        `AppNotification::BufferChanged`) via this sender whenever the state relevant to VSCode changes.
3.  **`VSCodeApp` Reception:** The main loop of `VSCodeApp` (`src/vscode/app.rs`) will hold the `Receiver` end of this
    channel.
    -   It must periodically check this receiver (e.g., using `try_recv`).
    -   When a notification is received, `VSCodeApp` translates it into the appropriate
        `ki-protocol-types::OutputMessage` and sends it to VSCode via the WebSocket IPC handler.

This ensures that VSCode reflects state changes initiated within the Rust backend.

## TS side (`ki-vscode/`)

1.  Establish and manage the WebSocket connection to the Rust backend.
2.  Listen to appropriate VSCode editor events (keyboard input via `type` command override, document changes, selection
    changes, etc.).
3.  Translate VSCode events into `InputMessage`s and send them to the Rust backend via the WebSocket.
4.  Listen for `OutputMessage`s from the Rust backend via the WebSocket.
5.  Dispatch incoming `OutputMessage`s to handlers (`ki-vscode/src/handlers/`) responsible for updating the VSCode
    editor state (moving cursor, updating text, changing decorations, etc.).
6.  Filter out non-editor documents (e.g., output panels) to prevent feedback loops.
7.  Keep handlers simple, focused on translating specific messages to VSCode API calls. Avoid complex state management,
    optimization, or retry logic within the TS side.

## Simplified Architecture Plan (Rust Side Principles)

The `VSCodeApp` implementation (`src/vscode/app.rs`) should adhere to these principles, regardless of the specific IPC
mechanism:

1.  **Minimal State**: `VSCodeApp` should act primarily as an IPC and translation layer. It should avoid duplicating
    editor state already managed by the core `ki-editor::App`. Necessary state includes IPC connection details and
    potentially mappings needed for translation (like buffer URIs to internal IDs).
2.  **Direct Ownership**: Core components needed for `VSCodeApp`'s function (like the IPC handler, the wrapped `App`
    instance via `Arc<Mutex<>>`) should be directly owned, avoiding excessive use of `Option` for required elements.
3.  **Channel-Based Communication**: Use standard Rust channels (`std::sync::mpsc`) for communication between the main
    `VSCodeApp` thread and any helper threads (e.g., the WebSocket reader thread) and for receiving state update
    notifications from the core `App`.

## IPC Mechanism Update: Switching from Stdin/Stdout to WebSockets

### Problem

The previous IPC mechanism relying on stdin/stdout conflicts with the core `ki-editor::App`'s potential need to use
these streams for subprocesses like LSP servers (`rust-analyzer`).

### Solution

The IPC mechanism will be switched to use **WebSockets** over TCP.

### Implementation Strategy (Synchronous Approach)

To maintain the existing synchronous structure of `VSCodeApp` and avoid introducing `tokio` at this stage, a synchronous
WebSocket implementation will be used.

**Technology:**

-   **Rust:** `tungstenite` crate with standard library networking (`std::net`).
-   **TypeScript:** Standard `WebSocket` client (e.g., using the `ws` library).

**Implementation Steps:**

1.  **Rust Backend (`src/vscode/`)**:

    -   Replace the current stdin/stdout `VscodeIpc` struct.
    -   Create a new WebSocket IPC handler using `tungstenite` and `std::net::TcpListener`.
    -   Bind the listener to `127.0.0.1:0` (requesting an OS-assigned port).
    -   **Crucially:** Print the assigned port to the original stdout (`println!("KI_LISTENING_ON={}", port);`) and
        flush stdout immediately after binding. This is stdout's sole purpose.
    -   Spawn a dedicated `std::thread` for blocking WebSocket handling:
        -   Accept one incoming TCP connection (`listener.accept()`).
        -   Perform the WebSocket handshake using `tungstenite::accept`.
        -   Loop:
            -   Read incoming WebSocket messages (`websocket.read_message()`).
            -   Deserialize valid text messages to `InputMessageWrapper`.
            -   Send deserialized messages to the main `VSCodeApp` thread via an `mpsc` channel.
            -   Receive outgoing `OutputMessageWrapper`s from the main thread via another `mpsc` channel and send them
                over the WebSocket (`websocket.write_message()`).
    -   Remove the old 4-byte length-prefixing logic.

2.  **VSCode Extension (`ki-vscode/src/`)**:

    -   Modify `IPC::start`:
        -   Spawn the Rust process.
        -   Listen to the process's `stdout` until the `KI_LISTENING_ON=PORT` line is received.
        -   Parse the `PORT`.
        -   Stop listening to `stdout`.
        -   Connect a WebSocket client to `ws://localhost:PORT`.
    -   Modify IPC message handling:
        -   Send `InputMessageWrapper`s via `websocket.send()`.
        -   Listen for WebSocket `message` events for incoming data.
        -   Deserialize event data to `OutputMessageWrapper`.
        -   Pass deserialized messages to the `Dispatcher`.
    -   Remove the old 4-byte length-prefixing and stream buffering logic.

3.  **Core App -> `VSCodeApp` Notifications**:
    -   Implement the outgoing notification channel mechanism described in the "Rust -> VSCode State Synchronization"
        section.
    -   The main `VSCodeApp` loop must poll the receiver end of this channel and forward notifications over the
        WebSocket via the handler thread.

---

_(Note: The following sections outline important implementation details and cleanup tasks. While the IPC mechanism and
state synchronization are the immediate priorities, these remain relevant for long-term correctness and
maintainability.)_

## Essential Protocol Events

### From VSCode to Ki (InputMessage)

-   `buffer.open` - Document opened
-   `buffer.close` - Document closed
-   `buffer.save` - Document saved
-   `buffer.change` - Document content changed
-   `buffer.active` - Active editor changed
-   `cursor.update` - Cursor position changed
-   `selection.set` - Selection changed
-   `mode.set` - Mode changed (including selection mode)
-   `keyboard.input` - Keyboard input received
-   `ping` - Connection test

### From Ki to VSCode (OutputMessage)

-   `buffer.update` / `buffer.diff` - Update document content
-   `cursor.update` - Update cursor positions
-   `selection.update` - Update selections
-   `mode.change` - Update editor mode
-   `error` / `success` - Operation results
-   `ping` - Connection response

## TypeScript Implementation Todo

1.  **Fix selection handler**

    -   Update `Selection` type usage to match protocol (start, end, is_extended)
    -   Add `primary` field to `SelectionSet`

2.  **Add missing handlers**

    -   Implement selection mode handler
    -   Implement search functionality

3.  **Fix protocol inconsistencies**

    -   Remove duplicate document.\* events
    -   Ensure protocol consistency between handlers

4.  **Add missing VSCode event listeners**

    -   Scroll synchronization (onDidChangeTextEditorVisibleRanges)
    -   Configuration changes

5.  **Clean up IPC service**

    -   Simplify connection management
    -   Standardize error handling

6.  **Testing and validation**
    -   Verify all protocol messages are handled correctly
    -   Ensure proper event propagation in both directions

## Implementation Clean-up Plan

After reviewing the current implementation, several issues need to be addressed to align with our original plan:

1.  **Consolidate buffer handling**

    -   Merge `buffer_manager.ts` and `document_change_manager.ts` into a single `buffer_handler.ts`
    -   Remove optimization logic, version conflict resolution, and complex debouncing
    -   Maintain a simple 1:1 mapping between VSCode document events and Ki notifications

2.  **Simplify architecture**

    -   Replace complex manager classes with simple event handlers
    -   Remove performance optimization code (prioritization, debouncing, batching)
    -   Eliminate complex error recovery and retry logic
    -   Keep core functionality focused on event translation only

3.  **Reorganize modules**

    -   Move all handlers to the `handlers/` directory

    *   Ensure each handler handles exactly one type of event/message
    *   Remove `*_manager.ts` files in favor of single-purpose handlers
    *   Create a central registry that maps events to handlers

4.  **Standardize event handling**

    -   Implement a single event dispatcher that routes all events
    -   Avoid duplicate event listeners across multiple files
    -   Ensure clean, consistent flow of events between VSCode and Ki

5.  **Unify IPC communication**
    -   Consolidate `ipc_client.ts` and `ipc_service.ts` into a single `ipc.ts` (as per the WebSocket plan)
    -   Implement a clean protocol layer that matches the Rust side
    -   Simplify message passing with no special cases for message types

These changes will bring the implementation closer to the original design principles of simplicity, clean separation,
and maintainability.

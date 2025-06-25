[![Version](https://img.shields.io/visual-studio-marketplace/v/ki-editor.ki-editor-vscode)](https://marketplace.visualstudio.com/items?itemName=ki-editor.ki-editor-vscode)
[![Installs](https://img.shields.io/visual-studio-marketplace/i/ki-editor.ki-editor-vscode)](https://marketplace.visualstudio.com/items?itemName=ki-editor.ki-editor-vscode)
[![Rating](https://img.shields.io/visual-studio-marketplace/r/ki-editor.ki-editor-vscode)](https://marketplace.visualstudio.com/items?itemName=ki-editor.ki-editor-vscode)
[![GitHub](https://img.shields.io/github/license/ki-editor/ki-editor)](https://github.com/ki-editor/ki-editor)

# Ki Editor for Visual Studio Code

> ⚠️ **BETA VERSION**: This extension is currently in beta and definitely contains bugs and incomplete features. Use
> with caution.

This extension integrates the Ki Editor with Visual Studio Code, providing a powerful and efficient editing experience.
The extension includes statically linked binaries for all major platforms (Windows, macOS, and Linux), so you don't need
to install anything separately.

## Why?

Ki as a standalone editor is quite barebones. Although it has basic LSP support and syntax highlighting, it lacks
important IDE features such as AI coding assistance, Git integrations, debuggers, extensive plugin ecosystems, and all
the other goodies that make modern development productive.

On the other hand, VS Code's default editing experience can feel sluggish.

Using Ki within VS Code gives you the best of both worlds: you get access to the vast ecosystem of VS Code extensions
and features, while enjoying Ki's fluid and efficient text editing motions.

## Features

-   🚀 Most Ki Editor functionality in VSCode.
-   See the [Ki Editor documentation](https://ki-editor.github.io/ki-editor/) to learn about Ki's innovative editing
    model.
-   Static binaries for most platforms included, no need to install anything

## Mapped functionalities

1. Diagnostics
1. Marks
1. Git Hunks
1. Text movements
1. Text operations (e.g. Delete, Swap etc)
1. LSP operations (e.g. Go to Definition)
1. Change keyboard layout (via `space t`)
1. Global operations (e.g. Global Search, Global Marks, Global Diagnostics)

## Unmapped functionalities

1. Window operations (e.g. change tab, pin tab, swap pane, etc.)

## Quick Start

1. **Installation**

    ```bash
    # Install from VS Code Marketplace
    code --install-extension ki-editor.ki-editor-vscode

    # Or install from a local VSIX file
    code --install-extension ki-editor-vscode-0.0.3.vsix
    ```

    The extension includes statically linked binaries for all major platforms (Windows, macOS, and Linux), so you don't
    need to install the Ki editor separately.

2. **Basic Usage**
    - Open a file in VSCode
    - Press `Esc` to enter Normal mode
    - Use Ki commands and keybindings
    - Press `u` to return to Insert mode

## Extension Settings

-   `ki.backendPath`: Optional path to the Ki editor backend executable. If not specified, the bundled platform-specific
    binary will be used.
-   `ki.enableDebugLogging`: Enable debug logging (default: false)
-   `ki.maxFileSize`: Maximum file size to process in bytes (default: 2MB)

## Known Issues

See our [issue tracker](https://github.com/ki-editor/ki-editor/issues) for current issues and planned features.

## Development Setup

### Prerequisites

-   Node.js (v16 or later)
-   npm (v8 or later)
-   Bun (v1.0.30 or later) - for bundling the extension
-   Visual Studio Code
-   Nix package manager (for building the Ki binaries)
-   [Just](https://github.com/casey/just)

### Development instructions

1. Clone the repository:

```bash
git clone https://github.com/ki-editor/ki-editor.git
cd ki-editor
```

2. Install dependencies for the VSCode extension:

```bash
cd ki-vscode
npm install
```

3. Build Ki's bridge:

```bash
just watch-build
```

4. Running the extension
    - This step is a little troublesome, because first you have to open VS Code in the `ki-vscode` folder of the
      `ki-editor` repository
    - Then, press F5 in VSCode to launch a new window with the extension loaded

### Building Ki Binaries

The extension includes statically linked binaries for all major platforms. To build these binaries:

1. Make sure you have Nix installed:

    ```bash
    # On macOS/Linux
    sh <(curl -L https://nixos.org/nix/install) --daemon

    # On Windows, follow the instructions at https://nixos.org/download.html
    ```

2. Run this command at the repository root (not `ki-vscode`):

    ```bash
    just vscode-package
    ```

    This command will:

    - Build the Ki editor for all supported platforms using Nix
    - Copy the binaries to `ki-vscode/dist/bin` with platform-specific names
    - Make the binaries executable
    - Package the extension into a `.vsix` package under the `ki-vscode` directory

3. The following binaries will be generated:
    - `ki-darwin-arm64`: macOS ARM64 (Apple Silicon)
    - `ki-darwin-x64`: macOS x86_64 (Intel)
    - `ki-linux-x64`: Linux x86_64
    - `ki-linux-arm64`: Linux ARM64
    - `ki-win32-x64.exe`: Windows x86_64

### Packaging

Create a VSIX package with the extension and all platform binaries:

```bash
npm run package:full
```

This will build the binaries for all platforms and package them with the extension.

#### Bundling

The extension uses Bun to bundle all dependencies, ensuring they're available when the extension is installed from the
marketplace:

```bash
npm run bundle
```

This creates a single bundle file that includes all dependencies, including the 'ws' module used for WebSocket
communication. The bundling script also ensures that binary files have the correct executable permissions on Unix-like
systems.

The bundling process is automatically run as part of the `vscode:prepublish` script when packaging the extension.

## License

[MPL 2.0](LICENSE)

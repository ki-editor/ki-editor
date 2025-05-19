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

## Features

- 🚀 Full Ki Editor functionality in VSCode.
- See the [Ki Editor documentation](https://ki-editor.github.io/ki-editor/) to learn about Ki's innovative editing
  model.
- Static binaries for all platforms included, no need to install anything

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
    - Press `i` to return to Insert mode

## Extension Settings

- `ki.backendPath`: Optional path to the Ki editor backend executable. If not specified, the bundled platform-specific
  binary will be used.
- `ki.enableDebugLogging`: Enable debug logging (default: false)
- `ki.maxFileSize`: Maximum file size to process in bytes (default: 2MB)

## Known Issues

See our [issue tracker](https://github.com/ki-editor/ki-editor/issues) for current issues and planned features.

## Development Setup

### Prerequisites

- Node.js (v16 or later)
- npm (v8 or later)
- Bun (v1.0.30 or later) - for bundling the extension
- Visual Studio Code
- Nix package manager (for building the Ki binaries)

### Build Instructions

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

3. Compile the extension:

```bash
npm run compile
```

4. Launch in development mode:
    - Press F5 in VSCode to launch a new window with the extension loaded
    - Or run the "Extension" launch configuration from the Run view

### Building Ki Binaries

The extension includes statically linked binaries for all major platforms. To build these binaries:

1. Make sure you have Nix installed:

    ```bash
    # On macOS/Linux
    sh <(curl -L https://nixos.org/nix/install) --daemon

    # On Windows, follow the instructions at https://nixos.org/download.html
    ```

2. Build the binaries for all platforms:

    ```bash
    # From the root of the repository
    ./build-all-platforms.sh

    # Or from the ki-vscode directory
    npm run build:binaries
    ```

    This will:

    - Build the Ki editor for all supported platforms using Nix
    - Copy the binaries to `ki-vscode/dist/bin` with platform-specific names
    - Make the binaries executable

3. The following binaries will be generated:
    - `ki-darwin-arm64`: macOS ARM64 (Apple Silicon)
    - `ki-darwin-x64`: macOS x86_64 (Intel)
    - `ki-linux-x64`: Linux x86_64
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

# Ki Editor VSCode Extension

[![Version](https://img.shields.io/visual-studio-marketplace/v/ki-editor.ki-vscode)](https://marketplace.visualstudio.com/items?itemName=ki-editor.ki-vscode)
[![Installs](https://img.shields.io/visual-studio-marketplace/i/ki-editor.ki-vscode)](https://marketplace.visualstudio.com/items?itemName=ki-editor.ki-vscode)
[![Rating](https://img.shields.io/visual-studio-marketplace/r/ki-editor.ki-vscode)](https://marketplace.visualstudio.com/items?itemName=ki-editor.ki-vscode)
[![Build Status](https://github.com/ki-editor/ki-vscode/workflows/CI/badge.svg)](https://github.com/ki-editor/ki-vscode/actions)

This extension integrates the Ki Editor with Visual Studio Code, providing a powerful and efficient editing experience.

## Features

- 🚀 Full Ki Editor functionality in VSCode
- ⚡ Efficient buffer synchronization
- 🎯 Precise cursor and selection management
- 🔄 Seamless mode transitions
- 💻 Native VSCode command integration
- 🎨 Customizable UI elements
- **Selection Modes**: Multiple selection modes (Word, Token, Character, Line)
- **Cursor Integration**: Multi-cursor support with mode-specific styling
- **Mode System**: Visual indicators and context variables for different editing modes
- **Command System**: Comprehensive command mapping and dispatch system

## Quick Start

1. **Installation**

   ```bash
   # Install from VS Code Marketplace
   code --install-extension ki-editor.ki-vscode

   # Or build from source
   git clone https://github.com/ki-editor/ki-vscode
   cd ki-vscode
   npm install
   npm run build
   ```

2. **Configuration**

   ```json
   {
     "ki.executablePath": "/path/to/ki",
     "ki.logLevel": "info"
   }
   ```

3. **Basic Usage**
   - Open a file in VSCode
   - Press `Esc` to enter Normal mode
   - Use Ki commands and keybindings
   - Press `i` to return to Insert mode

## Documentation

- [User Guide](docs/user-guide.md) - Complete user documentation
- [Developer Guide](docs/dev/developer-guide.md) - Development and API documentation
- [Integration Guide](docs/integration-guide.md) - Details on VSCode integration
- [Contributing Guide](CONTRIBUTING.md) - How to contribute to the project

## Requirements

- Visual Studio Code 1.60.0+
- Ki Editor installed on system
- Node.js 16+ (for development)
- Rust toolchain (for development)

## Extension Settings

- `ki.executablePath`: Path to Ki executable
- `ki.logLevel`: Logging level (debug, info, warn, error)
- `ki.enableExperimentalFeatures`: Enable experimental features
- `ki.statusLineComponents`: Customize status line display

## Known Issues

See our [issue tracker](https://github.com/ki-editor/ki-vscode/issues) for current issues and planned features.

## Release Notes

### 1.0.0

- Initial release
- Core Ki functionality
- Basic VSCode integration

### 0.9.0

- Beta release
- Feature complete
- Performance improvements

### 0.5.0

- Alpha release
- Core functionality
- Basic features

## Development Setup

### Prerequisites

- Node.js (v16 or later)
- npm (v8 or later)
- Visual Studio Code

### Build Instructions

1. Clone the repository:

```bash
git clone https://github.com/ki-editor/ki-editor.git
cd ki-editor/ki-vscode
```

2. Install dependencies:

```bash
npm install
```

3. Compile the extension:

```bash
npm run compile
```

4. Launch in development mode:
   - Press F5 in VSCode to launch a new window with the extension loaded
   - Or run the "Extension" launch configuration from the Run view

### Continuous Development

For continuous development, use:

```bash
npm run watch
```

This will watch for changes in the TypeScript files and recompile automatically.

### Testing

Run the tests with:

```bash
npm test
```

### Packaging

Create a VSIX package with:

```bash
npm run package
```

## Contributing

Contributions are welcome! Please check the [CONTRIBUTING.md](../CONTRIBUTING.md) file for guidelines.

## License

[MIT](LICENSE)

## Support

- [Documentation](docs/)
- [GitHub Issues](https://github.com/ki-editor/ki-vscode/issues)
- [Community Forum](https://forum.ki-editor.org)
- [Email Support](mailto:support@ki-editor.org)

## Acknowledgments

- VSCode team for the excellent extension API
- Ki Editor community for feedback and support
- Contributors who helped make this possible

```

```

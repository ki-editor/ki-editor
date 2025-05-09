# Contributing to Ki VSCode Extension

Thank you for your interest in contributing to the Ki VSCode extension! This document provides guidelines and
instructions for contributing.

## Code of Conduct

This project adheres to the Contributor Covenant code of conduct. By participating, you are expected to uphold this
code. Please report unacceptable behavior to conduct@ki-editor.org.

## Development Setup

### Prerequisites

1. **Required Software**

    - Node.js 16+
    - Rust toolchain (2021 edition)
    - Visual Studio Code
    - Git

2. **Recommended VSCode Extensions**
    - ESLint
    - Prettier
    - rust-analyzer
    - CodeLLDB

### Getting Started

1. **Clone the Repository**

    ```bash
    git clone https://github.com/ki-editor/ki-vscode
    cd ki-vscode
    ```

2. **Install Dependencies**

    ```bash
    npm install
    ```

3. **Build the Extension**

    ```bash
    npm run build
    ```

4. **Run Tests**
    ```bash
    npm test
    ```

## Development Workflow

### Branch Naming

- Feature branches: `feature/description`
- Bug fixes: `fix/issue-description`
- Documentation: `docs/description`
- Performance improvements: `perf/description`

### Commit Messages

Follow the [Conventional Commits](https://www.conventionalcommits.org/) specification:

```
type(scope): description

[optional body]

[optional footer]
```

Types:

- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation
- `style`: Code style changes
- `refactor`: Code refactoring
- `perf`: Performance improvements
- `test`: Adding/updating tests
- `chore`: Maintenance tasks

### Code Style

#### TypeScript

- Use strict mode
- Follow ESLint configuration
- Use type annotations
- Document public APIs
- Write unit tests

```typescript
// Good
export interface BufferManager {
    /**
     * Applies changes to the buffer
     * @param changes Array of text changes
     * @returns Promise that resolves when changes are applied
     */
    applyChanges(changes: TextChange[]): Promise<void>;
}

// Bad
export interface BufferManager {
    applyChanges(changes: any): void;
}
```

#### Rust

- Follow rustfmt style
- Use clippy lints
- Document public items
- Handle errors properly
- Write unit tests

```rust
// Good
/// Manages buffer synchronization between Ki and VSCode
#[derive(Debug)]
pub struct BufferManager {
    buffers: HashMap<buffer_id, Buffer>,
}

impl BufferManager {
    /// Creates a new buffer manager
    pub fn new() -> Self {
        Self {
            buffers: HashMap::new(),
        }
    }
}

// Bad
pub struct BufferManager {
    pub buffers: HashMap<buffer_id, Buffer>,
}
```

### Testing

1. **Unit Tests**

    - Write tests for new features
    - Update tests for changes
    - Maintain test coverage
    - Test error cases

2. **Integration Tests**

    - Test complete workflows
    - Test cross-component interaction
    - Test error handling
    - Test performance impact

3. **Manual Testing**
    - Test in different environments
    - Test edge cases
    - Test user workflows
    - Test error scenarios

## Pull Request Process

1. **Before Submitting**

    - Update documentation
    - Add/update tests
    - Run all checks
    - Update changelog
    - Rebase on main

2. **PR Requirements**

    - Clear description
    - Issue reference
    - Test coverage
    - Documentation updates
    - Clean commit history

3. **Review Process**

    - Code review by maintainers
    - CI checks must pass
    - Documentation review
    - Performance review

4. **After Merge**
    - Delete feature branch
    - Update related issues
    - Monitor CI/CD
    - Watch for regressions

## Release Process

1. **Preparation**

    - Update version
    - Update changelog
    - Review documentation
    - Run full test suite

2. **Release Steps**

    - Create release branch
    - Run final checks
    - Create GitHub release
    - Publish to marketplace

3. **Post-Release**
    - Monitor issues
    - Gather feedback
    - Update documentation
    - Plan next release

## Documentation

### Types of Documentation

1. **Code Documentation**

    - API documentation
    - Function documentation
    - Type documentation
    - Example usage

2. **User Documentation**

    - Installation guide
    - User guide
    - Configuration
    - Troubleshooting

3. **Developer Documentation**
    - Architecture guide
    - Development setup
    - Contributing guide
    - API reference

### Documentation Style

- Clear and concise
- Include examples
- Keep updated
- Link related docs

## Getting Help

- Check existing issues
- Join community chat
- Ask in discussions
- Email maintainers

## License

By contributing, you agree that your contributions will be licensed under the project's MIT License.

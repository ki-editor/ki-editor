#!/usr/bin/env bash
set -euo pipefail

# Script to build Ki Editor for all platforms and copy binaries to ki-vscode/dist/bin
# with platform-specific names

# Create the output directory if it doesn't exist
mkdir -p ki-vscode/dist/bin

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${YELLOW}Building Ki Editor for all platforms...${NC}"

# Build for native platform (current architecture)
echo -e "${GREEN}Building for native platform...${NC}"
nix build -L
# Immediately resolve and save the path
NATIVE_PATH=$(readlink -f result 2>/dev/null || echo "")

# Build for aarch64-darwin
echo -e "${GREEN}Building for aarch64-darwin...${NC}"
nix build -L .#aarch64-darwin
# Immediately resolve and save the path
AARCH64_DARWIN_PATH=$(readlink -f result 2>/dev/null || echo "")

# Build for x86_64-linux-musl
echo -e "${GREEN}Building for x86_64-linux-musl...${NC}"
nix build -L .#x86_64-linux-musl
# Immediately resolve and save the path
X86_64_LINUX_MUSL_PATH=$(readlink -f result 2>/dev/null || echo "")

# Build for aarch64-linux
echo -e "${GREEN}Building for aarch64-linux...${NC}"
nix build -L .#aarch64-linux
# Immediately resolve and save the path
AARCH64_LINUX_PATH=$(readlink -f result 2>/dev/null || echo "")

# Build for x86_64-windows-gnu
echo -e "${GREEN}Building for x86_64-windows-gnu...${NC}"
nix build -L .#x86_64-windows-gnu
# Immediately resolve and save the path
X86_64_WINDOWS_GNU_PATH=$(readlink -f result 2>/dev/null || echo "")

echo -e "${YELLOW}Copying binaries to ki-vscode/dist/bin...${NC}"

echo -e "${YELLOW}Build paths:${NC}"
echo "Native: ${NATIVE_PATH}"
echo "aarch64-darwin: ${AARCH64_DARWIN_PATH}"
echo "x86_64-linux-musl: ${X86_64_LINUX_MUSL_PATH}"
echo "aarch64-linux: ${AARCH64_LINUX_PATH}"
echo "x86_64-windows-gnu: ${X86_64_WINDOWS_GNU_PATH}"

# Copy native build (determine platform)
if [[ -n "${NATIVE_PATH}" ]]; then
    if [[ "$(uname)" == "Darwin" ]]; then
        if [[ "$(uname -m)" == "arm64" ]]; then
            # Native is already aarch64-darwin
            cp -f "${NATIVE_PATH}/bin/ki" ki-vscode/dist/bin/ki-darwin-arm64
            chmod +x ki-vscode/dist/bin/ki-darwin-arm64
            echo -e "${GREEN}Copied native build to ki-vscode/dist/bin/ki-darwin-arm64${NC}"
        else
            # Native is x86_64-darwin
            cp -f "${NATIVE_PATH}/bin/ki" ki-vscode/dist/bin/ki-darwin-x64
            chmod +x ki-vscode/dist/bin/ki-darwin-x64
            echo -e "${GREEN}Copied native build to ki-vscode/dist/bin/ki-darwin-x64${NC}"
        fi
    elif [[ "$(uname)" == "Linux" ]]; then
        if [[ "$(uname -m)" == "aarch64" ]]; then
            # Native is aarch64-linux
            cp -f "${NATIVE_PATH}/bin/ki" ki-vscode/dist/bin/ki-linux-arm64
            chmod +x ki-vscode/dist/bin/ki-linux-arm64
            echo -e "${GREEN}Copied native build to ki-vscode/dist/bin/ki-linux-arm64${NC}"
        else
            # Native is x86_64-linux
            cp -f "${NATIVE_PATH}/bin/ki" ki-vscode/dist/bin/ki-linux-x64
            chmod +x ki-vscode/dist/bin/ki-linux-x64
            echo -e "${GREEN}Copied native build to ki-vscode/dist/bin/ki-linux-x64${NC}"
        fi
    fi
else
    echo -e "${YELLOW}Native build not found${NC}"
fi

# Copy aarch64-darwin build
if [[ -n "${AARCH64_DARWIN_PATH}" && -f "${AARCH64_DARWIN_PATH}/bin/ki" ]]; then
    cp -f "${AARCH64_DARWIN_PATH}/bin/ki" ki-vscode/dist/bin/ki-darwin-arm64
    chmod +x ki-vscode/dist/bin/ki-darwin-arm64
    echo -e "${GREEN}Copied aarch64-darwin build to ki-vscode/dist/bin/ki-darwin-arm64${NC}"
else
    echo -e "${YELLOW}aarch64-darwin build not found${NC}"
fi

# Copy x86_64-linux-musl build
if [[ -n "${X86_64_LINUX_MUSL_PATH}" && -f "${X86_64_LINUX_MUSL_PATH}/bin/ki" ]]; then
    cp -f "${X86_64_LINUX_MUSL_PATH}/bin/ki" ki-vscode/dist/bin/ki-linux-x64
    chmod +x ki-vscode/dist/bin/ki-linux-x64
    echo -e "${GREEN}Copied x86_64-linux-musl build to ki-vscode/dist/bin/ki-linux-x64${NC}"
else
    echo -e "${YELLOW}x86_64-linux-musl build not found${NC}"
fi

# Copy aarch64-linux build
if [[ -n "${AARCH64_LINUX_PATH}" && -f "${AARCH64_LINUX_PATH}/bin/ki" ]]; then
    cp -f "${AARCH64_LINUX_PATH}/bin/ki" ki-vscode/dist/bin/ki-linux-arm64
    chmod +x ki-vscode/dist/bin/ki-linux-arm64
    echo -e "${GREEN}Copied aarch64-linux build to ki-vscode/dist/bin/ki-linux-arm64${NC}"
else
    echo -e "${YELLOW}aarch64-linux build not found${NC}"
fi

# Copy x86_64-windows-gnu build
if [[ -n "${X86_64_WINDOWS_GNU_PATH}" && -f "${X86_64_WINDOWS_GNU_PATH}/bin/ki.exe" ]]; then
    cp -f "${X86_64_WINDOWS_GNU_PATH}/bin/ki.exe" ki-vscode/dist/bin/ki-win32-x64.exe
    echo -e "${GREEN}Copied x86_64-windows-gnu build to ki-vscode/dist/bin/ki-win32-x64.exe${NC}"
else
    echo -e "${YELLOW}x86_64-windows-gnu build not found${NC}"
fi

echo -e "${YELLOW}Checking the copied binaries:${NC}"
ls -la ki-vscode/dist/bin/

# Clean up the final result symlink
echo -e "${YELLOW}Cleaning up...${NC}"
rm -f result

echo -e "${GREEN}Done!${NC}"

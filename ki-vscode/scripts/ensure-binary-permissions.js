#!/usr/bin/env node

/**
 * This script ensures that the bundled binaries have the correct permissions.
 * It's run as part of the vscode:prepublish script.
 */

const fs = require("fs");
const path = require("path");
const { execSync } = require("child_process");

// Get the platform
const platform = process.platform;

// Only need to set executable permissions on Unix-like systems
if (platform !== "win32") {
    console.log("Ensuring binaries have executable permissions...");

    const binDir = path.join(__dirname, "..", "dist", "bin");

    try {
        // Check if the directory exists
        if (fs.existsSync(binDir)) {
            // Get all files in the bin directory
            const files = fs.readdirSync(binDir);

            // Set executable permissions for each file
            for (const file of files) {
                if (!file.endsWith(".exe")) {
                    // Skip .exe files as they don't need +x on Windows
                    const filePath = path.join(binDir, file);
                    console.log(`Setting executable permissions for ${filePath}`);

                    // Set executable permissions (chmod +x)
                    fs.chmodSync(filePath, 0o755);
                }
            }

            console.log("All binaries now have executable permissions.");
        } else {
            console.log(`Binary directory not found: ${binDir}`);
        }
    } catch (error) {
        console.error("Error setting binary permissions:", error);
        process.exit(1);
    }
} else {
    console.log("Running on Windows, no need to set executable permissions.");
}

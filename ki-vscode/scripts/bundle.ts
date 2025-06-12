/**
 * Bundle script for Ki VS Code extension
 *
 * This script uses Bun's built-in bundler to create a single bundle file
 * that includes all dependencies, ensuring they're available when the extension
 * is installed from the marketplace.
 *
 * It also ensures that binary files have the correct permissions.
 */

import { join } from "node:path";
import { mkdir, readdir, chmod } from "node:fs/promises";
import { existsSync } from "node:fs";
import { platform } from "node:os";

async function bundle() {
    console.log("Bundling Ki VS Code extension...");

    // Get the project root directory
    const projectRoot = process.cwd();

    // Define input and output paths
    const entryPoint = join(projectRoot, "src", "extension.ts");
    const outDir = join(projectRoot, "dist");
    const binDir = join(outDir, "bin");

    try {
        // Ensure the dist directory exists
        await mkdir(outDir, { recursive: true });

        // Ensure the bin directory exists
        await mkdir(binDir, { recursive: true });

        // Bundle the extension
        const result = await Bun.build({
            entrypoints: [entryPoint],
            outdir: outDir,
            target: "node",
            format: "cjs", // CommonJS format for VS Code extensions
            sourcemap: "external",
            external: ["vscode"], // Don't bundle vscode API
            minify: true,
        });

        if (!result.success) {
            console.error("Bundle failed:");
            for (const message of result.logs) {
                console.error(message);
            }
            process.exit(1);
        }

        console.log(`Bundle successful! Output: ${outDir}/extension.js`);

        // Set executable permissions on binary files
        if (existsSync(binDir)) {
            // Only need to set executable permissions on Unix-like systems
            if (platform() !== "win32") {
                console.log("Ensuring binaries have executable permissions...");

                // Get all files in the bin directory
                const files = await readdir(binDir);

                // Set executable permissions for each file
                for (const file of files) {
                    if (!file.endsWith(".exe")) {
                        // Skip .exe files as they don't need +x on Windows
                        const filePath = join(binDir, file);
                        console.log(
                            `Setting executable permissions for ${filePath}`,
                        );

                        // Set executable permissions (chmod +x)
                        await chmod(filePath, 0o755);
                    }
                }

                console.log("All binaries now have executable permissions.");
            } else {
                console.log(
                    "Running on Windows, no need to set executable permissions.",
                );
            }
        } else {
            console.log(`Binary directory not found: ${binDir}`);
        }
    } catch (error) {
        console.error("Error during bundling:", error);
        process.exit(1);
    }
}

// Run the bundle function
bundle();

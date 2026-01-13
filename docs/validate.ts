import { fromMarkdown, type Value } from "mdast-util-from-markdown";
import { mdxFromMarkdown } from "mdast-util-mdx";
import { mdx } from "micromark-extension-mdx";
import type { Node, Parent } from "mdast";
import type {
    MdxJsxAttribute,
    MdxJsxFlowElement,
    MdxJsxTextElement,
} from "mdast-util-mdx";

// A type guard to check if a node has children
function isParent(node: Node | Parent): node is Parent {
    return "children" in node;
}

// Pure function to return an array of all nodes in the tree
function flattenTree(node: Node | Parent): (Node | Parent)[] {
    const nodes = [node];
    if (isParent(node)) {
        return nodes.concat(node.children.flatMap(flattenTree));
    }
    return nodes;
}

// Pure predicate function to check if node is a TutorialFallback element
const isFallbackNode =
    (name: string) =>
    (node: Node): node is MdxJsxFlowElement | MdxJsxTextElement => {
        const isFlowElement = node.type === "mdxJsxFlowElement";
        const isJsxTextElement = node.type === "mdxJsxTextElement";
        const isNameFallback = "name" in node && node.name === name;

        return (isFlowElement || isJsxTextElement) && isNameFallback;
    };

// Pure function to extract filename from node attributes
function extractFilename(
    node: MdxJsxFlowElement | MdxJsxTextElement,
): string | null {
    const fileNameAttr = node.attributes?.find(
        (attr) => attr.type === "mdxJsxAttribute" && attr.name === "filename",
    ) as MdxJsxAttribute | undefined;

    return (fileNameAttr?.value as string) || null;
}

export const makeExtractArgumentFileNames =
    (tagName: string) =>
    (mdxContent: Value): (string | null)[] => {
        const tree = fromMarkdown(mdxContent, {
            extensions: [mdx()],
            mdastExtensions: [mdxFromMarkdown()],
        });

        return flattenTree(tree)
            .filter(isFallbackNode(tagName))
            .map(extractFilename);
    };

function validateResourceAccess(
    mdxContent: Value,
    validFilenames: string[],
    tagName: string,
): string[] {
    const argFilenames = makeExtractArgumentFileNames(tagName)(mdxContent);
    return argFilenames.filter(
        (argFilename) => !validFilenames.includes(argFilename as string),
    );
}

module.exports = {
    extractArgumentFileNames: makeExtractArgumentFileNames,
    validateResourceAccess,
};

function validate(tagName: string, resourcesPath: string) {
    const glob = require("glob");
    const path = require("node:path");
    const fs = require("node:fs");

    const staticResources = glob.sync(resourcesPath);
    const validResourceFilenames = staticResources.map((filePath: string) =>
        path.basename(filePath, path.extname(filePath)),
    );

    const mdxFilePaths = glob.sync("docs/**/*.{md,mdx}");

    // Collect all validation outputs
    const allErrors: Array<{ file: string; invalidResources: string[] }> =
        mdxFilePaths
            .map((filePath) => ({
                file: filePath,
                content: fs.readFileSync(filePath, "utf8"),
            }))
            .map(({ file, content }) => ({
                file,
                invalidResources: validateResourceAccess(
                    content,
                    validResourceFilenames,
                    tagName,
                ),
            }))
            .filter(({ invalidResources }) => invalidResources.length > 0);

    // Log all invalid accesses per file
    allErrors.forEach(({ file, invalidResources }) => {
        console.error(`Invalid static resource accesses in ${file}:`);
        console.error(`\t[${invalidResources.join(", ")}]\n`);
    });

    // Throw one Error for any and all invalid access file paths
    if (allErrors.length > 0) {
        const totalInvalidCount = allErrors.reduce(
            (sum, { invalidResources }) => sum + invalidResources.length,
            0,
        );
        throw new Error(
            `Found ${totalInvalidCount} invalid static resource access(es) across ${allErrors.length} file(s)`,
        );
    }

    console.log(
        `\tALL STATIC RESOURCE ACCESSES of ${tagName} in md/mdx WERE VALID!`,
    );
}

if (require.main === module) {
    validate("TutorialFallback", "static/recipes/*.json");
    validate("KeymapFallback", "static/keymaps/*.json");
}

import { fromMarkdown, Value } from 'mdast-util-from-markdown';
import { mdxFromMarkdown } from 'mdast-util-mdx';
import { mdx } from 'micromark-extension-mdx';
import { Node, Parent } from 'mdast';
import { MdxJsxAttribute, MdxJsxFlowElement, MdxJsxTextElement } from 'mdast-util-mdx';

// A type guard to check if a node has children
function isParent(node: Node | Parent): node is Parent {
  return 'children' in node;
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
function isTutorialFallbackNode(node: Node): node is MdxJsxFlowElement | MdxJsxTextElement {
  const isFlowElement = node.type === 'mdxJsxFlowElement';
  const isJsxTextElement = node.type === 'mdxJsxTextElement';
  const isNameTutorialFallback = 'name' in node && node.name === 'TutorialFallback';
  
  return (isFlowElement || isJsxTextElement) && isNameTutorialFallback;
}

// Pure function to extract filename from node attributes
function extractFilename(node: MdxJsxFlowElement | MdxJsxTextElement): string | null {
  const fileNameAttr = node.attributes?.find(
    attr => attr.type === 'mdxJsxAttribute' && attr.name === 'filename'
  ) as MdxJsxAttribute | undefined;
  
  return (fileNameAttr?.value as string) || null;
}

export function extractArgumentFileNames(mdxContent: Value): (string | null)[] {
  const tree = fromMarkdown(mdxContent, {
    extensions: [mdx()],
    mdastExtensions: [mdxFromMarkdown()]
  });

  return flattenTree(tree)
    .filter(isTutorialFallbackNode)
    .map(extractFilename);
}

function validateResourceAccess(mdxContent: Value, validFilenames: string[]): string[] {
  const argFilenames = extractArgumentFileNames(mdxContent);
  return argFilenames
    .filter(argFilename => !validFilenames.includes(argFilename as string))
}

module.exports = {
  extractArgumentFileNames,
  validateResourceAccess,
};

if (require.main === module) {
  const glob = require('glob');
  const path = require('path');
  const fs = require('fs');

  const staticResources = glob.sync('static/**/*.json');
  const validResourceFilenames = staticResources.map((filePath: string) =>
      path.basename(filePath, path.extname(filePath))
  );

  const mdxFilePaths = glob.sync('docs/**/*.{md,mdx}');
  
  // Collect all validation outputs
  const allErrors: Array<{ file: string; invalidResources: string[] }> = mdxFilePaths
    .map(filePath => ({
      file: filePath,
      content: fs.readFileSync(filePath, 'utf8')
    }))
    .map(({ file, content }) => ({
      file,
      invalidResources: validateResourceAccess(content, validResourceFilenames)
    }))
    .filter(({ invalidResources }) => invalidResources.length > 0);

  // Log all invalid accesses per file
  allErrors.forEach(({ file, invalidResources }) => {
    console.error(`Invalid static resource accesses in ${file}:`);
    console.error(`\t[${invalidResources.join(', ')}]\n`);
  });

  // Throw one Error for any and all invalid access file paths
  if (allErrors.length > 0) {
    const totalInvalidCount = allErrors.reduce((sum, { invalidResources }) => sum + invalidResources.length, 0);
    throw new Error(`Found ${totalInvalidCount} invalid static resource access(es) across ${allErrors.length} file(s)`);
  }

  console.log("\tALL STATIC RESOURCE ACCESSES in md/mdx WERE VALID!");
}
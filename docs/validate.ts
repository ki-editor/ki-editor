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

function validateResourceAccess(mdxContent: Value, validFilenames: string[]): boolean {
  const argFilenames = extractArgumentFileNames(mdxContent);
  const validResources = argFilenames.map(argFileName =>
    validFilenames.includes(argFileName as string)
  );

  validResources.forEach((isValid, index) => {
    if (!isValid) {
      console.log(`\tERROR: NON-EXISTENT STATIC RESOURCE:\t "${argFilenames[index]}"`);
    }
  });

  return validResources.every(isValid => isValid);
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
  let validAccesses = mdxFilePaths
    .map((testFilePath: string) => fs.readFileSync(testFilePath, 'utf8'))
    .map((mdxContent: Value) => validateResourceAccess(mdxContent, validResourceFilenames));

  validAccesses.forEach((validAccess: boolean, index: number) => { 
    if (!validAccess) {
      throw new Error(`Invalid static resource access in ${mdxFilePaths[index]}`)
    }
  });

  if (validAccesses.every(Boolean)) { console.log("\t ALL STATIC RESOURCE ACCESSES in md/mdx WERE VALID!") }
}
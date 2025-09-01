const { fromMarkdown } = require('mdast-util-from-markdown');
const { mdxFromMarkdown } = require('mdast-util-mdx');
const { mdx } = require('micromark-extension-mdx');
// const { visit } = require('unist-util-visit');

// Pure function to return an array of all nodes in the tree
function flattenTree(node) {
  const nodes = [node];
  if (node.children) {
    return nodes.concat(
      node.children.flatMap(child => flattenTree(child))
    );
  }
  return nodes;
}

// Pure predicate function to check if node is a TutorialFallback element
function isTutorialFallbackNode(node) {
  const nodeType = node.type;
  const isFlowElement = nodeType === 'mdxJsxFlowElement';
  const isJsxTextElement = nodeType === 'mdxJsxTextElement';
  const isNameTutorialFallback = node.name === 'TutorialFallback';
  
  return (isFlowElement || isJsxTextElement) && isNameTutorialFallback;
}

// Pure function to extract filename from node attributes
function extractFilename(node) {
  const fileNameAttr = node.attributes?.find(
    attr => attr.type === 'mdxJsxAttribute' && attr.name === 'filename'
  );
  
  return fileNameAttr?.value || null;
}
export function extractArgumentFileNames(mdxContent) {
  const tree = fromMarkdown(mdxContent, {
    extensions: [mdx()],
    mdastExtensions: [mdxFromMarkdown()]
  });

  return flattenTree(tree)
    .filter(isTutorialFallbackNode)
    .map(extractFilename)
}

function validateResourceAccess(mdxContent: String, validFilenames: Array<String>) {
  const argFilenames = extractArgumentFileNames(mdxContent);
  const validResources = argFilenames.map(argFileName =>
    validFilenames.includes(argFileName)
  );

  let hasInvalidResource = false;
  validResources.forEach((isValid, index) => {
    if (!isValid) {
      console.log(`\tERROR: NON-EXISTENT STATIC RESOURCE:\t "${argFilenames[index]}"`);
      hasInvalidResource = true;
    }
  });

  return !hasInvalidResource;
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
  const validResourceFilenames = staticResources.map(filePath =>
      path.basename(filePath, path.extname(filePath))
  );
  const mdxFilePaths = glob.sync('docs/**/*.{md,mdx}');
  let validAccesses = mdxFilePaths
    .map(testFilePath => fs.readFileSync(testFilePath, 'utf8'))
    .map(mdxContent => validateResourceAccess(mdxContent, validResourceFilenames));

  validAccesses.forEach((validAccess, index) => { 
    if (!validAccess) {
      throw new Error(`Invalid static resource access in ${mdxFilePaths[index]}`)
    }
  });

  if (validAccesses.every(Boolean)) { console.log("\t ALL STATIC RESOURCE ACCESSES in md/mdx WERE VALID!") }
}
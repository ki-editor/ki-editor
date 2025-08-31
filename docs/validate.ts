const { fromMarkdown } = require('mdast-util-from-markdown');
const { mdxFromMarkdown } = require('mdast-util-mdx');
const { mdx } = require('micromark-extension-mdx');
const { visit } = require('unist-util-visit');

function extractArgumentFileNames(mdxContent) {
  const tree = fromMarkdown(mdxContent, {
      extensions: [mdx()],
      mdastExtensions: [mdxFromMarkdown()]
  });

  const argFileNames = [];
  visit(tree, (node) => {
    const nodeType = node.type;
    const isFlowElement = nodeType === 'mdxJsxFlowElement';
    const isJsxTextElement = nodeType === 'mdxJsxTextElement'
    const isNameTutorialFallback = node.name === 'TutorialFallback';
    if ((isFlowElement || isJsxTextElement) && (isNameTutorialFallback)) {
      const fileNameAttr = node.attributes?.find(
        attr => attr.type === 'mdxJsxAttribute' && attr.name === 'filename'
      );
      if (fileNameAttr && fileNameAttr.value) {
        argFileNames.push(fileNameAttr.value);
      }
    }
  });
  return argFileNames;
}

function validateResourceAccess(mdxContent: String, validFilenames: Array<String>) {
  const argFilenames = extractArgumentFileNames(mdxContent);
  const validResources = argFilenames.map(argFileName =>
    validFilenames.includes(argFileName)
  );
  
  let hasInvalidResource = false;
  validResources.forEach((isValid, index) => {
    if (!isValid) {
      console.log(`Non-existent static resource: "${argFilenames[index]}"`);
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
  let passedAll = mdxFilePaths
    .map(testFilePath => fs.readFileSync(testFilePath, 'utf8'))
    .map(mdxContent => validateResourceAccess(mdxContent, validResourceFilenames))
    .every(Boolean);

  if (!passedAll) { console.log("Failed Some Tests") }
  else { console.log("Passed All Tests") }
}
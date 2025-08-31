const glob = require('glob');
const path = require('path');

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
  // Traverse the ast
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

// Export for testing
module.exports = {
  extractArgumentFileNames,
  validateStaticResources
};

const fs = require('fs');

function validateStaticResources() {  
  const staticResourcesFilePaths = glob.sync('static/**/*.json');
  const staticResourcesFileNames = staticResourcesFilePaths.map(filePath =>
      path.basename(filePath, path.extname(filePath))
  );
  // console.log(staticResourcesFileNames);
  
  const mdxFilePaths = glob.sync('docs/**/*.{md,mdx}');
  // console.log(mdxFilePaths);
  
  mdxFilePaths.map(testFilePath => {
    const testFileContent = fs.readFileSync(testFilePath, 'utf8');
    // console.log(testFileContent);
    const testArgFileNamesOutput = extractArgumentFileNames(testFileContent);
    // console.log(testArgFileNamesOutput)
  
    testArgFileNamesOutput.map(argFileName => {
      let isValidStaticResourceName = staticResourcesFileNames.includes(argFileName);
      if (!isValidStaticResourceName) {
        throw new Error(`<TutorialFallback filename="${argFileName}" /> in file ${testFilePath}:\n\tStatic Resource named "${argFileName}" not found`);
      }
    });
  });
  
  console.log("All Static Resource access in <TutorialFallback filename=\"...\" /> were Valid");
}

if (require.main === module) {
  validateStaticResources();
}

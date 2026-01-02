import BrowserOnly from "@docusaurus/BrowserOnly";

export const AppConfigSchemaViewerFallback = () => {
    return (
        <BrowserOnly fallback={<div>Loading...</div>}>
            {() => {
                const LibComponent =
                    require("./AppConfigSchemaViewer").AppConfigSchemaViewer;
                return <LibComponent />;
            }}
        </BrowserOnly>
    );
};

import BrowserOnly from "@docusaurus/BrowserOnly";

export const AppConfigSchemaViewerFallback = () => {
    return (
        <BrowserOnly fallback={<div>Loading...</div>}>
            {() => {
                const LibComponent =
                    require("./SchemaViewer").AppConfigSchemaViewer;
                return <LibComponent />;
            }}
        </BrowserOnly>
    );
};
export const ScriptInputSchemaViewerFallback = () => {
    return (
        <BrowserOnly fallback={<div>Loading...</div>}>
            {() => {
                const LibComponent =
                    require("./SchemaViewer").ScriptInputSchemaViewer;
                return <LibComponent />;
            }}
        </BrowserOnly>
    );
};
export const ScriptOutputSchemaViewerFallback = () => {
    return (
        <BrowserOnly fallback={<div>Loading...</div>}>
            {() => {
                const LibComponent =
                    require("./SchemaViewer").ScriptOutputSchemaViewer;
                return <LibComponent />;
            }}
        </BrowserOnly>
    );
};

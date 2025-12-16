import JSONSchemaViewer from "@theme/JSONSchemaViewer";
import JSONSchemaEditor from "@theme/JSONSchemaEditor";

import Schema from "@site/static/app_config_json_schema.json";

export function AppConfigSchemaViewer() {
    return (
        <div style={{ display: "grid" }}>
            <JSONSchemaViewer
                schema={Schema}
                viewerOptions={{ showExamples: true, showExamples: true }}
            />

            <div style={{ display: "grid" }}>
                <JSONSchemaEditor schema={Schema} />
            </div>
        </div>
    );
}

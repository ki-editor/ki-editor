import JSONSchemaViewer from "@theme/JSONSchemaViewer";
import JSONSchemaEditor from "@theme/JSONSchemaEditor";

import Schema from "@site/static/app_config_json_schema.json";

import jsf from "json-schema-faker";
export function AppConfigSchemaViewer() {
    const sample = {
        theme: "VS Code (Light)",
        keyboard_layout: "Qwerty",
        languages: {},
    };
    return (
        <div style={{ display: "grid" }}>
            <JSONSchemaViewer
                schema={Schema}
                viewerOptions={{ showExamples: true }}
            />

            <div style={{ display: "grid" }}>
                <h2>Validator</h2>
                <JSONSchemaEditor
                    schema={Schema}
                    defaultValue={JSON.stringify(sample, null, 4)}
                />
            </div>
        </div>
    );
}

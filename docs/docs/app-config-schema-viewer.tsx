import JSONSchemaViewer from "@theme/JSONSchemaViewer";
import JSONSchemaEditor from "@theme/JSONSchemaEditor";
import Schema from "@site/static/app_config_json_schema.json";
import DefaultConfig from "@site/static/config_default.json";

export function AppConfigSchemaViewer() {
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
                    defaultValue={JSON.stringify(DefaultConfig, null, 4)}
                />
            </div>
        </div>
    );
}

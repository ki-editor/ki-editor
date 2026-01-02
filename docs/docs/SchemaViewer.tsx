// @ts-expect-error - docusaurus-json-schema-plugin types not resolving
import JSONSchemaViewer from "@theme/JSONSchemaViewer";
// @ts-expect-error - docusaurus-json-schema-plugin types not resolving
import JSONSchemaEditor from "@theme/JSONSchemaEditor";
import AppConfigSchema from "@site/static/app_config_json_schema.json";
import ScriptInputSchema from "@site/static/script_input_json_schema.json";
import ScriptOutputSchema from "@site/static/script_output_json_schema.json";
import DefaultConfig from "@site/static/config_default.json";

export function AppConfigSchemaViewer() {
    return (
        <div style={{ display: "grid" }}>
            <JSONSchemaViewer
                schema={AppConfigSchema}
                viewerOptions={{ showExamples: true }}
            />

            <div style={{ display: "grid" }}>
                <h2>Validator</h2>
                <JSONSchemaEditor
                    schema={AppConfigSchema}
                    defaultValue={JSON.stringify(DefaultConfig, null, 4)}
                />
            </div>
        </div>
    );
}

export function ScriptInputSchemaViewer() {
    return (
        <div style={{ display: "grid" }}>
            <JSONSchemaViewer
                schema={ScriptInputSchema}
                viewerOptions={{ showExamples: true }}
            />
        </div>
    );
}
export function ScriptOutputSchemaViewer() {
    return (
        <div style={{ display: "grid" }}>
            <JSONSchemaViewer
                schema={ScriptOutputSchema}
                viewerOptions={{ showExamples: true }}
            />
        </div>
    );
}

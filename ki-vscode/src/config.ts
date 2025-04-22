import * as vscode from "vscode";

export class KiConfig {
  private static readonly CONFIG_SECTION = "ki-vscode";

  private getConfig(): vscode.WorkspaceConfiguration {
    return vscode.workspace.getConfiguration(KiConfig.CONFIG_SECTION);
  }

  public getBackendPath(): string {
    return this.getConfig().get("backendPath", "");
  }

  public setBackendPath(path: string): Thenable<void> {
    return this.getConfig().update(
      "backendPath",
      path,
      vscode.ConfigurationTarget.Global
    );
  }
}

export const kiConfig = new KiConfig();

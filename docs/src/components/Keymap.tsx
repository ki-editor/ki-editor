import { useEffect, useState } from "react";
import { useColorMode } from "@docusaurus/theme-common";

import * as React from "react";
import useBaseUrl from "@docusaurus/useBaseUrl";
import * as z from "zod";

// Custom hook for localStorage
function useLocalStorage<T>(key: string, initialValue: T) {
  // Initialize state with value from localStorage or initial value
  const [storedValue, setStoredValue] = useState<T>(() => {
    try {
      const item = localStorage.getItem(key);
      return item ? JSON.parse(item) : initialValue;
    } catch (error) {
      console.error(`Error reading localStorage key "${key}":`, error);
      return initialValue;
    }
  });

  // Update localStorage when the state changes
  useEffect(() => {
    try {
      localStorage.setItem(key, JSON.stringify(storedValue));
    } catch (error) {
      console.error(`Error writing to localStorage key "${key}":`, error);
    }
  }, [key, storedValue]);

  return [storedValue, setStoredValue] as const;
}

const actionSchema = z.object({
  label: z.string(),
  docs: z.string().nullish(),
});

const keymapSchema = z.object({
  name: z.string(),
  rows: z.array(
    z.array(
      z.object({
        normal: z.nullable(actionSchema),
        alted: z.nullable(actionSchema),
        shifted: z.nullable(actionSchema),
      })
    )
  ),
  keyboard_layouts: z.array(
    z.object({
      name: z.string(),
      keys: z.array(z.array(z.string())),
    })
  ),
});

type Keymap = z.infer<typeof keymapSchema>;

const STORAGE_KEYS = {
  KEYBOARD_LAYOUT: "keymap-keyboard-layout",
  SHOW_KEYS: "keymap-show-keys",
  GRID_ALIGNMENT: "grid-alignment",
  PANEL_LAYOUT: "panel-layout",
} as const;

async function loadKeymap(url: string) {
  const response = await fetch(url);
  const json = await response.json();
  console.log(json);
  return keymapSchema.parse(json);
}

export const Keymap = (props: { filename: string }) => {
  const [keymap, setKeymap] = useState<Keymap | null>(null);
  const [error, setError] = useState<Error | null>(null);
  const url = useBaseUrl(`/keymaps/${props.filename}.json`);

  useEffect(() => {
    loadKeymap(url)
      .then((keymap) => {
        setKeymap(keymap);
      })
      .catch((error) => setError(error));
  }, [url]);

  return (
    <div style={{ display: "grid", gap: 64 }}>
      {keymap && <KeymapView keymap={keymap} />}
      {error && <div style={{ color: "red" }}>{error.message}</div>}
    </div>
  );
};

const gridAlignments = ["Row Staggered", "Ortholinear"] as const;
type GridAlignment = (typeof gridAlignments)[number];

const panelLayouts = ["Unified", "Split", "Stack"] as const;
type PanelLayout = (typeof panelLayouts)[number];

const KeymapView = (props: { keymap: Keymap }) => {
  // Using the custom hook for each stored value
  const [showKeys, setShowKeys] = useLocalStorage(STORAGE_KEYS.SHOW_KEYS, true);
  const [panelLayout, setPanelLayout] = useLocalStorage<PanelLayout>(
    STORAGE_KEYS.PANEL_LAYOUT,
    "Unified"
  );
  const [gridAlignment, setGridAlignment] = useLocalStorage<GridAlignment>(
    STORAGE_KEYS.GRID_ALIGNMENT,
    "Ortholinear"
  );

  // Special handling for keyboard layout since we need to find the layout object
  const [layoutName, setLayoutName] = useLocalStorage(
    STORAGE_KEYS.KEYBOARD_LAYOUT,
    props.keymap.keyboard_layouts[0].name
  );

  const keyboardLayout = React.useMemo(() => {
    return (
      props.keymap.keyboard_layouts.find(
        (layout) => layout.name === layoutName
      ) || props.keymap.keyboard_layouts[0]
    );
  }, [layoutName, props.keymap.keyboard_layouts]);

  const cellWidth = 100;
  const { colorMode } = useColorMode();

  const cellStyle: React.CSSProperties = {
    width: cellWidth,
    height: cellWidth,
    border: `1px solid ${colorMode === "light" ? "black" : "white"}`,
    display: "grid",
    placeItems: "center",
    borderRadius: 4,
    gridTemplateRows: `repeat(${showKeys ? 4 : 3}, 1fr)`,
    fontSize: 14,
  };

  const Inputs = () => (
    <div
      style={{
        display: "grid",
        gridAutoFlow: "column",
        gap: 8,
        justifyContent: "start",
        alignItems: "center",
        overflowX: "auto",
        whiteSpace: "nowrap",
        paddingBottom: 8,
      }}
    >
      <label>
        <input
          type="checkbox"
          checked={showKeys}
          onChange={() => setShowKeys(!showKeys)}
        />
        <span>Show keys</span>
      </label>

      {showKeys && (
        <select
          value={keyboardLayout.name}
          onChange={(e) => {
            setLayoutName(e.target.value);
          }}
          className="px-2 py-1 border rounded"
        >
          {props.keymap.keyboard_layouts
            .sort((a, b) => a.name.localeCompare(b.name))
            .map((layout) => (
              <option key={layout.name} value={layout.name}>
                {layout.name}
              </option>
            ))}
        </select>
      )}

      <select
        value={panelLayout}
        onChange={(e) => {
          setPanelLayout(e.target.value as PanelLayout);
        }}
        className="px-2 py-1 border rounded"
      >
        {Array(...panelLayouts)
          .sort()
          .map((panelLayout) => (
            <option key={panelLayout} value={panelLayout}>
              {panelLayout}
            </option>
          ))}
      </select>

      <select
        value={gridAlignment}
        onChange={(e) => {
          setGridAlignment(e.target.value as GridAlignment);
        }}
      >
        {gridAlignments.map((arrangement) => (
          <option key={arrangement} value={arrangement}>
            {arrangement}
          </option>
        ))}
      </select>
    </div>
  );

  function exhaustiveSwitchHelper(_: never): never {
    throw new Error("You are missing cases in your switch");
  }

  type PanelRenderOption = "All" | "Left" | "Right";

  const getKeyPanelFilterPredicate = (
    panel: PanelRenderOption
  ): ((_: unknown, keyIndex: number) => boolean) => {
    switch (panel) {
      case "Left":
        return (_: unknown, keyIndex: number) => keyIndex < 5;
      case "Right":
        return (_: unknown, keyIndex: number) => keyIndex >= 5;
      case "All":
        return (_: unknown, _keyIndex: number) => true;
      default:
        exhaustiveSwitchHelper(panel);
    }
  };

  const getIsHomeKeyPredicate = (
    panel: PanelRenderOption
  ): ((rowIndex: number, columnIndex: number) => boolean) => {
    switch (panel) {
      case "Left":
        return (rowIndex: number, columnIndex: number) =>
          rowIndex === 1 && columnIndex === 3;
      case "Right":
        return (rowIndex: number, columnIndex: number) =>
          rowIndex === 1 && columnIndex === 1;
      case "All":
        return (rowIndex: number, columnIndex: number) =>
          rowIndex === 1 && (columnIndex === 3 || columnIndex === 6);
      default:
        exhaustiveSwitchHelper(panel);
    }
  };

  // This now renders All the keyboard panels or just the Left or just the Right
  const renderKeymap = (panel: PanelRenderOption) => {
    const keyPredicate = getKeyPanelFilterPredicate(panel);
    const rows = props.keymap.rows.map((row) => row.filter(keyPredicate));
    const keyboardLayoutKeys = keyboardLayout.keys.map((row) =>
      row.filter(keyPredicate)
    );
    const isHomeKey = getIsHomeKeyPredicate(panel);
    return rows.map((row, rowIndex) => {
      const marginLeft =
        gridAlignment === "Row Staggered" ? [0, 24, 56][rowIndex] : 0;
      return (
        <div
          key={rowIndex}
          style={{
            display: "grid",
            gridAutoFlow: "column",
            gap: 4,
            justifyContent: "start",
            marginLeft: marginLeft,
          }}
        >
          {row.map((key, columnIndex) => (
            <React.Fragment key={`${rowIndex}-${columnIndex}`}>
              {panelLayout === "Split" && columnIndex === 5 && (
                <div style={{ width: cellWidth / 1.618 }} />
              )}
              <div style={{ textAlign: "center" }}>
                <div
                  style={{
                    ...cellStyle,
                    gridArea: "1 / 1",
                    overflow: "hidden",
                    backgroundColor: isHomeKey(rowIndex, columnIndex)
                      ? colorMode === "light"
                        ? "lightyellow"
                        : "darkblue"
                      : undefined,
                  }}
                >
                  {showKeys && (
                    <div
                      style={{
                        backgroundColor:
                          colorMode === "light" ? "black" : "white",
                        color: colorMode === "light" ? "white" : "black",
                        width: "100%",
                      }}
                    >
                      {keyboardLayoutKeys[rowIndex][columnIndex]}
                    </div>
                  )}
                  {key.alted ? <div>⌥ &nbsp;{key.alted?.label}</div> : <div />}
                  {key.shifted ? (
                    <div>⇧&nbsp;{key.shifted?.label}</div>
                  ) : (
                    <div />
                  )}
                  {key.normal ? <div>{key.normal?.label}</div> : <div />}
                </div>
              </div>
            </React.Fragment>
          ))}
        </div>
      );
    });
  };

  const Body = () => (
    <div
      style={{
        fontFamily: "sans-serif",
        whiteSpace: "nowrap",
        display: "grid",
        gap: 4,
        paddingBottom: 16,
        overflowX: "auto",
      }}
    >
      {panelLayout === "Stack" ? (
        <React.Fragment>
          <h3>Left Key Panel</h3>
          {renderKeymap("Left")}
          <h3 style={{ paddingTop: 16 }}>Right Key Panel</h3>
          {renderKeymap("Right")}
        </React.Fragment>
      ) : (
        renderKeymap("All")
      )}
    </div>
  );

  return (
    <div
      style={{
        display: "grid",
        gap: 8,
        marginTop: 8,
        marginBottom: 16,
      }}
    >
      <Inputs />
      <Body />
    </div>
  );
};

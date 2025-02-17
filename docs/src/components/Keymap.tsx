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

const keymapSchema = z.object({
  name: z.string(),
  rows: z.array(
    z.array(
      z.object({
        normal: z.nullable(z.string()),
        alted: z.nullable(z.string()),
        shifted: z.nullable(z.string()),
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
  SPLIT: "keymap-split",
  KEYS_ARRANGEMENT: "keymap-keys-arrangement",
} as const;

export const Keymap = (props: { filename: string }) => {
  const [keymap, setKeymap] = useState<Keymap | null>(null);
  const [error, setError] = useState<Error | null>(null);
  const url = useBaseUrl(`/keymaps/${props.filename}.json`);

  async function loadKeymap(url: string) {
    try {
      const response = await fetch(url);
      const json = await response.json();
      console.log(json);
      return keymapSchema.parse(json);
    } catch (error) {
      setError(error);
    }
  }

  useEffect(() => {
    loadKeymap(url).then((keymap) => {
      setKeymap(keymap);
    });
  }, []);

  return (
    <div style={{ display: "grid", gap: 64 }}>
      {keymap && <KeymapView keymap={keymap} />}
      {error && <div style={{ color: "red" }}>{error.message}</div>}
    </div>
  );
};

const keysArrangements = ["Row Staggered", "Ortholinear"] as const;
type KeysArrangement = (typeof keysArrangements)[number];

const KeymapView = (props: { keymap: Keymap }) => {
  // Using the custom hook for each stored value
  const [showKeys, setShowKeys] = useLocalStorage(STORAGE_KEYS.SHOW_KEYS, true);
  const [split, setSplit] = useLocalStorage(STORAGE_KEYS.SPLIT, true);
  const [keysArrangement, setKeysArrangement] =
    useLocalStorage<KeysArrangement>(
      STORAGE_KEYS.KEYS_ARRANGEMENT,
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

      <label>
        <input
          type="checkbox"
          checked={split}
          onChange={() => setSplit(!split)}
        />
        <span>Split</span>
      </label>

      <select
        value={keysArrangement}
        onChange={(e) => {
          setKeysArrangement(e.target.value as KeysArrangement);
        }}
      >
        {keysArrangements.map((arrangement) => (
          <option key={arrangement} value={arrangement}>
            {arrangement}
          </option>
        ))}
      </select>
    </div>
  );

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
      {props.keymap.rows.map((row, rowIndex) => {
        return (
          <div
            key={rowIndex}
            style={{
              display: "grid",
              gridAutoFlow: "column",
              gap: 4,
              justifyContent: "start",
              marginLeft:
                keysArrangement === "Row Staggered" ? [0, 24, 56][rowIndex] : 0,
            }}
          >
            {row.map((key, columnIndex) => (
              <React.Fragment key={`${rowIndex}-${columnIndex}`}>
                {split && columnIndex === 5 && (
                  <div style={{ width: cellWidth / 1.618 }} />
                )}
                <div style={{ textAlign: "center" }}>
                  <div
                    style={{
                      ...cellStyle,
                      gridArea: "1 / 1",
                      overflow: "hidden",
                      backgroundColor:
                        rowIndex === 1 && (columnIndex == 3 || columnIndex == 6)
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
                        {keyboardLayout.keys[rowIndex][columnIndex]}
                      </div>
                    )}
                    {key.alted ? <div>⌥ {key.alted}</div> : <div />}
                    {key.shifted ? <div>⇧ {key.shifted}</div> : <div />}
                    {key.normal ? <div>{key.normal}</div> : <div />}
                  </div>
                </div>
              </React.Fragment>
            ))}
          </div>
        );
      })}
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

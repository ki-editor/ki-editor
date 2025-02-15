import { useEffect, useState } from "react";
import { Select } from "@/components/ui/select";

import * as React from "react";
import useBaseUrl from "@docusaurus/useBaseUrl";

import * as z from "zod";

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
  const [keyboardLayout, setKeyboardLayout] = useState(
    props.keymap.keyboard_layouts[0]
  );
  const [showKeys, setShowKeys] = useState(true);
  const [split, setSplit] = useState(false);
  const [keysArrangement, setKeysArragement] =
    useState<KeysArrangement>("Row Staggered");

  const cellWidth = 80;
  const cellStyle: React.CSSProperties = {
    width: cellWidth,
    height: 80,
    border: "1px solid black",
    display: "grid",
    placeItems: "center",
    borderRadius: 4,
  };

  return (
    <div style={{ display: "grid", gap: 8, overflowX: "auto" }}>
      <div
        style={{
          display: "grid",
          gridAutoFlow: "column",
          gap: 8,
          justifyContent: "start",
          alignItems: "center",
        }}
      >
        <label>
          <input
            type="checkbox"
            checked={showKeys}
            onChange={() => {
              setShowKeys(!showKeys);
            }}
          />
          <span>Show keys</span>
        </label>
        {showKeys && (
          <select
            value={keyboardLayout.name}
            onChange={(e) => {
              const newLayout = props.keymap.keyboard_layouts.find(
                (layout) => layout.name === e.target.value
              );
              if (newLayout) {
                setKeyboardLayout(newLayout);
              }
            }}
            className="px-2 py-1 border rounded"
          >
            {props.keymap.keyboard_layouts.sort().map((layout) => (
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
            onChange={() => {
              setSplit(!split);
            }}
          />
          <span>Split</span>
        </label>

        <select
          value={keysArrangement}
          onChange={(e) => {
            setKeysArragement(e.target.value as KeysArrangement);
          }}
        >
          {keysArrangements.map((keysArrangement) => (
            <option key={keysArrangement} value={keysArrangement}>
              {keysArrangement}
            </option>
          ))}
        </select>
      </div>
      <div style={{ fontFamily: "monospace", display: "grid", gap: 4 }}>
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
                  keysArrangement === "Row Staggered"
                    ? [0, 24, 56][rowIndex]
                    : 0,
              }}
            >
              {row.map((key, keyIndex) => (
                <React.Fragment key={`${rowIndex}-${keyIndex}`}>
                  {split && keyIndex === 5 && (
                    <div style={{ width: cellWidth }} />
                  )}
                  <div>
                    <div style={{ ...cellStyle, gridArea: "1 / 1" }}>
                      {showKeys && (
                        <div style={{ textAlign: "start" }}>
                          {keyboardLayout.keys[rowIndex][keyIndex]}
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
    </div>
  );
};

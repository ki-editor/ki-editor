import React from "react";
import BrowserOnly from "@docusaurus/BrowserOnly";

export const KeymapFallback = (props: { filename: string }) => {
  return (
    <BrowserOnly fallback={<div>Loading...</div>}>
      {() => {
        const LibComponent = require("./Keymap").Keymap;
        return <LibComponent filename={props.filename} />;
      }}
    </BrowserOnly>
  );
};

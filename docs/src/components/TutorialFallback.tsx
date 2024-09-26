import React from "react";
import BrowserOnly from "@docusaurus/BrowserOnly";

export const TutorialFallback = () => {
  return (
    <BrowserOnly fallback={<div>Loading...</div>}>
      {() => {
        const LibComponent = require("./Tutorial").Tutorial;
        return <LibComponent />;
      }}
    </BrowserOnly>
  );
};

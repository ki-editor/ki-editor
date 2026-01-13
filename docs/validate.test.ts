import { makeExtractArgumentFileNames } from "./validate";

import { describe, expect, it } from "vitest";

const extractArgumentFileNames =
    makeExtractArgumentFileNames("TutorialFallback");

describe("Validating correctness of `extractArgumentFileNames`", () => {
    it("Empty mdx should return empty array", () => {
        const mdx = "";
        expect(extractArgumentFileNames(mdx)).toEqual([]);
    });

    it("Single tag should extract the argument filename", () => {
        const mdx = '<TutorialFallback filename="example-config" />';
        expect(extractArgumentFileNames(mdx)).toEqual(["example-config"]);
    });

    it("Multiple tags should extract multiple filenames", () => {
        const mdx = `
<TutorialFallback filename="resource1" />
<TutorialFallback filename="resource2" />`;
        expect(extractArgumentFileNames(mdx)).toEqual([
            "resource1",
            "resource2",
        ]);
    });

    it("Commented tags should be ignored", () => {
        const mdx = `
<TutorialFallback filename="resource1" />
<TutorialFallback filename="resource2" />
{/* <TutorialFallback filename="resource3" /> */}`;
        expect(extractArgumentFileNames(mdx)).toEqual([
            "resource1",
            "resource2",
        ]);
    });

    it("Mdx with Frontmatter should work", () => {
        const mdx = `
---
title: Hello World
---
<TutorialFallback filename="resource" />`;
        expect(extractArgumentFileNames(mdx)).toEqual(["resource"]);
    });

    it("Tags other than `TutorialFallback` should be ignored", () => {
        const mdx = `
<SomeOtherComponent filename="ignore-me" />
<TutorialFallback filename="keep-me" />`;
        expect(extractArgumentFileNames(mdx)).toEqual(["keep-me"]);
    });

    it("Inlining of single tags should work", () => {
        const mdx = `
Here is an inline use of <TutorialFallback filename="resource" /> resource`;
        expect(extractArgumentFileNames(mdx)).toEqual(["resource"]);
    });

    it("Inlining of multiple tags should work", () => {
        const mdx = `
Multiple <TutorialFallback filename="resource1" /> tags <TutorialFallback filename="resource2" />`;
        expect(extractArgumentFileNames(mdx)).toEqual([
            "resource1",
            "resource2",
        ]);
    });

    it("Mixing inlining tags and regular tags should work", () => {
        const mdx = `
Multiple <TutorialFallback filename="resource1" /> <TutorialFallback filename="resource2" />`;
        expect(extractArgumentFileNames(mdx)).toEqual([
            "resource1",
            "resource2",
        ]);
    });
});

import { describe, expect, it } from "vitest";
import { parseQwertyKey } from "./parseQwertyKey";

describe("parseQwertyKey", () => {
    it("parses a key with a keyboard modifier (e.g. shift+j)", () => {
        expect(parseQwertyKey("shift+j")).toEqual({
            keyboardModifier: "shift",
            releaseModifier: undefined,
            rowIndex: 1,
            columnIndex: 6,
        });
    });

    it("parses capital key", () => {
        expect(parseQwertyKey("shift+J")).toEqual({
            keyboardModifier: "shift",
            releaseModifier: undefined,
            rowIndex: 1,
            columnIndex: 6,
        });
    });

    it("parses a plain key", () => {
        expect(parseQwertyKey("q")).toEqual({
            keyboardModifier: undefined,
            releaseModifier: undefined,
            rowIndex: 0,
            columnIndex: 0,
        });
    });

    it("parses a key with a keyboard modifier (e.g. ctrl+a)", () => {
        expect(parseQwertyKey("ctrl+a")).toEqual({
            keyboardModifier: "ctrl",
            releaseModifier: undefined,
            rowIndex: 1,
            columnIndex: 0,
        });
    });

    it("parses a key with a release modifier (e.g. release-a)", () => {
        expect(parseQwertyKey("release-a")).toEqual({
            keyboardModifier: undefined,
            releaseModifier: "release",
            rowIndex: 1,
            columnIndex: 0,
        });
    });

    it("parses a key with a release modifier and a keyboard modifier (e.g. release-shift+a)", () => {
        expect(parseQwertyKey("release-shift+a")).toEqual({
            keyboardModifier: "shift",
            releaseModifier: "release",
            rowIndex: 1,
            columnIndex: 0,
        });
    });

    it("handles keys in the bottom row", () => {
        expect(parseQwertyKey("/")).toEqual({
            keyboardModifier: undefined,
            releaseModifier: undefined,
            rowIndex: 2,
            columnIndex: 9,
        });
    });
});

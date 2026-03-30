import { z } from "zod";

export const keyboardLayoutSchema = z.object({
    name: z.string(),
    keys: z.array(z.array(z.string())),
});

export type KeyboardLayout = z.infer<typeof keyboardLayoutSchema>;

const QWERTY_KEYS = [
    ["q", "w", "e", "r", "t", "y", "u", "i", "o", "p"],
    ["a", "s", "d", "f", "g", "h", "j", "k", "l", ";"],
    ["z", "x", "c", "v", "b", "n", "m", ",", ".", "/"],
];

export const translateQwertyToTargetLayout = (
    qwertyKey: string,
    targetLayout: KeyboardLayout,
) => {
    // We need to extract the key from the potentially added modifiers
    const { keyboardModifier, releaseModifier, rowIndex, columnIndex } =
        parseQwertyKey(qwertyKey);

    const result = targetLayout.keys[rowIndex]?.[columnIndex ?? 0];
    if (keyboardModifier && releaseModifier) {
        return `${releaseModifier}-${keyboardModifier}+${result}`;
    }
    if (keyboardModifier) {
        return `${keyboardModifier}+${result}`;
    }
    if (releaseModifier) {
        return `${releaseModifier}-${result}`;
    }
    return result;
};

export const parseQwertyKey = (raw: string) => {
    const [rest, releaseModifier] = raw.split("-").reverse();
    const [key, keyboardModifier] = (rest ?? "").split("+").reverse();
    const rowIndex = QWERTY_KEYS.findIndex((row) =>
        row.includes(key?.toLowerCase() ?? ""),
    );
    const columnIndex = QWERTY_KEYS[rowIndex]?.indexOf(
        key?.toLowerCase() ?? "",
    );
    return { keyboardModifier, releaseModifier, rowIndex, columnIndex };
};

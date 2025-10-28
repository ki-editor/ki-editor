module.exports = grammar({
    name: "quickfix",

    extras: (_$) => [/ /, "\n"], // Ignore whitespace

    rules: {
        // The entry point of the grammar
        source_file: ($) => repeat1($.section),

        // A section is a header followed by zero or more values
        section: ($) => seq($.header, "\n", $.values),

        header: ($) => $.word,

        values: ($) => repeat1($.value),

        // A value is a word followed by a newline
        value: ($) => seq("    ", $.word, optional("\n")),

        // A word is a sequence of non-whitespace characters
        word: (_$) => /[^\n]+/,
    },
});

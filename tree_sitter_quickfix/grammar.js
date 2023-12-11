const makeQuote = (quote) => ($) =>
  choice(
    prec.left(seq(quote, quote)),
    prec.left(seq(quote, $.expressions, quote))
  );

const makeEnclose = (open, close) => ($) =>
  choice(seq(open, close), seq(open, $.expressions, close));

module.exports = grammar({
  name: "yard",

  extras: ($) => [/ /, "\n"], // Ignore whitespace

  rules: {
    // The entry point of the grammar
    source_file: ($) => repeat($.section),

    // A section is a header followed by zero or more values
    section: ($) => seq($.header, $.values),

    // A header is a word enclosed in square brackets
    header: ($) => seq("■┬", $.word),

    values: ($) => seq(repeat($.value), $.lastValue),

    // A value is a word followed by a newline
    value: ($) => seq("├", $.word, "\n"),

    lastValue: ($) => seq("└", $.word),

    // A word is a sequence of non-whitespace characters
    word: ($) => /[^\n]+/,
  },
});
